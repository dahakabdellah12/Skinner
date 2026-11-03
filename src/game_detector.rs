use crate::models::GameInstallation;
use std::fs;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

pub struct GameDetector;

impl GameDetector {
    pub fn detect(custom_path: Option<&str>) -> GameInstallation {
        let mut installation = GameInstallation::default();
        let mut candidates = Vec::new();

        if let Some(cp) = custom_path {
            let trimmed = cp.trim().trim_matches('"').trim_matches('\'');
            let path = PathBuf::from(trimmed);
            if path.is_file()
                && path
                    .file_name()
                    .map_or(false, |f| f.to_string_lossy().eq_ignore_ascii_case("Brawlhalla.exe"))
            {
                if let Some(parent) = path.parent() {
                    candidates.push(parent.to_path_buf());
                }
            } else if path.is_dir() {
                candidates.push(path);
            }
        }

        // 0. Primary active user path & saved custom path from LocalAppData
        candidates.push(PathBuf::from("D:\\SteamLibrary\\steamapps\\common\\Brawlhalla"));

        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p0 = PathBuf::from(&local_app_data).join("Skinner").join("game_path.txt");
            if let Ok(content) = fs::read_to_string(p0) {
                let s = content.trim().trim_matches('"').trim_matches('\'');
                if !s.is_empty() {
                    candidates.push(PathBuf::from(s));
                }
            }
            let p1 = PathBuf::from(&local_app_data).join("BrawlCraft").join("game_path.txt");
            if let Ok(content) = fs::read_to_string(p1) {
                let s = content.trim().trim_matches('"').trim_matches('\'');
                if !s.is_empty() {
                    candidates.push(PathBuf::from(s));
                }
            }
            let p2 = PathBuf::from(&local_app_data)
                .join("BrawlhallaSkinStudio")
                .join("game_path.txt");
            if let Ok(content) = fs::read_to_string(p2) {
                let s = content.trim().trim_matches('"').trim_matches('\'');
                if !s.is_empty() {
                    candidates.push(PathBuf::from(s));
                }
            }
        }

        // 1. Steam registry checks
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(steam_key) = hkcu.open_subkey("Software\\Valve\\Steam") {
            if let Ok(p) = steam_key.get_value::<String, _>("SteamPath") {
                candidates.extend(get_steam_library_candidates(&p));
            }
        }

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(steam_key) = hklm.open_subkey("Software\\Valve\\Steam") {
            if let Ok(p) = steam_key.get_value::<String, _>("InstallPath") {
                candidates.extend(get_steam_library_candidates(&p));
            }
        }

        if let Ok(steam_key) = hklm.open_subkey("Software\\WOW6432Node\\Valve\\Steam") {
            if let Ok(p) = steam_key.get_value::<String, _>("InstallPath") {
                candidates.extend(get_steam_library_candidates(&p));
            }
        }

