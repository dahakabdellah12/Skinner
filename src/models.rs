#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallation {
    pub game_path: String,
    pub exe_path: String,
    pub game_swz_path: String,
    pub air_swf_path: String,
    pub version: String,
    pub build_id: String,
    pub discovered_swf_count: usize,
    pub is_valid: bool,
    pub validation_messages: Vec<String>,
}

impl Default for GameInstallation {
    fn default() -> Self {
        Self {
            game_path: String::new(),
            exe_path: String::new(),
            game_swz_path: String::new(),
            air_swf_path: String::new(),
            version: "Unknown".into(),
            build_id: "Unknown".into(),
            discovered_swf_count: 0,
            is_valid: false,
            validation_messages: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegendInfo {
    #[serde(default)]
    pub hero_id: i32,
    #[serde(default)]
    pub hero_name: String,
    #[serde(default)]
    pub hero_key: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub weapon1: String,
    #[serde(default)]
    pub weapon2: String,
    #[serde(default)]
    pub icon_name: String,
    #[serde(default)]
    pub pose_symbol: String,
    #[serde(default)]
    pub primary_swf: String,
    #[serde(default)]
    pub costumes: Vec<CostumeInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CostumeInfo {
    #[serde(default)]
    pub costume_id: i32,
    #[serde(default)]
    pub costume_name: String,
    #[serde(default)]
    pub owner_hero: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub swf_file: String,
    #[serde(default)]
    pub symbol_suffix: String,
    #[serde(default)]
    pub weapon_set: String,
    #[serde(default)]
    pub icon_name: String,
    #[serde(default)]
    pub icon_file_name: String,
    #[serde(default)]
    pub is_crossover: bool,
    #[serde(default)]
    pub is_metadev: bool,
    #[serde(default)]
    pub hands_art: String,
    #[serde(default)]
    pub hands_suffix: String,
    #[serde(default)]
    pub hands_lt_swap: String,
    #[serde(default)]
    pub hands_dk_swap: String,
    #[serde(default)]
    pub hands_skin_lt_swap: String,
    #[serde(default)]
    pub hands_skin_dk_swap: String,
    #[serde(default)]
    pub custom_art_layers: Vec<String>,
    #[serde(default)]
    pub swap_custom_arts: Vec<String>,
    #[serde(default)]
    pub default_cape: String,
    #[serde(default)]
    pub head_gfx: String,
    #[serde(default)]
    pub bone_override: String,
    #[serde(default)]
    pub name_sound_event: String,
    #[serde(default)]
    pub on_selected_sound_event: String,
    #[serde(default)]
    pub replacement_portrait: String,
    #[serde(default)]
    pub crossover_gem: String,
    #[serde(default)]
    pub base_colors: HashMap<String, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WeaponSkinInfo {
    #[serde(default)]
    pub weapon_skin_id: i32,
    #[serde(default)]
    pub weapon_skin_name: String,
    #[serde(default)]
    pub base_weapon: String,
    #[serde(default)]
    pub owner_hero: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub swf_file: String,
    #[serde(default)]
    pub symbol_suffix: String,
    #[serde(default)]
    pub weapon_icon: String,
    #[serde(default)]
    pub sound_bank: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub base_colors: HashMap<String, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ColorSchemeInfo {
    #[serde(default)]
    pub scheme_id: i32,
    #[serde(default)]
    pub scheme_name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub rarity: String,
    #[serde(default)]
    pub indicator_color: u32,
    #[serde(default)]
    pub swap_colors: HashMap<String, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KoEffectInfo {
    #[serde(default)]
    pub ko_effect_id: i32,
    #[serde(default)]
    pub ko_effect_name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub swf_file: String,
    #[serde(default)]
    pub anim_class: String,
    #[serde(default)]
    pub anim_scale: f32,
    #[serde(default)]
    pub sound_event: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub category: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub legend_name: String,
    pub target_costume_name: String,
    pub replacement_costume_name: String,
    pub weapon1_type: String,
    pub weapon1_skin_name: String,
    pub weapon2_type: String,
    pub weapon2_skin_name: String,
    pub color_scheme_name: String,
    pub custom_palette: HashMap<String, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub backup_id: String,
    pub timestamp: String,
    pub reason: String,
    pub file_original_hashes: HashMap<String, String>,
    pub backup_files_relative_path: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModificationRequest {
    pub target_legend: Option<LegendInfo>,
    pub target_costume: Option<CostumeInfo>,
    pub replacement_costume: Option<CostumeInfo>,
    pub target_weapon1: Option<WeaponSkinInfo>,
    pub replacement_weapon1: Option<WeaponSkinInfo>,
    pub target_weapon2: Option<WeaponSkinInfo>,
    pub replacement_weapon2: Option<WeaponSkinInfo>,
    pub selected_color_scheme: Option<ColorSchemeInfo>,
    pub legend_name: Option<String>,
    pub target_costume_name: Option<String>,
    pub replacement_costume_name: Option<String>,
    pub target_weapon1_name: Option<String>,
    pub replacement_weapon1_name: Option<String>,
    pub target_weapon2_name: Option<String>,
    pub replacement_weapon2_name: Option<String>,
    pub target_ko_effect: Option<KoEffectInfo>,
    pub replacement_ko_effect: Option<KoEffectInfo>,
    pub target_ko_effect_name: Option<String>,
    pub replacement_ko_effect_name: Option<String>,
    pub color_scheme_name: Option<String>,
    #[serde(default)]
    pub custom_palette: HashMap<String, u32>,
    #[serde(default)]
    pub auto_restart: bool,
}

impl ModificationRequest {
    pub fn get_legend_name(&self) -> String {
        self.target_legend.as_ref().map(|l| l.hero_name.clone())
            .or_else(|| self.legend_name.clone())
            .unwrap_or_default()
    }

    pub fn get_target_costume_name(&self) -> String {
        self.target_costume.as_ref().map(|c| c.costume_name.clone())
            .or_else(|| self.target_costume_name.clone())
            .unwrap_or_default()
    }

    pub fn get_replacement_costume_name(&self) -> String {
        self.replacement_costume.as_ref().map(|c| c.costume_name.clone())
            .or_else(|| self.replacement_costume_name.clone())
            .unwrap_or_else(|| self.get_target_costume_name())
    }

    pub fn get_target_weapon1_name(&self) -> Option<String> {
        self.target_weapon1.as_ref().map(|w| w.weapon_skin_name.clone())
            .or_else(|| self.target_weapon1_name.clone())
    }

    pub fn get_replacement_weapon1_name(&self) -> Option<String> {
        self.replacement_weapon1.as_ref().map(|w| w.weapon_skin_name.clone())
            .or_else(|| self.replacement_weapon1_name.clone())
    }

    pub fn get_target_weapon2_name(&self) -> Option<String> {
        self.target_weapon2.as_ref().map(|w| w.weapon_skin_name.clone())
            .or_else(|| self.target_weapon2_name.clone())
    }

    pub fn get_replacement_weapon2_name(&self) -> Option<String> {
        self.replacement_weapon2.as_ref().map(|w| w.weapon_skin_name.clone())
            .or_else(|| self.replacement_weapon2_name.clone())
    }

    pub fn get_target_ko_effect_name(&self) -> Option<String> {
        self.target_ko_effect.as_ref().map(|k| k.ko_effect_name.clone())
            .or_else(|| self.target_ko_effect_name.clone())
    }

    pub fn get_replacement_ko_effect_name(&self) -> Option<String> {
        self.replacement_ko_effect.as_ref().map(|k| k.ko_effect_name.clone())
            .or_else(|| self.replacement_ko_effect_name.clone())
    }

    pub fn get_color_scheme_name(&self) -> Option<String> {
        self.selected_color_scheme.as_ref().map(|cs| cs.scheme_name.clone())
            .or_else(|| self.color_scheme_name.clone())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModificationResult {
    pub success: bool,
    pub message: String,
    pub modified_files: Vec<String>,
    pub backup_id: String,
    pub operation_log: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub generated_at: String,
    pub installation: GameInstallation,
    pub total_legends: usize,
    pub total_costumes: usize,
    pub total_weapon_skins: usize,
    pub total_color_schemes: usize,
    pub modified_files: Vec<String>,
    pub backup_count: usize,
    pub is_game_running: bool,
    pub health_checks: Vec<String>,
}
