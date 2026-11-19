#![allow(dead_code)]
use crate::models::BackupRecord;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(Clone)]
struct CachedHash {
    len: u64,
    mtime: Option<SystemTime>,
    hash: String,
}

pub struct BackupManager {
    game_path: PathBuf,
    backup_root: PathBuf,
    manifest_path: PathBuf,
    records: Vec<BackupRecord>,
    hash_cache: Mutex<HashMap<String, CachedHash>>,
}

impl BackupManager {
    pub fn new(game_path: &str) -> Self {
        let game_buf = PathBuf::from(game_path);
        let backup_root = game_buf.join(".studio_backups");
        let manifest_path = backup_root.join("backup_manifest.json");

        let _ = fs::create_dir_all(&backup_root);

        let mut mgr = Self {
            game_path: game_buf,
            backup_root,
            manifest_path,
            records: Vec::new(),
            hash_cache: Mutex::new(HashMap::new()),
        };

        mgr.load_manifest();
        mgr
    }

    pub fn records(&self) -> &[BackupRecord] {
        &self.records
    }

    pub fn create_backup(&mut self, files: &[PathBuf], reason: &str) -> Result<String, String> {
        let timestamp = chrono::Utc::now();
        let backup_id = format!("backup_{}", timestamp.format("%Y%m%d_%H%M%S_%3f"));
        let backup_folder = self.backup_root.join(&backup_id);
        fs::create_dir_all(&backup_folder)
            .map_err(|e| format!("Failed to create backup dir: {}", e))?;

        let mut record = BackupRecord {
            backup_id: backup_id.clone(),
            timestamp: timestamp.to_rfc3339(),
            reason: reason.to_string(),
            file_original_hashes: HashMap::new(),
            backup_files_relative_path: HashMap::new(),
        };

        for full_path in files {
            if !full_path.exists() {
                continue;
            }

            let rel_path = match full_path.strip_prefix(&self.game_path) {
                Ok(p) => p.to_string_lossy().replace('\\', "/"),
                Err(_) => full_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            };

            let dest_path = backup_folder.join(&rel_path);
            if let Some(parent) = dest_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            if let Err(e) = fs::copy(full_path, &dest_path) {
                return Err(format!("Failed to backup {}: {}", rel_path, e));
            }

            let hash = Self::compute_sha256(full_path).unwrap_or_default();
            let backup_rel = dest_path
                .strip_prefix(&self.backup_root)
                .unwrap_or(&dest_path)
                .to_string_lossy()
                .replace('\\', "/");

            record.file_original_hashes.insert(rel_path.clone(), hash);
            record.backup_files_relative_path.insert(rel_path, backup_rel);
        }

        self.records.insert(0, record);
        self.save_manifest();
        self.invalidate_cache();
        Ok(backup_id)
    }