        // Direct Steam App 291550 Uninstall key
        if let Ok(app_key) = hklm.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App 291550") {
            if let Ok(p) = app_key.get_value::<String, _>("InstallLocation") {
                candidates.push(PathBuf::from(p));
            }
        }
        if let Ok(app_key) = hklm.open_subkey("Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App 291550") {
            if let Ok(p) = app_key.get_value::<String, _>("InstallLocation") {
                candidates.push(PathBuf::from(p));
            }
        }

        // 2. Common drive paths (C:, D:, E:, etc.) across all alphabet letters
        for drive in "CDEFGHIJKLMNOPQRSTUVWXYZ".chars() {
            candidates.push(PathBuf::from(format!("{}:\\SteamLibrary\\steamapps\\common\\Brawlhalla", drive)));
            candidates.push(PathBuf::from(format!("{}:\\Program Files (x86)\\Steam\\steamapps\\common\\Brawlhalla", drive)));
            candidates.push(PathBuf::from(format!("{}:\\Program Files\\Steam\\steamapps\\common\\Brawlhalla", drive)));
            candidates.push(PathBuf::from(format!("{}:\\Steam\\steamapps\\common\\Brawlhalla", drive)));
            candidates.push(PathBuf::from(format!("{}:\\Games\\Steam\\steamapps\\common\\Brawlhalla", drive)));
            candidates.push(PathBuf::from(format!("{}:\\Games\\Brawlhalla", drive)));
            candidates.push(PathBuf::from(format!("{}:\\Brawlhalla", drive)));
        }

        // Deduplicate and find first valid path
        let mut resolved: Option<PathBuf> = None;
        for candidate in candidates {
            if candidate.exists() && candidate.join("Brawlhalla.exe").is_file() {
                resolved = Some(candidate);
                break;
            }
        }

        let game_dir = match resolved {
            Some(dir) => dir,
            None => {
                installation.is_valid = false;
                installation
                    .validation_messages
                    .push("Could not locate Brawlhalla installation directory.".into());
                return installation;
            }
        };

        installation.game_path = game_dir.to_string_lossy().to_string();
        installation.exe_path = game_dir.join("Brawlhalla.exe").to_string_lossy().to_string();
        installation.game_swz_path = game_dir.join("Game.swz").to_string_lossy().to_string();
        installation.air_swf_path = game_dir.join("BrawlhallaAir.swf").to_string_lossy().to_string();

        if !Path::new(&installation.exe_path).is_file() {
            installation.validation_messages.push("Missing Brawlhalla.exe".into());
        }
        if !Path::new(&installation.game_swz_path).is_file() {
            installation.validation_messages.push("Missing Game.swz".into());
        }
        if !Path::new(&installation.air_swf_path).is_file() {
            installation.validation_messages.push("Missing BrawlhallaAir.swf".into());
        }

        // Count loose .swf files
        if let Ok(entries) = fs::read_dir(&game_dir) {
            let count = entries
                .flatten()
                .filter(|e| e.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("swf")))
                .count();
            installation.discovered_swf_count = count;
        }

        // Read build ID from appmanifest_291550.acf if exists
        let manifest_path = game_dir.join("../../appmanifest_291550.acf");
        if let Ok(content) = fs::read_to_string(manifest_path) {
            if let Some(pos) = content.find("\"buildid\"") {
                let rest = &content[pos + 9..];
                if let Some(start) = rest.find('"') {
                    let val = &rest[start + 1..];
                    if let Some(end) = val.find('"') {
                        installation.build_id = val[..end].to_string();
                    }
                }
            }
        }

        // Version detect from BrawlhallaAir.swf (stream first 512KB with zero allocation)
        if let Ok(file) = fs::File::open(&installation.air_swf_path) {
            use std::io::Read;
            let mut prefix = Vec::with_capacity(512 * 1024);
            if file.take(512 * 1024).read_to_end(&mut prefix).is_ok() {
                if let Some(pos) = prefix.windows(3).position(|w| w == b"11.") {
                    let end_pos = (pos + 15).min(prefix.len());
                    let segment = &prefix[pos..end_pos];
                    let len = segment.iter().position(|&b| !b.is_ascii_digit() && b != b'.').unwrap_or(segment.len());
                    if let Ok(ver_str) = std::str::from_utf8(&segment[..len]) {
                        installation.version = ver_str.to_string();
                    } else {
                        installation.version = "11.00+".into();
                    }
                } else {
                    installation.version = "11.00+".into();
                }
            }
        }

        installation.is_valid = installation.validation_messages.is_empty();

        // Save valid path to LocalAppData config for permanent persistence
        if installation.is_valid {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                let dir = PathBuf::from(&local_app_data).join("Skinner");
                let _ = fs::create_dir_all(&dir);
                let _ = fs::write(dir.join("game_path.txt"), &installation.game_path);
            }
        }

        installation
    }
}

fn get_steam_library_candidates(steam_path: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let root = PathBuf::from(steam_path);
    out.push(root.join("steamapps/common/Brawlhalla"));

    let vdf_path = root.join("steamapps/libraryfolders.vdf");
    if let Ok(content) = fs::read_to_string(vdf_path) {
        for line in content.lines() {
            if line.contains("\"path\"") {
                if let Some((_, right)) = line.split_once("\"path\"") {
                    let clean = right.trim().trim_matches('"').replace("\\\\", "\\");
                    if !clean.is_empty() {
                        out.push(PathBuf::from(clean).join("steamapps/common/Brawlhalla"));
                    }
                }
            }
        }
    }
    out
}
