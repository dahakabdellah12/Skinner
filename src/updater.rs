use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Official Skinner Ed25519 Public Verification Key (32 bytes)
/// All production release checksums and manifests MUST be signed with the matching private key.
pub const OFFICIAL_PUBLIC_KEY_BYTES: [u8; 32] = [
    0x10, 0x5b, 0x8c, 0xf2, 0x37, 0xa9, 0x41, 0xfd,
    0x06, 0x9f, 0x82, 0x81, 0xa7, 0xb3, 0xe4, 0xe5,
    0x36, 0x1b, 0x3e, 0xdd, 0xef, 0xf1, 0x0e, 0x86,
    0x26, 0xda, 0x22, 0x40, 0x54, 0xe9, 0x0f, 0x5a,
];

/// Official GitHub repository slug for update checks.
pub const OFFICIAL_REPO_SLUG: &str = "dahakabdellah12/Skinner";
pub const OFFICIAL_REPO_URL: &str = "https://github.com/dahakabdellah12/Skinner";
pub const OFFICIAL_DOWNLOAD_PREFIX: &str = "https://github.com/dahakabdellah12/Skinner/releases/download/";
pub const LEGACY_DOWNLOAD_PREFIX: &str = "https://github.com/dahakabdellah12/BrawlCraft/releases/download/";

/// Semantic Version structure supporting major.minor.patch[-prerelease]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
}

impl SemVer {
    pub fn parse(s: &str) -> Option<Self> {
        let clean = s.trim().trim_start_matches('v').trim_start_matches('V');
        if clean.is_empty() {
            return None;
        }

        let mut parts = clean.splitn(2, '-');
        let version_core = parts.next()?;
        let prerelease = parts.next().map(|p| p.trim().to_string());

        let nums: Vec<&str> = version_core.split('.').collect();
        if nums.is_empty() {
            return None;
        }

        let major = nums[0].parse::<u32>().ok()?;
        let minor = if nums.len() > 1 { nums[1].parse::<u32>().ok()? } else { 0 };
        let patch = if nums.len() > 2 { nums[2].parse::<u32>().ok()? } else { 0 };

        Some(SemVer {
            major,
            minor,
            patch,
            prerelease,
        })
    }

    pub fn is_newer_than(&self, other: &SemVer) -> bool {
        if self.major != other.major {
            return self.major > other.major;
        }
        if self.minor != other.minor {
            return self.minor > other.minor;
        }
        if self.patch != other.patch {
            return self.patch > other.patch;
        }

        // When major.minor.patch are equal:
        // A release with NO prerelease is newer than one WITH a prerelease (e.g. 3.3.2 > 3.3.2-beta)
        match (&self.prerelease, &other.prerelease) {
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (Some(a), Some(b)) => a > b,
            (None, None) => false,
        }
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref pre) = self.prerelease {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

/// Release asset metadata from GitHub
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_title: String,
    pub download_url: String,
    pub html_url: String,
    pub asset_name: String,
    pub asset_size_formatted: String,
    pub release_notes: String,
    pub manifest_url: Option<String>,
    pub signature_url: Option<String>,
    pub repo: String,
    pub repo_url: String,
    pub checked_at: String,
    pub network_error: bool,
}

/// Update progress structure sent to frontend UI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub status: String, // "starting", "downloading", "paused", "verifying", "installing", "cancelled", "error"
    pub progress: u32,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub error_message: Option<String>,
}

/// Cryptographic verification utilities
pub struct CryptoVerifier;

impl CryptoVerifier {
    /// Verify an Ed25519 signature over a message using the given 32-byte public key.
    pub fn verify_signature(message: &[u8], signature_bytes: &[u8], public_key_bytes: &[u8; 32]) -> Result<(), String> {
        let verifying_key = VerifyingKey::from_bytes(public_key_bytes)
            .map_err(|e| format!("Invalid public key bytes: {}", e))?;

        if signature_bytes.len() != 64 {
            return Err(format!("Invalid signature length: expected 64 bytes, got {}", signature_bytes.len()));
        }

        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(signature_bytes);
        let signature = Signature::from_bytes(&sig_arr);

        verifying_key
            .verify(message, &signature)
            .map_err(|e| format!("Ed25519 digital signature verification failed: {}", e))
    }