    pub fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.hash_cache.lock() {
            cache.clear();
        }
    }

    pub fn is_safe_relative_path(path_str: &str) -> bool {
        let trimmed = path_str.trim();
        if trimmed.is_empty() || trimmed.starts_with('/') || trimmed.starts_with('\\') || trimmed.contains(':') {
            return false;
        }
        let p = Path::new(trimmed);
        if p.is_absolute() {
            return false;
        }
        for comp in p.components() {
            match comp {
                std::path::Component::Normal(_) => {}
                _ => return false,
            }
        }
        true
    }

    pub fn restore_file(&self, relative_path: &str) -> Result<bool, String> {
        let clean_rel = relative_path.replace('\\', "/");
        if !Self::is_safe_relative_path(&clean_rel) {
            return Err(format!("Unsafe or invalid relative path: '{}'", relative_path));
        }

        let pristine_file = self.backup_root.join("_pristine").join(&clean_rel);
        let game_full = self.game_path.join(&clean_rel);

        if pristine_file.is_file() {
            if let Some(parent) = game_full.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::copy(&pristine_file, &game_full)
                .map_err(|e| format!("Restore failed from pristine for {}: {}", clean_rel, e))?;
            self.invalidate_cache();
            return Ok(true);
        }

        // Iterate backwards from oldest (pristine) backup
        for record in self.records.iter().rev() {
            if let Some(backup_rel) = record.backup_files_relative_path.get(&clean_rel) {
                let backup_full = self.backup_root.join(backup_rel);

                if backup_full.is_file() {
                    if let Some(parent) = game_full.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    fs::copy(&backup_full, &game_full)
                        .map_err(|e| format!("Restore failed for {}: {}", clean_rel, e))?;
                    self.invalidate_cache();
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn restore_all(&self) -> Result<usize, String> {
        let mut restored = 0;
        let mut seen = std::collections::HashSet::new();

        // 1. Restore from _pristine if available (highest priority ground truth)
        let pristine_dir = self.backup_root.join("_pristine");
        if pristine_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&pristine_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "swf" || ext == "anm" || ext == "bnk") {
                        if let Some(file_name) = path.file_name() {
                            let rel = file_name.to_string_lossy().to_string();
                            let target = self.game_path.join(&rel);
                            if fs::copy(&path, &target).is_ok() {
                                seen.insert(rel);
                                restored += 1;
                            }
                        }
                    }
                }
            }
        }

        // 2. Oldest records first to get pristine copies
        for record in self.records.iter().rev() {
            for (rel_path, backup_rel) in &record.backup_files_relative_path {
                if seen.insert(rel_path.clone()) {
                    let backup_full = self.backup_root.join(backup_rel);
                    let game_full = self.game_path.join(rel_path);

                    if backup_full.is_file() {
                        if let Some(parent) = game_full.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if fs::copy(&backup_full, &game_full).is_ok() {
                            restored += 1;
                        }
                    }
                }
            }
        }

        self.invalidate_cache();
        Ok(restored)
    }

    pub fn restore_backup(&self, backup_id: &str) -> Result<usize, String> {
        let record = self
            .records
            .iter()
            .find(|r| r.backup_id == backup_id)
            .ok_or_else(|| format!("Backup '{}' not found", backup_id))?;

        let mut restored = 0;
        for (rel_path, backup_rel) in &record.backup_files_relative_path {
            let backup_full = self.backup_root.join(backup_rel);
            let game_full = self.game_path.join(rel_path);

            if backup_full.is_file() {
                if let Some(parent) = game_full.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                fs::copy(&backup_full, &game_full)
                    .map_err(|e| format!("Rollback failed for {}: {}", rel_path, e))?;
                restored += 1;
            }
        }

        self.invalidate_cache();
        Ok(restored)
    }

    pub fn restore_legend(&self, legend_name: &str, catalog: &crate::catalog::Catalog) -> Result<usize, String> {
        let mut files_to_restore: std::collections::HashSet<String> = std::collections::HashSet::new();

        let legend_opt = catalog.legends.iter().find(|l| {
            l.hero_name.eq_ignore_ascii_case(legend_name)
                || l.display_name.eq_ignore_ascii_case(legend_name)
        });

        if let Some(legend) = legend_opt {
            // 1. All costume SWFs for this legend
            for c in &legend.costumes {
                if !c.swf_file.is_empty() {
                    files_to_restore.insert(c.swf_file.clone());
                }
            }

            // 2. All character-dedicated weapon SWFs owned exclusively by this legend
            for w in &catalog.weapon_skins {
                if w.owner_hero.eq_ignore_ascii_case(&legend.hero_name) {
                    if !w.swf_file.is_empty() && w.swf_file.starts_with("Gfx_") && !w.swf_file.starts_with("Gfx_Weapons") {
                        files_to_restore.insert(w.swf_file.clone());
                    }
                }
            }
        }

        // 3. Optional character select / crossover presentation assets associated with legend customizations
        files_to_restore.insert("UI_Icons.swf".to_string());
        files_to_restore.insert("anims/Animation_CharacterSelect.anm".to_string());
        files_to_restore.insert("audio/pc/English(US)/VOX_Announcer.bnk".to_string());
        files_to_restore.insert("audio/pc/PC_Movement.bnk".to_string());
        files_to_restore.insert("audio/pc/WEP_Pistol.bnk".to_string());

        // 6. Crossover signature BNK files
        let crossover_bnks = [
            "SIG_Hammer_01.bnk", "SIG_Sword_01.bnk", "SIG_Sword_02.bnk", "SIG_Pistol_01.bnk",
            "SIG_Pistol_02.bnk", "SIG_Axe_01.bnk", "SIG_Lance_01.bnk", "SIG_Fists_01.bnk",
            "SIG_Cannon_01.bnk", "SIG_Greatsword_01.bnk"
        ];
        for bnk in crossover_bnks {
            files_to_restore.insert(format!("audio/pc/{}", bnk));
        }

        let mut restored_count = 0;
        for rel_path in &files_to_restore {
            if let Ok(true) = self.restore_file(rel_path) {
                restored_count += 1;
            }
        }

        self.invalidate_cache();
        crate::telemetry::log("INFO", "RESTORE-LEGEND", &format!("Restored {} files for legend '{}'", restored_count, legend_name), None);
        Ok(restored_count)
    }

    /// Health-check and repair all sound effect files (SFX_*.swf).
    /// Ensures DirectX audio engine (XAudio2_9.dll) stability and eliminates 0xc0000005 crashes.
    pub fn repair_sfx_files(&self) -> Result<usize, String> {
        let pristine_dir = self.backup_root.join("_pristine");
        if !pristine_dir.is_dir() {
            return Ok(0);
        }

        let mut repaired = 0;
        if let Ok(entries) = fs::read_dir(&pristine_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("SFX_") && file_name.ends_with(".swf") {
                        let game_file = self.game_path.join(file_name);
                        let needs_repair = if !game_file.is_file() {
                            true
                        } else if fs::metadata(&game_file).map(|m| m.len() == 0).unwrap_or(true) {
                            true
                        } else {
                            // File exists and is non-empty; only repair if structurally damaged or corrupted
                            crate::swf_engine::SwfPackage::read_from_file(&game_file).is_err()
                        };

                        if needs_repair {
                            if fs::copy(&p, &game_file).is_ok() {
                                crate::telemetry::log("INFO", "REPAIR-SFX", &format!("Restored corrupted audio file {} to pristine state", file_name), None);
                                repaired += 1;
                            }
                        }
                    }
                }
            }
        }

        if repaired > 0 {
            self.invalidate_cache();
        }
        Ok(repaired)
    }

    /// High-performance file modification check with mtime and length caching.
    /// Eliminates redundant multi-megabyte disk reads on frequent UI polling pings.
    pub fn is_file_modified(&self, relative_path: &str) -> bool {
        let clean_rel = relative_path.replace('\\', "/");
        let full_path = self.game_path.join(&clean_rel);
        let metadata = match fs::metadata(&full_path) {
            Ok(m) if m.is_file() => m,
            _ => return false,
        };

        let file_len = metadata.len();
        let mtime = metadata.modified().ok();

        let current_hash = {
            let mut cache = self.hash_cache.lock().unwrap();
            if let Some(entry) = cache.get(&clean_rel) {
                if entry.len == file_len && entry.mtime == mtime {
                    entry.hash.clone()
                } else {
                    let h = match Self::compute_sha256(&full_path) {
                        Ok(hash) => hash,
                        Err(_) => return false,
                    };
                    cache.insert(clean_rel.clone(), CachedHash { len: file_len, mtime, hash: h.clone() });
                    h
                }
            } else {
                let h = match Self::compute_sha256(&full_path) {
                    Ok(hash) => hash,
                    Err(_) => return false,
                };
                cache.insert(clean_rel.clone(), CachedHash { len: file_len, mtime, hash: h.clone() });
                h
            }
        };

        // Check earliest backup hash
        for record in self.records.iter().rev() {
            if let Some(original_hash) = record.file_original_hashes.get(&clean_rel) {
                return !current_hash.eq_ignore_ascii_case(original_hash);
            }
        }

        false
    }

    pub fn count_modified_files(&self) -> usize {
        self.get_modified_files().len()
    }

    pub fn get_modified_files(&self) -> Vec<String> {
        let mut checked = std::collections::HashSet::new();
        let mut modified = Vec::new();

        for record in &self.records {
            for rel in record.file_original_hashes.keys() {
                if checked.insert(rel.clone()) && self.is_file_modified(rel) {
                    modified.push(rel.clone());
                }
            }
        }

        modified
    }

    pub fn compute_sha256(path: &Path) -> Result<String, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mut reader = std::io::BufReader::with_capacity(65536, file);
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let hash_bytes = hasher.finalize();
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut hex = String::with_capacity(64);
        for &b in hash_bytes.as_slice() {
            hex.push(HEX[(b >> 4) as usize] as char);
            hex.push(HEX[(b & 0xF) as usize] as char);
        }
        Ok(hex)
    }

    fn load_manifest(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.manifest_path) {
            if let Ok(records) = serde_json::from_str::<Vec<BackupRecord>>(&content) {
                self.records = records;
            }
        }
    }

    fn save_manifest(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.records) {
            let _ = fs::write(&self.manifest_path, json);
        }
    }
}
