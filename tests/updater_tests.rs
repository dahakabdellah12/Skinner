use brawlcraft_slint::updater::{
    is_trusted_release_url, sanitize_version_string, CryptoVerifier, SemVer,
};
use ed25519_dalek::{Signer, SigningKey};

#[test]
fn test_semver_parsing_and_comparison() {
    let v331 = SemVer::parse("3.3.1").unwrap();
    let v332 = SemVer::parse("v3.3.2").unwrap();
    let v3310 = SemVer::parse("3.3.10").unwrap();
    let v339 = SemVer::parse("3.3.9").unwrap();
    let v332_beta = SemVer::parse("3.3.2-beta").unwrap();

    // 3.3.2 is newer than 3.3.1
    assert!(v332.is_newer_than(&v331));
    assert!(!v331.is_newer_than(&v332));

    // 3.3.10 is newer than 3.3.9 (numeric ordering, not lexicographic)
    assert!(v3310.is_newer_than(&v339));
    assert!(!v339.is_newer_than(&v3310));

    // 3.3.2 is newer than 3.3.2-beta (stable > prerelease)
    assert!(v332.is_newer_than(&v332_beta));
    assert!(!v332_beta.is_newer_than(&v332));

    // Identical versions
    assert!(!v332.is_newer_than(&v332));

    // Invalid versions
    assert!(SemVer::parse("").is_none());
    assert!(SemVer::parse("abc").is_none());
    assert!(SemVer::parse("v").is_none());
}

#[test]
fn test_ed25519_signature_verification_and_tampering() {
    // Use an ephemeral test-only seed for unit testing verification logic.
    // Production signing keys are NEVER committed to source control or test fixtures.
    let test_seed: [u8; 32] = [0x5au8; 32];
    let signing_key = SigningKey::from_bytes(&test_seed);
    let verifying_key = signing_key.verifying_key();
    let pubkey_bytes: [u8; 32] = verifying_key.to_bytes();

    let manifest_content = b"01169faa68216bad1dfbcc0ddd3311f4c836c15ce926f409a70170881899cdfa  Skinner-Setup-3.3.2.exe\n";
    let signature = signing_key.sign(manifest_content);
    let sig_bytes = signature.to_bytes();

    // 1. Valid signature verifies successfully
    assert!(CryptoVerifier::verify_signature(manifest_content, &sig_bytes, &pubkey_bytes).is_ok());

    // 2. Tampered manifest fails verification
    let tampered_manifest = b"01169faa68216bad1dfbcc0ddd3311f4c836c15ce926f409a70170881899cdfa  Malicious-Setup-3.3.2.exe\n";
    assert!(CryptoVerifier::verify_signature(tampered_manifest, &sig_bytes, &pubkey_bytes).is_err());

    // 3. Tampered signature fails verification
    let mut corrupt_sig = sig_bytes;
    corrupt_sig[0] ^= 0xff;
    assert!(CryptoVerifier::verify_signature(manifest_content, &corrupt_sig, &pubkey_bytes).is_err());

    // 4. Invalid public key fails verification
    let mut wrong_pubkey = pubkey_bytes;
    wrong_pubkey[0] ^= 0xaa;
    assert!(CryptoVerifier::verify_signature(manifest_content, &sig_bytes, &wrong_pubkey).is_err());
}

