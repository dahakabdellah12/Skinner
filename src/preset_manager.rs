#![allow(dead_code)]
use crate::models::Preset;
use std::fs;
use std::path::PathBuf;

pub struct PresetManager {
    file_path: PathBuf,
    presets: Vec<Preset>,
}

impl PresetManager {
    pub fn new() -> Self {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        let dir = PathBuf::from(&appdata).join("Skinner");
        let _ = fs::create_dir_all(&dir);
        let file_path = dir.join("presets.json");

        // Migrate legacy BrawlCraft presets if present and no Skinner presets exist yet
        if !file_path.exists() {
            let legacy = PathBuf::from(&appdata).join("BrawlCraft").join("presets.json");
            if legacy.exists() {
                let _ = fs::copy(&legacy, &file_path);
            }
        }

        let mut mgr = Self {
            file_path,
            presets: Vec::new(),
        };
        mgr.load();
        mgr
    }

    pub fn presets(&self) -> &[Preset] {
        &self.presets
    }

    pub fn save_preset(&mut self, preset: Preset) {
        if let Some(pos) = self.presets.iter().position(|p| p.id == preset.id) {
            self.presets[pos] = preset;
        } else {
            self.presets.push(preset);
        }
        self.save();
    }

    pub fn delete_preset(&mut self, id: &str) -> bool {
        let len = self.presets.len();
        self.presets.retain(|p| p.id != id);
        if self.presets.len() < len {
            self.save();
            true
        } else {
            false
        }
    }

    pub fn export_json(&self, id: &str) -> Option<String> {
        self.presets
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| serde_json::to_string_pretty(p).ok())
    }

    pub fn import_json(&mut self, json_str: &str) -> Result<Preset, String> {
        let mut preset: Preset = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid preset JSON: {}", e))?;
        preset.id = format!("preset_{}", chrono::Utc::now().timestamp_millis());
        preset.created_at = chrono::Utc::now().to_rfc3339();
        self.save_preset(preset.clone());
        Ok(preset)
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.file_path) {
            if let Ok(list) = serde_json::from_str::<Vec<Preset>>(&content) {
                self.presets = list;
            }
        }
    }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.presets) {
            let _ = fs::write(&self.file_path, json);
        }
    }
}