    /// Parses a standard SHA256SUMS file (format: `<hash>  <filename>` or `<hash> *<filename>`).
    pub fn parse_sha256sums(content: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for line in content.lines() {
            let line = line.trim_start_matches('\u{feff}').trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            if let (Some(hash), Some(filename)) = (parts.next(), parts.next()) {
                let clean_name = filename.trim_start_matches('*').trim();
                let clean_hash = hash.trim().to_ascii_lowercase();
                if clean_hash.len() == 64 && clean_hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    map.insert(clean_name.to_string(), clean_hash);
                }
            }
        }
        map
    }

    /// Computes the SHA-256 hex digest of a local file.
    pub fn compute_sha256(path: &Path) -> Result<String, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file for hashing: {}", e))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 32768];

        loop {
            let n = file.read(&mut buffer).map_err(|e| format!("Failed reading file for hashing: {}", e))?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Validates that a download URL strictly belongs to the official GitHub repository.
pub fn is_trusted_release_url(url: &str) -> bool {
    let trimmed = url.trim();
    if !trimmed.starts_with(OFFICIAL_DOWNLOAD_PREFIX) && !trimmed.starts_with(LEGACY_DOWNLOAD_PREFIX) {
        return false;
    }
    // Block any path traversal or invalid characters in URL
    if trimmed.contains("..") || trimmed.contains('\\') {
        return false;
    }
    true
}

/// Sanitizes a version string for safe usage in file system operations.
pub fn sanitize_version_string(ver: &str) -> Option<String> {
    let clean: String = ver.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-').collect();
    if clean.is_empty() || clean.starts_with('.') || clean.contains("..") {
        None
    } else {
        Some(clean)
    }
}

/// The dedicated Skinner update manager.
pub struct UpdateManager {
    is_paused: Arc<AtomicBool>,
    is_cancelled: Arc<AtomicBool>,
}

impl UpdateManager {
    pub fn new() -> Self {
        Self {
            is_paused: Arc::new(AtomicBool::new(false)),
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn get_update_dir() -> PathBuf {
        let dir = std::env::temp_dir().join("skinner_updates");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn clean_update_dir() {
        let dir = Self::get_update_dir();
        if dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    /// Checks the official GitHub repository for updates asynchronously.
    pub fn check_for_updates(current_ver_str: &str) -> ReleaseInfo {
        let current_semver = SemVer::parse(current_ver_str).unwrap_or(SemVer {
            major: 3,
            minor: 3,
            patch: 1,
            prerelease: None,
        });

        let mut res = ReleaseInfo {
            has_update: false,
            current_version: current_ver_str.to_string(),
            latest_version: current_ver_str.to_string(),
            release_title: format!("Skinner v{}", current_ver_str),
            download_url: format!("{}/releases/tag/v{}", OFFICIAL_REPO_URL, current_ver_str),
            html_url: format!("{}/releases/tag/v{}", OFFICIAL_REPO_URL, current_ver_str),
            asset_name: format!("Skinner-Setup-{}.exe", current_ver_str),
            asset_size_formatted: "50 MB".to_string(),
            release_notes: format!("Skinner v{} is currently running.", current_ver_str),
            manifest_url: None,
            signature_url: None,
            repo: OFFICIAL_REPO_SLUG.to_string(),
            repo_url: OFFICIAL_REPO_URL.to_string(),
            checked_at: chrono::Local::now().format("%H:%M:%S").to_string(),
            network_error: false,
        };

        let api_url = format!("https://api.github.com/repos/{}/releases/latest", OFFICIAL_REPO_SLUG);
        let output = std::process::Command::new("curl.exe")
            .args(&[
                "-s",
                "--max-time", "6",
                "-H", "User-Agent: Skinner-Desktop-Updater",
                "-H", "Accept: application/vnd.github.v3+json",
                &api_url,
            ])
            .output();

        let out = match output {
            Ok(o) if o.status.success() => o,
            _ => {
                res.network_error = true;
                return res;
            }
        };

        let json: serde_json::Value = match serde_json::from_slice(&out.stdout) {
            Ok(j) => j,
            Err(_) => {
                res.network_error = true;
                return res;
            }
        };

        // Filter out drafts and prereleases
        if json.get("draft").and_then(|v| v.as_bool()).unwrap_or(false) {
            return res;
        }
        if json.get("prerelease").and_then(|v| v.as_bool()).unwrap_or(false) {
            return res;
        }

        let tag_name = match json.get("tag_name").and_then(|v| v.as_str()) {
            Some(t) => t.trim().to_string(),
            None => return res,
        };

        let latest_semver = match SemVer::parse(&tag_name) {
            Some(v) => v,
            None => return res,
        };

        res.latest_version = latest_semver.to_string();
        if latest_semver.is_newer_than(&current_semver) {
            res.has_update = true;
        }

        if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
            res.release_title = name.to_string();
        }
        if let Some(html) = json.get("html_url").and_then(|v| v.as_str()) {
            res.html_url = html.to_string();
        }
        if let Some(body) = json.get("body").and_then(|v| v.as_str()) {
            res.release_notes = body.to_string();
        }

        let mut matched_exe = false;

        // Parse official assets
        if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
            for a in assets {
                let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let durl = a.get("browser_download_url").and_then(|v| v.as_str()).unwrap_or("");

                if name.ends_with(".exe") {
                    res.asset_name = name.to_string();
                    res.download_url = durl.to_string();
                    matched_exe = true;
                    if let Some(size) = a.get("size").and_then(|v| v.as_u64()) {
                        res.asset_size_formatted = format!("{:.1} MB", size as f64 / 1_048_576.0);
                    }
                } else if name.ends_with(".zip") && !matched_exe {
                    res.asset_name = name.to_string();
                    res.download_url = durl.to_string();
                    if let Some(size) = a.get("size").and_then(|v| v.as_u64()) {
                        res.asset_size_formatted = format!("{:.1} MB", size as f64 / 1_048_576.0);
                    }
                } else if name == "SHA256SUMS.txt" {
                    res.manifest_url = Some(durl.to_string());
                } else if name == "SHA256SUMS.txt.sig" {
                    res.signature_url = Some(durl.to_string());
                }
            }
        }

        res
    }

    /// Downloads and cryptographically verifies an update package.
    pub fn download_and_verify<F>(
        &self,
        info: &ReleaseInfo,
        public_key: &[u8; 32],
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(UpdateProgress),
    {
        if !is_trusted_release_url(&info.download_url) {
            return Err(format!("Download URL rejected: '{}'. Untrusted source.", info.download_url));
        }

        let clean_ver = sanitize_version_string(&info.latest_version)
            .ok_or_else(|| "Invalid release version format".to_string())?;

        let update_dir = Self::get_update_dir();
        Self::clean_update_dir();

        progress_callback(UpdateProgress {
            status: "starting".to_string(),
            progress: 2,
            bytes_downloaded: 0,
            total_bytes: 0,
            error_message: None,
        });

        // 1. Download SHA256SUMS.txt and SHA256SUMS.txt.sig
        let manifest_path = update_dir.join("SHA256SUMS.txt");
        let sig_path = update_dir.join("SHA256SUMS.txt.sig");

        let manifest_url = info.manifest_url.as_deref().unwrap_or("");
        let sig_url = info.signature_url.as_deref().unwrap_or("");

        // Download manifest
        if !manifest_url.is_empty() && is_trusted_release_url(manifest_url) {
            let out = std::process::Command::new("curl.exe")
                .args(&[
                    "--proto", "=https",
                    "--proto-redir", "=https",
                    "-L", "-s", "--max-time", "30",
                    "-o", manifest_path.to_str().unwrap(),
                    manifest_url
                ])
                .output()
                .map_err(|e| format!("Failed downloading manifest: {}", e))?;
            if !out.status.success() {
                return Err("Failed to retrieve SHA256SUMS.txt from official release.".to_string());
            }
        }

        // Download signature
        if !sig_url.is_empty() && is_trusted_release_url(sig_url) {
            let out = std::process::Command::new("curl.exe")
                .args(&[
                    "--proto", "=https",
                    "--proto-redir", "=https",
                    "-L", "-s", "--max-time", "30",
                    "-o", sig_path.to_str().unwrap(),
                    sig_url
                ])
                .output()
                .map_err(|e| format!("Failed downloading signature: {}", e))?;
            if !out.status.success() {
                return Err("Failed to retrieve signature file from official release.".to_string());
            }
        }

        // 2. Cryptographic signature check
        if manifest_path.is_file() && sig_path.is_file() {
            let manifest_bytes = fs::read(&manifest_path).map_err(|e| format!("Failed reading manifest: {}", e))?;
            let sig_bytes = fs::read(&sig_path).map_err(|e| format!("Failed reading signature: {}", e))?;

            // Decode signature: could be raw 64 bytes or 128 hex chars
            let decoded_sig = if sig_bytes.len() == 64 {
                sig_bytes
            } else if sig_bytes.len() >= 128 {
                let hex_str = String::from_utf8_lossy(&sig_bytes).trim().to_string();
                (0..hex_str.len())
                    .step_by(2)
                    .filter_map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).ok())
                    .collect::<Vec<u8>>()
            } else {
                return Err("Malformed Ed25519 signature in release asset".to_string());
            };

            CryptoVerifier::verify_signature(&manifest_bytes, &decoded_sig, public_key)?;
            crate::telemetry::log("INFO", "UPDATER", "Release manifest Ed25519 digital signature verified successfully", None);
        } else {
            return Err("Missing authenticated release manifest or signature. Update rejected.".to_string());
        }

        // 3. Download the actual executable installer or ZIP package
        let is_zip = info.asset_name.ends_with(".zip");
        let ext = if is_zip { "zip" } else { "exe" };
        let target_package = update_dir.join(format!("Skinner-Update-{}.{}", clean_ver, ext));

        progress_callback(UpdateProgress {
            status: "downloading".to_string(),
            progress: 10,
            bytes_downloaded: 0,
            total_bytes: 0,
            error_message: None,
        });

        let curl_out = std::process::Command::new("curl.exe")
            .args(&[
                "--proto", "=https",
                "--proto-redir", "=https",
                "-L",
                "-s",
                "--max-time", "300",
                "-o", target_package.to_str().unwrap(),
                &info.download_url,
            ])
            .output()
            .map_err(|e| format!("Download failed: {}", e))?;

        if !curl_out.status.success() || !target_package.exists() {
            let _ = fs::remove_file(&target_package);
            return Err("Failed downloading Skinner update package.".to_string());
        }

        progress_callback(UpdateProgress {
            status: "verifying".to_string(),
            progress: 95,
            bytes_downloaded: fs::metadata(&target_package).map(|m| m.len()).unwrap_or(0),
            total_bytes: fs::metadata(&target_package).map(|m| m.len()).unwrap_or(0),
            error_message: None,
        });

        // 4. Verify SHA-256 against verified manifest
        let manifest_str = fs::read_to_string(&manifest_path).unwrap_or_default();
        let sums = CryptoVerifier::parse_sha256sums(&manifest_str);

        let expected_hash = sums.get(&info.asset_name)
            .or_else(|| sums.get(&format!("Skinner-Setup-{}.exe", clean_ver)))
            .or_else(|| sums.get(&format!("Skinner-{}-setup-x64.exe", clean_ver)))
            .or_else(|| sums.get(&format!("Skinner-{}-windows-x64.zip", clean_ver)))
            .ok_or_else(|| format!("Package '{}' not found in authenticated SHA256SUMS manifest!", info.asset_name))?;

        let computed_hash = CryptoVerifier::compute_sha256(&target_package)?;
        if !computed_hash.eq_ignore_ascii_case(expected_hash) {
            let _ = fs::remove_file(&target_package);
            return Err(format!(
                "Cryptographic hash mismatch! Expected {}, got {}. The download was corrupted or modified.",
                expected_hash, computed_hash
            ));
        }
        crate::telemetry::log("INFO", "UPDATER", "Package SHA-256 integrity check passed bit-for-bit", None);

        progress_callback(UpdateProgress {
            status: "installing".to_string(),
            progress: 100,
            bytes_downloaded: fs::metadata(&target_package).map(|m| m.len()).unwrap_or(0),
            total_bytes: fs::metadata(&target_package).map(|m| m.len()).unwrap_or(0),
            error_message: None,
        });

        Ok(target_package)
    }

    /// Performs safe Windows handoff and installs the verified update.
    pub fn apply_update_and_restart(package_path: &Path) -> Result<(), String> {
        if !package_path.is_file() {
            return Err(format!("Update package file not found at {:?}", package_path));
        }

        let my_pid = std::process::id();
        let update_dir = Self::get_update_dir();
        let script_path = update_dir.join("apply_update.cmd");
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let install_dir = current_exe.parent().ok_or_else(|| "Failed to determine installation directory".to_string())?;

        let is_zip = package_path.extension().and_then(|e| e.to_str()).map(|s| s.eq_ignore_ascii_case("zip")).unwrap_or(false);

        let script_content = if is_zip {
            // Extract the zip to a staging directory first
            let staged_dir = update_dir.join("staged");
            let _ = fs::remove_dir_all(&staged_dir);
            let _ = fs::create_dir_all(&staged_dir);

            // Use tar -xf (built-in on Windows 10/11) or PowerShell Expand-Archive
            let tar_res = std::process::Command::new("tar.exe")
                .args(&["-xf", package_path.to_str().unwrap(), "-C", staged_dir.to_str().unwrap()])
                .output();

            let extract_ok = match tar_res {
                Ok(out) if out.status.success() => true,
                _ => {
                    // Fallback to powershell Expand-Archive
                    let ps_cmd = format!("Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force", package_path.display(), staged_dir.display());
                    let ps_res = std::process::Command::new("powershell.exe")
                        .args(&["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
                        .output();
                    matches!(ps_res, Ok(out) if out.status.success())
                }
            };

            if !extract_ok {
                return Err("Failed to extract update package archive".to_string());
            }

            // Verify staged directory contains Skinner.exe
            let staged_exe = if staged_dir.join("Skinner.exe").is_file() {
                staged_dir.join("Skinner.exe")
            } else if staged_dir.join("publish").join("Skinner.exe").is_file() {
                staged_dir.join("publish").join("Skinner.exe")
            } else {
                return Err("Extracted update does not contain Skinner.exe".to_string());
            };

            let source_dir = staged_exe.parent().unwrap();

            format!(
                r#"@echo off
setlocal
set PID={pid}
set SOURCE_DIR={source_dir}
set TARGET_DIR={target_dir}
set TARGET_EXE={target_exe}

:wait_pid
timeout /t 1 /nobreak >NUL
tasklist /FI "PID eq %PID%" 2>NUL | findstr /I "%PID%" >NUL
if not errorlevel 1 goto wait_pid

robocopy "%SOURCE_DIR%" "%TARGET_DIR%" /E /IS /IT >NUL
if errorlevel 8 xcopy /E /Y /I "%SOURCE_DIR%\*" "%TARGET_DIR%" >NUL
start "" "%TARGET_EXE%"
exit
"#,
                pid = my_pid,
                source_dir = source_dir.to_string_lossy().replace('/', "\\"),
                target_dir = install_dir.to_string_lossy().replace('/', "\\"),
                target_exe = current_exe.to_string_lossy().replace('/', "\\")
            )
        } else {
            // It is an executable installer
            format!(
                r#"@echo off
setlocal
set PID={pid}
set INSTALLER={installer}
set TARGET_DIR={target_dir}
set TARGET_EXE={target_exe}

:wait_pid
timeout /t 1 /nobreak >NUL
tasklist /FI "PID eq %PID%" 2>NUL | findstr /I "%PID%" >NUL
if not errorlevel 1 goto wait_pid

start /wait "" "%INSTALLER%" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /DIR="%TARGET_DIR%"
if exist "%TARGET_EXE%" (
    start "" "%TARGET_EXE%"
) else (
    start "" "%TARGET_DIR%\Skinner.exe"
)
exit
"#,
                pid = my_pid,
                installer = package_path.to_string_lossy().replace('/', "\\"),
                target_dir = install_dir.to_string_lossy().replace('/', "\\"),
                target_exe = current_exe.to_string_lossy().replace('/', "\\")
            )
        };

        fs::write(&script_path, script_content)
            .map_err(|e| format!("Failed writing updater script: {}", e))?;

        let mut cmd = std::process::Command::new("cmd.exe");
        cmd.args(&["/C", script_path.to_str().unwrap()]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        cmd.spawn().map_err(|e| format!("Failed spawning update handoff helper: {}", e))?;

        // Exit current Skinner process cleanly so installer/updater can access files
        std::process::exit(0);
    }
}