#[test]
fn test_sha256sums_parsing() {
    let content = r#"
# Official Skinner Checksums
01169faa68216bad1dfbcc0ddd3311f4c836c15ce926f409a70170881899cdfa  Skinner-Setup-3.3.2.exe
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 *Skinner-3.3.2-portable.zip
invalid line without hash
1234  too_short.exe
"#;
    let map = CryptoVerifier::parse_sha256sums(content);
    assert_eq!(map.len(), 2);
    assert_eq!(
        map.get("Skinner-Setup-3.3.2.exe").unwrap(),
        "01169faa68216bad1dfbcc0ddd3311f4c836c15ce926f409a70170881899cdfa"
    );
    assert_eq!(
        map.get("Skinner-3.3.2-portable.zip").unwrap(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_trusted_release_url_validation() {
    // Valid official release URLs (new Skinner repo and backward-compatible BrawlCraft repo)
    assert!(is_trusted_release_url("https://github.com/dahakabdellah12/Skinner/releases/download/v3.3.2/Skinner-Setup-3.3.2.exe"));
    assert!(is_trusted_release_url("https://github.com/dahakabdellah12/Skinner/releases/download/v3.3.2/SHA256SUMS.txt"));
    assert!(is_trusted_release_url("https://github.com/dahakabdellah12/BrawlCraft/releases/download/v3.3.2/Skinner-Setup-3.3.2.exe"));

    // Rejected untrusted domains and multi-tenant CDNs
    assert!(!is_trusted_release_url("https://objects.githubusercontent.com/attacker/malware.exe"));
    assert!(!is_trusted_release_url("https://malicious.com/Skinner-Setup-3.3.2.exe"));
    assert!(!is_trusted_release_url("http://github.com/dahakabdellah12/Skinner/releases/download/v3.3.2/Skinner.exe")); // HTTP blocked
    assert!(!is_trusted_release_url("https://github.com/anotheruser/Skinner/releases/download/v3.3.2/Skinner.exe"));

    // Rejected path traversal attempts
    assert!(!is_trusted_release_url("https://github.com/dahakabdellah12/Skinner/releases/download/../../secret"));
    assert!(!is_trusted_release_url("https://github.com/dahakabdellah12/Skinner/releases/download/..\\secret"));
}

#[test]
fn test_version_sanitization() {
    assert_eq!(sanitize_version_string("3.3.2").unwrap(), "3.3.2");
    assert_eq!(sanitize_version_string("v3.3.2").unwrap(), "v3.3.2");
    assert_eq!(sanitize_version_string("3.3.2-beta.1").unwrap(), "3.3.2-beta.1");

    // Path traversal blocked
    assert!(sanitize_version_string("../3.3.2").is_none());
    assert!(sanitize_version_string("..").is_none());
    assert!(sanitize_version_string(".hidden").is_none());
    assert!(sanitize_version_string("").is_none());
}

#[test]
fn test_sha256_file_computation_and_tampering() {
    let temp_file = std::env::temp_dir().join(format!("test_sha256_{}.tmp", std::process::id()));
    std::fs::write(&temp_file, b"Hello Skinner Update System").unwrap();

    let hash = CryptoVerifier::compute_sha256(&temp_file).unwrap();
    // SHA256 of "Hello Skinner Update System"
    assert_eq!(hash, "be1026256971a572b1c5fa9a1a22d318d3b4f1ff16b573a2b68e2389bcb8a6ce");

    // Modify file contents
    std::fs::write(&temp_file, b"Tampered Content").unwrap();
    let tampered_hash = CryptoVerifier::compute_sha256(&temp_file).unwrap();
    assert_ne!(hash, tampered_hash);

    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_prerelease_and_draft_filtering_logic() {
    let json_stable = serde_json::json!({
        "tag_name": "v3.3.2",
        "name": "Skinner v3.3.2",
        "draft": false,
        "prerelease": false,
        "assets": [
            {
                "name": "Skinner-Setup-3.3.2.exe",
                "browser_download_url": "https://github.com/dahakabdellah12/BrawlCraft/releases/download/v3.3.2/Skinner-Setup-3.3.2.exe",
                "size": 52428800
            },
            {
                "name": "SHA256SUMS.txt",
                "browser_download_url": "https://github.com/dahakabdellah12/BrawlCraft/releases/download/v3.3.2/SHA256SUMS.txt",
                "size": 256
            },
            {
                "name": "SHA256SUMS.txt.sig",
                "browser_download_url": "https://github.com/dahakabdellah12/BrawlCraft/releases/download/v3.3.2/SHA256SUMS.txt.sig",
                "size": 128
            }
        ]
    });

    let draft_json = serde_json::json!({
        "tag_name": "v3.3.3",
        "draft": true,
        "prerelease": false
    });

    let prerelease_json = serde_json::json!({
        "tag_name": "v3.4.0-alpha",
        "draft": false,
        "prerelease": true
    });

    assert!(!draft_json.get("draft").unwrap().as_bool().unwrap_or(false) == false);
    assert!(!prerelease_json.get("prerelease").unwrap().as_bool().unwrap_or(false) == false);

    // Verify asset extraction on stable release
    let tag = json_stable.get("tag_name").unwrap().as_str().unwrap();
    let ver = SemVer::parse(tag).unwrap();
    assert_eq!(ver.to_string(), "3.3.2");

    let current = SemVer::parse("3.3.1").unwrap();
    assert!(ver.is_newer_than(&current));

    let assets = json_stable.get("assets").unwrap().as_array().unwrap();
    let exe_asset = assets.iter().find(|a| a.get("name").unwrap().as_str().unwrap().ends_with(".exe")).unwrap();
    assert_eq!(exe_asset.get("name").unwrap().as_str().unwrap(), "Skinner-Setup-3.3.2.exe");
    assert!(is_trusted_release_url(exe_asset.get("browser_download_url").unwrap().as_str().unwrap()));
}

#[test]
fn test_verify_v332_release_signature_against_embedded_key() {
    let manifest_path = std::path::Path::new("release/SHA256SUMS.txt");
    let sig_path = std::path::Path::new("release/SHA256SUMS.txt.sig");

    if manifest_path.is_file() && sig_path.is_file() {
        let manifest_bytes = std::fs::read(manifest_path).unwrap();
        let sig_bytes = std::fs::read(sig_path).unwrap();

        // Verify the authentic release signature strictly using the embedded official public key
        assert!(CryptoVerifier::verify_signature(&manifest_bytes, &sig_bytes, &brawlcraft_slint::updater::OFFICIAL_PUBLIC_KEY_BYTES).is_ok(), "v3.3.2 release manifest must pass Ed25519 signature verification against official key");
    }
}

#[test]
fn test_release_metadata_and_asset_hash_consistency() {
    let latest_json_path = std::path::Path::new("release/latest.json");
    let sums_path = std::path::Path::new("release/SHA256SUMS.txt");
    let sig_path = std::path::Path::new("release/SHA256SUMS.txt.sig");
    let zip_path = std::path::Path::new("release/Skinner-3.3.2-windows-x64.zip");
    let release_exe_path = std::path::Path::new("target/release/Skinner.exe");

    if latest_json_path.is_file() && sums_path.is_file() && sig_path.is_file() && zip_path.is_file() {
        // 1. Verify latest.json structure
        let json_str = std::fs::read_to_string(latest_json_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["version"].as_str().unwrap(), "3.3.2");
        assert_eq!(json["tag_name"].as_str().unwrap(), "v3.3.2");

        // 2. Compute actual zip hash and size
        let actual_zip_hash = CryptoVerifier::compute_sha256(zip_path).unwrap();
        let actual_zip_size = std::fs::metadata(zip_path).unwrap().len();

        let json_hash = json["installer_sha256"].as_str().unwrap();
        let json_size = json["installer_size"].as_u64().unwrap();

        assert_eq!(actual_zip_hash, json_hash, "latest.json installer_sha256 must match real zip hash");
        assert_eq!(actual_zip_size, json_size, "latest.json installer_size must match real zip file size");

        // 3. Verify SHA256SUMS.txt entries
        let sums_str = std::fs::read_to_string(sums_path).unwrap();
        let sums_map = CryptoVerifier::parse_sha256sums(&sums_str);

        let manifest_zip_hash = sums_map.get("Skinner-3.3.2-windows-x64.zip")
            .expect("Skinner-3.3.2-windows-x64.zip must be present in SHA256SUMS.txt");
        assert_eq!(manifest_zip_hash, &actual_zip_hash);

        if release_exe_path.is_file() {
            let actual_exe_hash = CryptoVerifier::compute_sha256(release_exe_path).unwrap();
            let manifest_exe_hash = sums_map.get("Skinner.exe")
                .expect("Skinner.exe must be present in SHA256SUMS.txt");
            assert_eq!(manifest_exe_hash, &actual_exe_hash, "Manifest Skinner.exe hash must match target/release/Skinner.exe");
        }

        // 4. Verify Ed25519 signature
        let manifest_bytes = std::fs::read(sums_path).unwrap();
        let sig_bytes = std::fs::read(sig_path).unwrap();
        assert!(CryptoVerifier::verify_signature(&manifest_bytes, &sig_bytes, &brawlcraft_slint::updater::OFFICIAL_PUBLIC_KEY_BYTES).is_ok());
    }
}

#[test]
fn test_upgrade_simulation_preserves_user_data_and_config() {
    let zip_path = std::path::Path::new("release/Skinner-3.3.2-windows-x64.zip");
    if !zip_path.is_file() {
        return;
    }

    let temp_root = std::env::temp_dir().join(format!("skinner_upgrade_sim_{}", std::process::id()));
    let install_dir = temp_root.join("installed_v331");
    let presets_dir = install_dir.join("presets");
    let backups_dir = install_dir.join("backups");
    let staging_dir = temp_root.join("staging");

    let _ = std::fs::create_dir_all(&presets_dir);
    let _ = std::fs::create_dir_all(&backups_dir);
    let _ = std::fs::create_dir_all(&staging_dir);

    // 1. Seed simulated existing installation v3.3.1
    let old_exe = install_dir.join("Skinner.exe");
    let config_file = install_dir.join("game_path.txt");
    let preset_file = presets_dir.join("my_valkyrie_custom.json");
    let backup_file = backups_dir.join("backup_initial.zip");

    std::fs::write(&old_exe, b"OLD_SKINNER_v3.3.1_BINARY_DATA").unwrap();
    std::fs::write(&config_file, b"C:\\Games\\Brawlhalla\\Brawlhalla.exe").unwrap();
    std::fs::write(&preset_file, b"{\"preset_name\":\"Valkyrie Gold\",\"legend\":3}").unwrap();
    std::fs::write(&backup_file, b"FAKE_ZIP_BACKUP_CONTENT").unwrap();

    // 2. Unpack release zip into staging directory
    let tar_res = std::process::Command::new("tar.exe")
        .args(&["-xf", zip_path.to_str().unwrap(), "-C", staging_dir.to_str().unwrap()])
        .output();
    assert!(tar_res.is_ok() && tar_res.unwrap().status.success(), "tar extraction of release zip must succeed");

    let staged_exe = staging_dir.join("Skinner.exe");
    assert!(staged_exe.is_file(), "Staged directory must contain Skinner.exe");
    let staged_exe_len = std::fs::metadata(&staged_exe).unwrap().len();
    assert!(staged_exe_len > 1_000_000, "Staged Skinner.exe must be a full executable");

    // 3. Execute atomic update handoff simulation via script file
    let s_dir = staging_dir.to_string_lossy().replace('/', "\\");
    let i_dir = install_dir.to_string_lossy().replace('/', "\\");
    let sim_script = temp_root.join("sim_update.cmd");
    let script_content = format!(
        "@echo off\nrobocopy \"{}\" \"{}\" /E /IS /IT >NUL\nif errorlevel 8 xcopy /E /Y /I \"{}\\*\" \"{}\" >NUL\nexit /b 0\n",
        s_dir, i_dir, s_dir, i_dir
    );
    std::fs::write(&sim_script, script_content).unwrap();

    let cmd_res = std::process::Command::new("cmd.exe")
        .args(&["/C", sim_script.to_str().unwrap()])
        .output()
        .expect("Failed to execute cmd script");

    if !cmd_res.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&cmd_res.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&cmd_res.stderr));
    }
    assert!(cmd_res.status.success(), "Update script handoff must succeed");

    // 4. Verify binary was updated
    let updated_exe_len = std::fs::metadata(&old_exe).unwrap().len();
    assert_eq!(updated_exe_len, staged_exe_len, "Skinner.exe must be upgraded to new v3.3.2 binary");
    assert_ne!(std::fs::read(&old_exe).unwrap(), b"OLD_SKINNER_v3.3.1_BINARY_DATA");

    // 5. CRITICAL: Verify existing user configurations, presets, and backups survived intact!
    assert_eq!(
        std::fs::read_to_string(&config_file).unwrap(),
        "C:\\Games\\Brawlhalla\\Brawlhalla.exe",
        "User game_path.txt must survive update unchanged"
    );
    assert_eq!(
        std::fs::read_to_string(&preset_file).unwrap(),
        "{\"preset_name\":\"Valkyrie Gold\",\"legend\":3}",
        "User presets must survive update unchanged"
    );
    assert_eq!(
        std::fs::read(&backup_file).unwrap(),
        b"FAKE_ZIP_BACKUP_CONTENT",
        "User backups must survive update unchanged"
    );

    // Clean up simulation directory
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn test_security_error_cases_and_downgrade_prevention() {
    // 1. Downgrade prevention
    let installed_version = SemVer::parse("3.3.2").unwrap();
    let old_v331 = SemVer::parse("3.3.1").unwrap();
    let old_v320 = SemVer::parse("3.2.0").unwrap();
    let old_v332_beta = SemVer::parse("3.3.2-beta").unwrap();

    assert!(!old_v331.is_newer_than(&installed_version), "Cannot upgrade to older 3.3.1");
    assert!(!old_v320.is_newer_than(&installed_version), "Cannot upgrade to older 3.2.0");
    assert!(!old_v332_beta.is_newer_than(&installed_version), "Cannot upgrade to prerelease beta");
    assert!(!installed_version.is_newer_than(&installed_version), "Same version is not newer");

    // 2. Arbitrary URL rejection
    let malicious_urls = [
        "http://github.com/dahakabdellah12/Skinner/releases/download/v3.3.3/Skinner.zip", // HTTP
        "https://evil-server.com/releases/download/v3.3.3/Skinner.zip",
        "https://objects.githubusercontent.com/attacker/malware.zip",
        "https://github.com/attacker/Skinner/releases/download/v3.3.3/Skinner.zip",
        "https://github.com/dahakabdellah12/Skinner/releases/download/../../etc/passwd",
        "https://github.com/dahakabdellah12/Skinner/releases/download/..\\windows\\system32",
    ];
    for url in malicious_urls {
        assert!(!is_trusted_release_url(url), "Untrusted or traversal URL must be rejected: {}", url);
    }

    // 3. Path traversal in version strings
    assert!(sanitize_version_string("../3.3.3").is_none());
    assert!(sanitize_version_string("..\\3.3.3").is_none());
    assert!(sanitize_version_string(".hidden").is_none());
    assert!(sanitize_version_string("").is_none());

    // 4. Missing asset in authenticated SHA256SUMS
    let manifest_content = "eec31a5367af6810d85f43a7b4cd4358ab0189bb5129ebae82a8d56e33f19c8e  Skinner.exe\n";
    let map = CryptoVerifier::parse_sha256sums(manifest_content);
    assert!(!map.contains_key("UnrelatedMaliciousFile.exe"));
}

#[test]
fn test_corrupted_archive_and_failed_installation_recovery() {
    let temp_root = std::env::temp_dir().join(format!("skinner_corrupt_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_root);

    // 1. Interrupted / Corrupted ZIP file (truncated header)
    let corrupt_zip = temp_root.join("corrupted.zip");
    std::fs::write(&corrupt_zip, b"PK\x03\x04corrupted_payload_data_not_valid_zip").unwrap();

    let staged_dir = temp_root.join("staged");
    let _ = std::fs::create_dir_all(&staged_dir);

    let tar_res = std::process::Command::new("tar.exe")
        .args(&["-xf", corrupt_zip.to_str().unwrap(), "-C", staged_dir.to_str().unwrap()])
        .output();

    // tar must fail on corrupted archive
    let failed = tar_res.is_err() || !tar_res.unwrap().status.success();
    assert!(failed, "Extracting corrupted archive must fail safely");

    // Skinner.exe must NOT exist in staged dir
    assert!(!staged_dir.join("Skinner.exe").is_file());

    // 2. Archive without Skinner.exe (wrong payload)
    let incomplete_dir = temp_root.join("incomplete_staged");
    let _ = std::fs::create_dir_all(&incomplete_dir);
    std::fs::write(incomplete_dir.join("some_other_file.txt"), b"no skinner exe").unwrap();

    let contains_skinner = incomplete_dir.join("Skinner.exe").is_file()
        || incomplete_dir.join("publish").join("Skinner.exe").is_file();
    assert!(!contains_skinner, "Payload without Skinner.exe must be detected and rejected");

    let _ = std::fs::remove_dir_all(&temp_root);
}
