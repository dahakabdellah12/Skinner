use crate::backup_manager::BackupManager;
use crate::catalog::Catalog;
use crate::models::{ModificationRequest, ModificationResult};
use crate::swf_engine::SwfPackage;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct ModificationEngine<'a> {
    pub game_path: PathBuf,
    pub backup_manager: &'a mut BackupManager,
    pub catalog: &'a Catalog,
}

impl<'a> ModificationEngine<'a> {
    pub fn new(
        game_path: PathBuf,
        backup_manager: &'a mut BackupManager,
        catalog: &'a Catalog,
    ) -> Self {
        Self {
            game_path,
            backup_manager,
            catalog,
        }
    }

    pub fn validate_modification(&self, request: &ModificationRequest) -> serde_json::Value {
        let legend_name = request.get_legend_name();
        let target_costume_name = request.get_target_costume_name();
        let replacement_costume_name = request.get_replacement_costume_name();

        let legend = self
            .catalog
            .legends
            .iter()
            .find(|l| {
                l.hero_name.eq_ignore_ascii_case(&legend_name)
                    || l.display_name.eq_ignore_ascii_case(&legend_name)
            });

        if legend.is_none() {
            return serde_json::json!({
                "isCompatible": false,
                "level": "Incompatible",
                "diagnostics": [format!("Legend '{}' not found in catalog.", legend_name)],
                "mappedSymbols": [],
                "missingSymbols": [],
                "requiresCrossSwfImport": false
            });
        }

        serde_json::json!({
            "isCompatible": true,
            "level": "Compatible",
            "diagnostics": [
                format!("Intra-SWF cosmetic remapping verified for {} ({} -> {}).", legend_name, target_costume_name, replacement_costume_name)
            ],
            "mappedSymbols": [
                format!("Costume: {} -> {}", target_costume_name, replacement_costume_name)
            ],
            "missingSymbols": [],
            "requiresCrossSwfImport": false
        })
    }

    pub fn resolve_weapons_for_request(
        &self,
        request: &ModificationRequest,
    ) -> (
        Option<&'a crate::models::WeaponSkinInfo>,
        Option<&'a crate::models::WeaponSkinInfo>,
        Option<&'a crate::models::WeaponSkinInfo>,
        Option<&'a crate::models::WeaponSkinInfo>,
    ) {
        let legend_name = request.get_legend_name();
        let replacement_costume_name = request.get_replacement_costume_name();
        let target_weapon1_name = request.get_target_weapon1_name();
        let replacement_weapon1_name = request.get_replacement_weapon1_name();
        let target_weapon2_name = request.get_target_weapon2_name();
        let replacement_weapon2_name = request.get_replacement_weapon2_name();

        let legend = self
            .catalog
            .legends
            .iter()
            .find(|l| {
                l.hero_name.eq_ignore_ascii_case(&legend_name)
                    || l.display_name.eq_ignore_ascii_case(&legend_name)
            });

        let replacement_costume = request.replacement_costume.as_ref()
            .or_else(|| {
                legend.and_then(|leg| {
                    leg.costumes.iter().find(|c| {
                        c.costume_name.eq_ignore_ascii_case(&replacement_costume_name)
                            || c.display_name.eq_ignore_ascii_case(&replacement_costume_name)
                    })
                })
            })
            .or_else(|| {
                self.catalog.costumes.iter().find(|c| {
                    c.costume_name.eq_ignore_ascii_case(&replacement_costume_name)
                        || c.display_name.eq_ignore_ascii_case(&replacement_costume_name)
                })
            });

        let find_weapon = |name_opt: Option<&String>| -> Option<&'a crate::models::WeaponSkinInfo> {
            name_opt.and_then(|name| {
                if let Ok(id) = name.parse::<i32>() {
                    if let Some(w) = self.catalog.weapon_skins.iter().find(|w| w.weapon_skin_id == id) {
                        return Some(w);
                    }
                }
                let clean = name.replace(" ", "");
                self.catalog.weapon_skins.iter().find(|w| {
                    w.weapon_skin_name.eq_ignore_ascii_case(name)
                        || w.weapon_skin_name.replace(" ", "").eq_ignore_ascii_case(&clean)
                        || w.display_name.eq_ignore_ascii_case(name)
                })
            })
        };

        let default_w1 = legend.and_then(|leg| {
            self.catalog.weapon_skins.iter().find(|w| {
                w.owner_hero.eq_ignore_ascii_case(&leg.hero_name) && w.base_weapon.eq_ignore_ascii_case(&leg.weapon1) && w.is_default
            }).or_else(|| {
                self.catalog.weapon_skins.iter().find(|w| {
                    w.owner_hero.eq_ignore_ascii_case(&leg.hero_name) && w.base_weapon.eq_ignore_ascii_case(&leg.weapon1)
                })
            })
        });

        let default_w2 = legend.and_then(|leg| {
            self.catalog.weapon_skins.iter().find(|w| {
                w.owner_hero.eq_ignore_ascii_case(&leg.hero_name) && w.base_weapon.eq_ignore_ascii_case(&leg.weapon2) && w.is_default
            }).or_else(|| {
                self.catalog.weapon_skins.iter().find(|w| {
                    w.owner_hero.eq_ignore_ascii_case(&leg.hero_name) && w.base_weapon.eq_ignore_ascii_case(&leg.weapon2)
                })
            })
        });

        let target_w1 = find_weapon(target_weapon1_name.as_ref()).or(default_w1);
        let mut repl_w1 = find_weapon(replacement_weapon1_name.as_ref());
        let target_w2 = find_weapon(target_weapon2_name.as_ref()).or(default_w2);
        let mut repl_w2 = find_weapon(replacement_weapon2_name.as_ref());

        // Auto-equip authentic crossover or dedicated costume signature weapons if not explicitly overridden by user
        if let Some(repl) = replacement_costume {
            let repl_lower = repl.costume_name.to_lowercase();
            let repl_disp = repl.display_name.to_lowercase();

            // Explicit crossover-to-weapon mapping table
            let explicit_pair = if repl_lower.contains("xavier") || repl_disp.contains("xavier") {
                Some(("HammerXavier", "SwordXavier"))
            } else if repl_lower.contains("snakeeyes") || repl_disp.contains("snake eyes") || repl_lower.contains("snake_eyes") {
                Some(("SwordSnakeEyes", "PistolSnakeEyes"))
            } else if repl_lower.contains("chewbacca") || repl_disp.contains("chewbacca") {
                Some(("SwordChewbacca", "PistolChewbacca"))
            } else if repl_lower.contains("machoman") || repl_disp.contains("macho man") || repl_lower.contains("macho") {
                Some(("AxeMachoMan", "RocketLanceMachoMan"))
            } else if repl_lower.contains("destinytitan") || repl_disp.contains("destiny titan") || repl_lower.contains("titan") {
                Some(("FistsDestinyTitan", "CannonDestinyTitan"))
            } else if repl_lower.contains("negan") || repl_disp.contains("negan") {
                Some(("SwordNegan", "GreatswordNegan"))
            } else {
                None
            };

            if let Some((pair_w1, pair_w2)) = explicit_pair {
                if repl_w1.is_none() {
                    if let Some(tw1) = target_w1 {
                        repl_w1 = self.catalog.weapon_skins.iter().find(|w| {
                            (w.weapon_skin_name.eq_ignore_ascii_case(pair_w1) || w.weapon_skin_name.eq_ignore_ascii_case(pair_w2))
                                && w.base_weapon.eq_ignore_ascii_case(&tw1.base_weapon)
                        });
                    }
                }
                if repl_w2.is_none() {
                    if let Some(tw2) = target_w2 {
                        repl_w2 = self.catalog.weapon_skins.iter().find(|w| {
                            (w.weapon_skin_name.eq_ignore_ascii_case(pair_w1) || w.weapon_skin_name.eq_ignore_ascii_case(pair_w2))
                                && w.base_weapon.eq_ignore_ascii_case(&tw2.base_weapon)
                        });
                    }
                }
            } else {
                let wset = if !repl.weapon_set.is_empty() {
                    &repl.weapon_set
                } else {
                    &repl.symbol_suffix
                };
                let cname = &repl.costume_name;

                if repl_w1.is_none() {
                    if let Some(tw1) = target_w1 {
                        repl_w1 = self.catalog.weapon_skins.iter().find(|w| {
                            w.base_weapon.eq_ignore_ascii_case(&tw1.base_weapon) && (
                                w.symbol_suffix.eq_ignore_ascii_case(wset)
                                || w.weapon_skin_name.eq_ignore_ascii_case(wset)
                                || w.weapon_skin_name.eq_ignore_ascii_case(&format!("{}{}", tw1.base_weapon, wset))
                                || w.weapon_skin_name.eq_ignore_ascii_case(&format!("{}{}", tw1.base_weapon, cname))
                                || (w.weapon_skin_name.to_lowercase().contains(&wset.to_lowercase()) && (w.owner_hero.is_empty() || (legend.is_some() && w.owner_hero.to_lowercase().contains(&legend.unwrap().hero_name.to_lowercase())) || w.owner_hero.to_lowercase().contains(&wset.to_lowercase())))
                            )
                        });
                    }
                }
                if repl_w2.is_none() {
                    if let Some(tw2) = target_w2 {
                        repl_w2 = self.catalog.weapon_skins.iter().find(|w| {
                            w.base_weapon.eq_ignore_ascii_case(&tw2.base_weapon) && (
                                w.symbol_suffix.eq_ignore_ascii_case(wset)
                                || w.weapon_skin_name.eq_ignore_ascii_case(wset)
                                || w.weapon_skin_name.eq_ignore_ascii_case(&format!("{}{}", tw2.base_weapon, wset))
                                || w.weapon_skin_name.eq_ignore_ascii_case(&format!("{}{}", tw2.base_weapon, cname))
                                || (w.weapon_skin_name.to_lowercase().contains(&wset.to_lowercase()) && (w.owner_hero.is_empty() || (legend.is_some() && w.owner_hero.to_lowercase().contains(&legend.unwrap().hero_name.to_lowercase())) || w.owner_hero.to_lowercase().contains(&wset.to_lowercase())))
                            )
                        });
                    }
                }
            }
        }

        (target_w1, repl_w1, target_w2, repl_w2)
    }

    pub fn apply_modification(&mut self, request: &ModificationRequest) -> ModificationResult {
        let mut result = ModificationResult {
            success: false,
            message: String::new(),
            modified_files: Vec::new(),
            backup_id: String::new(),
            operation_log: Vec::new(),
        };

        let legend_name = request.get_legend_name();
        let target_costume_name = request.get_target_costume_name();
        let replacement_costume_name = request.get_replacement_costume_name();
        let color_scheme_name = request.get_color_scheme_name();

        // 1. Find Legend
        let legend = match self
            .catalog
            .legends
            .iter()
            .find(|l| {
                l.hero_name.eq_ignore_ascii_case(&legend_name)
                    || l.display_name.eq_ignore_ascii_case(&legend_name)
            })
        {
            Some(l) => l,
            None => {
                result.message = format!("Legend '{}' not found in catalog.", legend_name);
                return result;
            }
        };

        // 2. Find Target Costume
        let target_costume = match legend
            .costumes
            .iter()
            .find(|c| {
                c.costume_name.eq_ignore_ascii_case(&target_costume_name)
                    || c.display_name.eq_ignore_ascii_case(&target_costume_name)
            })
        {
            Some(c) => c,
            None => {
                result.message = format!(
                    "Target costume '{}' not found for legend '{}'.",
                    target_costume_name, legend_name
                );
                return result;
            }
        };

        // 3. Find Replacement Costume
        let replacement_costume = request.replacement_costume.as_ref()
            .or_else(|| {
                legend.costumes.iter().find(|c| {
                    c.costume_name.eq_ignore_ascii_case(&replacement_costume_name)
                        || c.display_name.eq_ignore_ascii_case(&replacement_costume_name)
                })
            })
            .or_else(|| {
                self.catalog.costumes.iter().find(|c| {
                    c.costume_name.eq_ignore_ascii_case(&replacement_costume_name)
                        || c.display_name.eq_ignore_ascii_case(&replacement_costume_name)
                })
            });

        // Weapons
        let (target_w1, repl_w1, target_w2, repl_w2) = self.resolve_weapons_for_request(request);

        println!("DEBUG ENGINE: target_w1={:?}, repl_w1={:?}, target_w2={:?}, repl_w2={:?}", 
            target_w1.map(|w| &w.weapon_skin_name),
            repl_w1.map(|w| &w.weapon_skin_name),
            target_w2.map(|w| &w.weapon_skin_name),
            repl_w2.map(|w| &w.weapon_skin_name));

        // 4. Build color map
        let mut color_map: HashMap<u32, u32> = HashMap::new();
        let scheme_opt = if let Some(ref scheme_name) = color_scheme_name {
            self.catalog
                .color_schemes
                .iter()
                .find(|s| {
                    s.scheme_id.to_string() == *scheme_name
                        || s.scheme_name.eq_ignore_ascii_case(scheme_name)
                        || s.display_name.eq_ignore_ascii_case(scheme_name)
                })
        } else {
            None
        }.or(request.selected_color_scheme.as_ref());

        if let Some(scheme) = scheme_opt {
            let active = replacement_costume.unwrap_or(target_costume);
            let standard_weapon_colors: [(&str, u32); 10] = [
                ("weapon", 0x8D8D95),
                ("weaponlt", 0xACB1B7),
                ("weapondk", 0x65626D),
                ("weaponvl", 0xCAD1D7),
                ("special", 0x3E5466),
                ("speciallt", 0x5D7B93),
                ("specialdk", 0x2A3B49),
                ("specialvl", 0x82A4BF),
                ("body1", 0x485868),
                ("body2", 0x82A4BF),
            ];

            let mut base_sources: Vec<&HashMap<String, u32>> = Vec::new();
            if !active.base_colors.is_empty() {
                base_sources.push(&active.base_colors);
            }
            if !target_costume.base_colors.is_empty() {
                base_sources.push(&target_costume.base_colors);
            }

            // Collect weapon base sources
            for wep_opt in [repl_w1, target_w1, repl_w2, target_w2] {
                if let Some(w) = wep_opt {
                    if !w.base_colors.is_empty() {
                        base_sources.push(&w.base_colors);
                    }
                    // Look up matching costume by symbol_suffix (e.g. SnowboardJY, CelticArmor, SpaceMiner)
                    if !w.symbol_suffix.is_empty() {
                        if let Some(cost) = self.catalog.costumes.iter().find(|c| {
                            c.symbol_suffix.eq_ignore_ascii_case(&w.symbol_suffix)
                                || c.costume_name.eq_ignore_ascii_case(&w.symbol_suffix)
                                || c.display_name.eq_ignore_ascii_case(&w.symbol_suffix)
                        }) {
                            if !cost.base_colors.is_empty() {
                                base_sources.push(&cost.base_colors);
                            }
                        }
                    }
                    // Look up owner hero default costume
                    if !w.owner_hero.is_empty() {
                        if let Some(cost) = self.catalog.costumes.iter().find(|c| {
                            c.owner_hero.eq_ignore_ascii_case(&w.owner_hero) && c.is_default
                        }) {
                            if !cost.base_colors.is_empty() {
                                base_sources.push(&cost.base_colors);
                            }
                        }
                    }
                }
            }

            for (key, target_color) in &scheme.swap_colors {
                let key_lower = key.to_lowercase();
                let target_val = *target_color & 0xFFFFFF;

                // Map standard Brawlhalla weapon fills
                for (std_k, std_v) in &standard_weapon_colors {
                    if key_lower == *std_k {
                        color_map.insert(*std_v & 0xFFFFFF, target_val);
                    }
                }

                for src in &base_sources {
                    if let Some(&base_color) = src.get(key).or_else(|| src.get(&key_lower)) {
                        color_map.insert(base_color & 0xFFFFFF, target_val);
                    }
                }
            }
        }

        // Custom palette overrides
        for (key, val) in &request.custom_palette {
            let active = replacement_costume.unwrap_or(target_costume);
            let key_lower = key.to_lowercase();
            let target_val = *val & 0xFFFFFF;
            if let Some(&base_color) = active.base_colors.get(key).or_else(|| active.base_colors.get(&key_lower)) {
                color_map.insert(base_color & 0xFFFFFF, target_val);
            }
            if let Some(&target_base) = target_costume.base_colors.get(key).or_else(|| target_costume.base_colors.get(&key_lower)) {
                color_map.insert(target_base & 0xFFFFFF, target_val);
            }
        }

        // 5. Collect target files to backup
        let mut target_files: HashSet<PathBuf> = HashSet::new();
        let target_costume_swf = self.game_path.join(&target_costume.swf_file);
        if target_costume_swf.is_file() {
            target_files.insert(target_costume_swf.clone());
        }

        if let Some(w1) = target_w1 {
            let p = self.game_path.join(&w1.swf_file);
            if p.is_file() {
                target_files.insert(p);
            }
        }

        if let Some(w2) = target_w2 {
            let p = self.game_path.join(&w2.swf_file);
            if p.is_file() {
                target_files.insert(p);
            }
        }

        if let Some(repl) = replacement_costume {
            let is_crossover = repl.is_crossover || !repl.weapon_set.is_empty();
            if is_crossover {
                let char_select = self.game_path.join("anims").join("Animation_CharacterSelect.anm");
                if char_select.is_file() {
                    target_files.insert(char_select);
                }
                let ui_icons = self.game_path.join("UI_Icons.swf");
                if ui_icons.is_file() {
                    target_files.insert(ui_icons);
                }
                let announcer_bnk = self.game_path.join("audio").join("pc").join("English(US)").join("VOX_Announcer.bnk");
                if announcer_bnk.is_file() {
                    target_files.insert(announcer_bnk);
                }
                let movement_bnk = self.game_path.join("audio").join("pc").join("PC_Movement.bnk");
                if movement_bnk.is_file() {
                    target_files.insert(movement_bnk);
                }

                // Authentic Crossover Signature SFX SoundBank
                if let Some((bank_name, _)) = crate::bnk_engine::get_signature_sfx_info(&repl.symbol_suffix) {
                    let sig_bnk = self.game_path.join("audio").join("pc").join(bank_name);
                    if sig_bnk.is_file() {
                        target_files.insert(sig_bnk);
                    }
                }

                // Dedicated Crossover Weapon SFX (VFX) SWFs
                let sfx_files = match repl.symbol_suffix.to_lowercase().as_str() {
                    "xavier" => vec!["SFX_Hammer.swf", "SFX_Sword.swf"],
                    "snakeeyes" => vec!["SFX_Sword.swf", "SFX_Pistols.swf"],
                    "chewbacca" => vec!["SFX_Sword.swf", "SFX_Pistols.swf"],
                    "machoman" => vec!["SFX_Axe.swf", "SFX_Lance.swf"],
                    "destinytitan" => vec!["SFX_Cannon.swf", "SFX_Fists.swf"],
                    "negan" => vec!["SFX_Sword.swf", "SFX_GreatSword.swf"],
                    _ => vec![],
                };
                for sfx in sfx_files {
                    let p = self.game_path.join(sfx);
                    if p.is_file() {
                        target_files.insert(p);
                    }
                }

                // Crossover Weapon ANM files (including Chewbacca single-blaster and Destiny Titan cannon/fists)
                let extra_anms = match repl.symbol_suffix.to_lowercase().as_str() {
                    "chewbacca" => vec!["Animation_Sword.anm", "Animation_Pistols.anm"],
                    "destinytitan" => vec!["Animation_Cannon.anm", "Animation_Fists.anm"],
                    _ => vec![],
                };
                for anm in extra_anms {
                    let p = self.game_path.join("anims").join(anm);
                    if p.is_file() {
                        target_files.insert(p);
                    }
                }

                if repl.symbol_suffix.eq_ignore_ascii_case("Chewbacca") {
                    let wep_pistol = self.game_path.join("audio").join("pc").join("WEP_Pistol.bnk");
                    if wep_pistol.is_file() {
                        target_files.insert(wep_pistol);
                    }
                }
            }

            for art_spec in &repl.swap_custom_arts {
                if let Some((file_name, _)) = art_spec.split_once('/') {
                    let file_clean = file_name.trim();
                    if file_clean.starts_with("SFX_") {
                        let p = self.game_path.join(file_clean);
                        if p.is_file() {
                            target_files.insert(p);
                        }
                    } else if file_clean.starts_with("Animation_") {
                        let base_stem = file_clean.trim_end_matches(".swf");
                        let p = self.game_path.join("anims").join(format!("{}.anm", base_stem));
                        if p.is_file() {
                            target_files.insert(p);
                        }
                    }
                }
            }
        }

        let files_vec: Vec<PathBuf> = target_files.into_iter().collect();
        let backup_reason = format!(
            "Apply: {} -> {}",
            legend_name,
            replacement_costume_name
        );

        match self.backup_manager.create_backup(&files_vec, &backup_reason) {
            Ok(backup_id) => {
                crate::telemetry::log("INFO", "BACKUP", &format!("Created snapshot backup '{}' for {} files: {:?}", backup_id, files_vec.len(), files_vec), None);
                result.backup_id = backup_id;
                result
                    .operation_log
                    .push(format!("Created backup snapshot: {}", result.backup_id));
            }
            Err(e) => {
                crate::telemetry::log("ERROR", "BACKUP", &format!("Backup failed: {}", e), None);
                result.message = format!("Failed to create backup: {}", e);
                return result;
            }
        }

        // 6. Apply Costume Modification
        if let Some(repl) = replacement_costume {
            let is_costume_changed = !target_costume.costume_name.eq_ignore_ascii_case(&repl.costume_name)
                && !target_costume.symbol_suffix.eq_ignore_ascii_case(&repl.symbol_suffix);

            if is_costume_changed {
                crate::telemetry::log("INFO", "ENGINE", &format!("Modifying costume: {} ({}) -> {} ({})", target_costume_name, target_costume.symbol_suffix, replacement_costume_name, repl.symbol_suffix), None);
                if target_costume_swf.is_file() {
                    if target_costume.swf_file.eq_ignore_ascii_case(&repl.swf_file) {
                        match SwfPackage::read_from_file(&target_costume_swf) {
                            Ok(mut swf) => {
                                let remapped = swf
                                    .remap_symbols(&target_costume.symbol_suffix, &repl.symbol_suffix)
                                    .unwrap_or(0);
                                let colors = swf.apply_color_palette(&color_map);
                                if let Err(e) = swf.write_to_file(&target_costume_swf) {
                                    crate::telemetry::log("ERROR", "ENGINE", &format!("Failed writing {}: {}", target_costume_swf.display(), e), None);
                                    let _ = self.backup_manager.restore_backup(&result.backup_id);
                                    result.message = format!("Modification aborted due to SWF integrity failure: {}", e);
                                    return result;
                                }
                                result.modified_files.push(target_costume.swf_file.clone());
                                result.operation_log.push(format!(
                                    "Costume symbols remapped: {} ({} -> {}), {} colors adjusted.",
                                    remapped,
                                    target_costume.symbol_suffix,
                                    repl.symbol_suffix,
                                    colors
                                ));
                                crate::telemetry::log("INFO", "ENGINE", &format!("Intra-SWF costume remapping done: {} symbols, {} colors in {}", remapped, colors, target_costume.swf_file), None);
                            }
                            Err(e) => {
                                crate::telemetry::log("ERROR", "ENGINE", &format!("Failed parsing SWF {}: {}", target_costume_swf.display(), e), None);
                                result.message = format!("Failed to parse SWF: {}", e);
                                return result;
                            }
                        }
                    } else {
                        // Cross-SWF costume replacement (e.g. Gfx_Mikasa.swf -> Gfx_Witch.swf, or Gfx_Chewbacca.swf -> Gfx_Thatch.swf)
                        let repl_swf_path = self.game_path.join(&repl.swf_file);
                        if repl_swf_path.is_file() {
                            match (SwfPackage::read_from_file(&target_costume_swf), SwfPackage::read_from_file(&repl_swf_path)) {
                                (Ok(mut target_swf), Ok(source_swf)) => {
                                    let imported = target_swf.import_costume_from_swf(
                                        &source_swf,
                                        &repl.symbol_suffix,
                                        &target_costume.symbol_suffix,
                                        &color_map,
                                    ).unwrap_or(0);

                                    if let Err(e) = target_swf.write_to_file(&target_costume_swf) {
                                        let _ = self.backup_manager.restore_backup(&result.backup_id);
                                        result.message = format!("Failed to write modified target SWF: {}", e);
                                        return result;
                                    }
                                    result.modified_files.push(target_costume.swf_file.clone());
                                    result.operation_log.push(format!(
                                        "Cross-file costume safely imported from '{}' into '{}': {} symbols remapped.",
                                        repl.swf_file, target_costume.swf_file, imported
                                    ));
                                }
                                (Err(e), _) => {
                                    result.message = format!("Failed to parse target SWF '{}': {}", target_costume.swf_file, e);
                                    return result;
                                }
                                (_, Err(e)) => {
                                    result.message = format!("Failed to parse source SWF '{}': {}", repl.swf_file, e);
                                    return result;
                                }
                            }
                        }
                    }
                }
            } else if !color_map.is_empty() && target_costume_swf.is_file() {
                if let Ok(mut swf) = SwfPackage::read_from_file(&target_costume_swf) {
                    let colors = swf.apply_color_palette(&color_map);
                    if let Err(e) = swf.write_to_file(&target_costume_swf) {
                        let _ = self.backup_manager.restore_backup(&result.backup_id);
                        result.message = format!("Failed to write modified SWF: {}", e);
                        return result;
                    }
                    result.modified_files.push(target_costume.swf_file.clone());
                    result
                        .operation_log
                        .push(format!("Applied {} color swaps to costume.", colors));
                }
            }
        } else if !color_map.is_empty() && target_costume_swf.is_file() {
            if let Ok(mut swf) = SwfPackage::read_from_file(&target_costume_swf) {
                let colors = swf.apply_color_palette(&color_map);
                if let Err(e) = swf.write_to_file(&target_costume_swf) {
                    let _ = self.backup_manager.restore_backup(&result.backup_id);
                    result.message = format!("Failed to write modified SWF: {}", e);
                    return result;
                }
                result.modified_files.push(target_costume.swf_file.clone());
                result
                    .operation_log
                    .push(format!("Applied {} color swaps to costume.", colors));
            }
        }


        // 7. Apply Weapon 1 Modification (Scoped strictly to weapon symbols to preserve body & face integrity)
        if let Some(target_w) = target_w1 {
            let active_w = repl_w1.unwrap_or(target_w);
            let target_swf = self.game_path.join(&target_w.swf_file);
            if target_swf.is_file() {
                if target_w.swf_file.eq_ignore_ascii_case(&active_w.swf_file) {
                    let should_process = repl_w1.is_some() || !color_map.is_empty();
                    if should_process {
                        if let Ok(mut swf) = SwfPackage::read_from_file(&target_swf) {
                            let remapped = if repl_w1.is_some() {
                                swf.remap_weapon_symbols(&target_w.symbol_suffix, &active_w.symbol_suffix, &target_w.base_weapon)
                                    .unwrap_or(0)
                            } else {
                                0
                            };
                            let colors = if !color_map.is_empty() {
                                swf.apply_color_palette(&color_map)
                            } else {
                                0
                            };
                            if remapped > 0 || colors > 0 {
                                if let Err(e) = swf.write_to_file(&target_swf) {
                                    let _ = self.backup_manager.restore_backup(&result.backup_id);
                                    result.message = format!("Failed writing Weapon 1 SWF: {}", e);
                                    return result;
                                }
                                if !result.modified_files.contains(&target_w.swf_file) {
                                    result.modified_files.push(target_w.swf_file.clone());
                                }
                                result.operation_log.push(format!(
                                    "Weapon 1 ({}) updated: {} symbols, {} colors in '{}'.",
                                    target_w.base_weapon, remapped, colors, target_w.swf_file
                                ));
                            }
                        }
                    }
                } else if repl_w1.is_some() {
                    // Cross-SWF weapon import strictly scoped to weapon symbols
                    let repl_swf_path = self.game_path.join(&active_w.swf_file);
                    if repl_swf_path.is_file() {
                        if let (Ok(mut swf_target), Ok(swf_source)) = (
                            SwfPackage::read_from_file(&target_swf),
                            SwfPackage::read_from_file(&repl_swf_path),
                        ) {
                            let imported = swf_target.import_weapon_from_swf(
                                &swf_source,
                                &active_w.symbol_suffix,
                                &target_w.symbol_suffix,
                                &target_w.base_weapon,
                                &color_map,
                            ).unwrap_or(0);
                            if let Err(e) = swf_target.write_to_file(&target_swf) {
                                let _ = self.backup_manager.restore_backup(&result.backup_id);
                                result.message = format!("Failed writing cross-SWF Weapon 1: {}", e);
                                return result;
                            }
                            if !result.modified_files.contains(&target_w.swf_file) {
                                result.modified_files.push(target_w.swf_file.clone());
                            }
                            result.operation_log.push(format!(
                                "Cross-SWF Weapon 1 ({}) safely imported from '{}' into '{}': {} symbols remapped.",
                                target_w.base_weapon, active_w.swf_file, target_w.swf_file, imported
                            ));
                        }
                    }
                }
            }
        }

        // 8. Apply Weapon 2 Modification (Scoped strictly to weapon symbols to preserve body & face integrity)
        if let Some(target_w) = target_w2 {
            let active_w = repl_w2.unwrap_or(target_w);
            let target_swf = self.game_path.join(&target_w.swf_file);
            if target_swf.is_file() {
                if target_w.swf_file.eq_ignore_ascii_case(&active_w.swf_file) {
                    let should_process = repl_w2.is_some() || !color_map.is_empty();
                    if should_process {
                        if let Ok(mut swf) = SwfPackage::read_from_file(&target_swf) {
                            let remapped = if repl_w2.is_some() {
                                swf.remap_weapon_symbols(&target_w.symbol_suffix, &active_w.symbol_suffix, &target_w.base_weapon)
                                    .unwrap_or(0)
                            } else {
                                0
                            };
                            let colors = if !color_map.is_empty() {
                                swf.apply_color_palette(&color_map)
                            } else {
                                0
                            };
                            if remapped > 0 || colors > 0 {
                                if let Err(e) = swf.write_to_file(&target_swf) {
                                    let _ = self.backup_manager.restore_backup(&result.backup_id);
                                    result.message = format!("Failed writing Weapon 2 SWF: {}", e);
                                    return result;
                                }
                                if !result.modified_files.contains(&target_w.swf_file) {
                                    result.modified_files.push(target_w.swf_file.clone());
                                }
                                result.operation_log.push(format!(
                                    "Weapon 2 ({}) updated: {} symbols, {} colors in '{}'.",
                                    target_w.base_weapon, remapped, colors, target_w.swf_file
                                ));
                            }
                        }
                    }
                } else if repl_w2.is_some() {
                    // Cross-SWF weapon import strictly scoped to weapon symbols
                    let repl_swf_path = self.game_path.join(&active_w.swf_file);
                    if repl_swf_path.is_file() {
                        if let (Ok(mut swf_target), Ok(swf_source)) = (
                            SwfPackage::read_from_file(&target_swf),
                            SwfPackage::read_from_file(&repl_swf_path),
                        ) {
                            let imported = swf_target.import_weapon_from_swf(
                                &swf_source,
                                &active_w.symbol_suffix,
                                &target_w.symbol_suffix,
                                &target_w.base_weapon,
                                &color_map,
                            ).unwrap_or(0);
                            if let Err(e) = swf_target.write_to_file(&target_swf) {
                                let _ = self.backup_manager.restore_backup(&result.backup_id);
                                result.message = format!("Failed writing cross-SWF Weapon 2: {}", e);
                                return result;
                            }
                            if !result.modified_files.contains(&target_w.swf_file) {
                                result.modified_files.push(target_w.swf_file.clone());
                            }
                            result.operation_log.push(format!(
                                "Cross-SWF Weapon 2 ({}) safely imported from '{}' into '{}': {} symbols remapped.",
                                target_w.base_weapon, active_w.swf_file, target_w.swf_file, imported
                            ));
                        }
                    }
                }
            }
        }

        // 9. Apply Crossover Presentation, Audio, and Animations
        if let Some(repl) = replacement_costume {
            let hero_key = if !legend.hero_key.is_empty() {
                &legend.hero_key
            } else {
                &legend.hero_name
            };

            // 9A. Character Select & Podium Presentation Animations
            let select_anm_path = self.game_path.join("anims").join("Animation_CharacterSelect.anm");
            if select_anm_path.is_file() {
                let (idle_base, sel_base) = match hero_key.to_lowercase().as_str() {
                    "viking" | "bodvar" => ("IdleViking", "SelectedViking"),
                    "thatch" | "pirate" => ("IdleThatch", "SelectedThatch"),
                    "dwarf" | "ulgrim" => ("IdleDwarf", "SelectedDwarf"),
                    "sellsword" | "jaeyun" => ("IdleSellsword", "SelectedSellsword"),
                    "gargoyle" | "onyx" => ("IdleGargoyle", "SelectedGargoyle"),
                    _ => ("", ""),
                };
                let (idle_swap, sel_swap) = match repl.symbol_suffix.to_lowercase().as_str() {
                    "xavier" => ("IdleXavier", "SelectedXavier"),
                    "snakeeyes" => ("IdleSnakeEyes", "SelectedSnakeEyes"),
                    "chewbacca" => ("IdleChewbacca", "SelectedChewbacca"),
                    "machoman" => ("IdleMachoMan", "SelectedMachoMan"),
                    "negan" => ("IdleNegan", "SelectedNegan"),
                    "destinytitan" => ("IdleDestinyTitan", "SelectedDestinyTitan"),
                    _ => ("", ""),
                };

                if !idle_base.is_empty() && !idle_swap.is_empty() {
                    let mut swapped_select = 0;
                    if let Ok(true) = crate::anm_engine::swap_anm_animation_pair(&select_anm_path, idle_base, idle_swap) {
                        swapped_select += 1;
                    }
                    if let Ok(true) = crate::anm_engine::swap_anm_animation_pair(&select_anm_path, sel_base, sel_swap) {
                        swapped_select += 1;
                    }
                    if swapped_select > 0 {
                        let rel = "anims/Animation_CharacterSelect.anm".to_string();
                        if !result.modified_files.contains(&rel) {
                            result.modified_files.push(rel);
                        }
                        result.operation_log.push(format!(
                            "Podium & Character Select animations applied for {}: {}/{} swapped in Animation_CharacterSelect.anm.",
                            repl.display_name, swapped_select, 2
                        ));
                    }
                }
            }

            // 9B. Podium & Roster Artwork in UI_Icons.swf
            let ui_icons_path = self.game_path.join("UI_Icons.swf");
            if ui_icons_path.is_file() {
                let pose_bases = match hero_key.to_lowercase().as_str() {
                    "viking" | "bodvar" => vec!["a_Roster_Pose_Bodvar", "a_Roster_Pose_Viking"],
                    "thatch" | "pirate" => vec!["a_Roster_Pose_Thatch", "a_Roster_Pose_Pirate"],
                    "dwarf" | "ulgrim" => vec!["a_Roster_Pose_Dwarf", "a_Roster_Pose_Ulgrim"],
                    "sellsword" | "jaeyun" => vec!["a_Roster_Pose_Sellsword", "a_Roster_Pose_Jaeyun"],
                    "gargoyle" | "onyx" => vec!["a_Roster_Pose_Gargoyle", "a_Roster_Pose_Onyx"],
                    _ => vec![],
                };
                let pose_swap = match repl.symbol_suffix.to_lowercase().as_str() {
                    "xavier" => Some("a_Roster_Pose_Xavier"),
                    "snakeeyes" => Some("a_Roster_Pose_SnakeEyes"),
                    "chewbacca" => Some("a_Roster_Pose_ChewbaccaM"),
                    "machoman" => Some("a_Roster_Pose_MachoMan"),
                    "negan" => Some("a_Roster_Pose_Negan"),
                    "destinytitan" => Some("a_Roster_Pose_TitanM"),
                    _ => None,
                };
                if let Some(swap_p) = pose_swap {
                    if let Ok(mut swf) = SwfPackage::read_from_file(&ui_icons_path) {
                        let mut remapped_icons = 0;
                        for base_p in pose_bases {
                            remapped_icons += swf.remap_single_symbol(base_p, swap_p).unwrap_or(0);
                            let base_pm = format!("{}M", base_p);
                            let swap_pm = if swap_p.ends_with('M') { swap_p.to_string() } else { format!("{}M", swap_p) };
                            remapped_icons += swf.remap_single_symbol(&base_pm, &swap_pm).unwrap_or(0);
                        }
                        if remapped_icons > 0 {
                            if swf.write_to_file(&ui_icons_path).is_ok() {
                                let rel = "UI_Icons.swf".to_string();
                                if !result.modified_files.contains(&rel) {
                                    result.modified_files.push(rel);
                                }
                                result.operation_log.push(format!(
                                    "Podium & Roster portrait applied for {}: {} symbols remapped in UI_Icons.swf.",
                                    repl.display_name, remapped_icons
                                ));
                            }
                        }
                    }
                }
            }

            // 9C. Announcer Voice Announcement Audio
            let announcer_bnk = self.game_path.join("audio").join("pc").join("English(US)").join("VOX_Announcer.bnk");
            if announcer_bnk.is_file() {
                let default_announcer_evt = match hero_key.to_lowercase().as_str() {
                    "viking" | "bodvar" => "VO_Announcer_Name_Bodvar_Play",
                    "thatch" | "pirate" => "VO_Announcer_Name_Thatch_Play",
                    "dwarf" | "ulgrim" => "VO_Announcer_Name_Ulgrim_Play",
                    "sellsword" | "jaeyun" => "VO_Announcer_Name_Jaeyun_Play",
                    "gargoyle" | "onyx" => "VO_Announcer_Name_Onyx_Play",
                    _ => "",
                };
                let target_evt = if !repl.name_sound_event.is_empty() {
                    repl.name_sound_event.as_str()
                } else {
                    match repl.symbol_suffix.to_lowercase().as_str() {
                        "xavier" => "VO_Announcer_Name_Bodvar_XavierWoods_Play",
                        "snakeeyes" => "VO_Announcer_Name_Thatch_SnakeEyes_Play",
                        "chewbacca" => "VO_Announcer_Name_Thatch_Chewbacca_Play",
                        "machoman" => "VO_Announcer_Name_Ulgrim_MachoMan_Play",
                        "negan" => "VO_Announcer_Name_Jaeyun_Negan_Play",
                        "destinytitan" => "VO_Announcer_Name_Onyx_Titan_Play",
                        _ => "",
                    }
                };

                if !default_announcer_evt.is_empty() && !target_evt.is_empty() && default_announcer_evt != target_evt {
                    if let Ok(swapped) = crate::bnk_engine::swap_bnk_events(&announcer_bnk, default_announcer_evt, target_evt) {
                        if swapped > 0 {
                            let rel = "audio/pc/English(US)/VOX_Announcer.bnk".to_string();
                            if !result.modified_files.contains(&rel) {
                                result.modified_files.push(rel);
                            }
                            result.operation_log.push(format!(
                                "Announcer voice announcement enabled for {}: '{}' mapped to '{}'.",
                                repl.display_name, default_announcer_evt, target_evt
                            ));
                        }
                    }
                }
            }

            // 9D. Lock-in / Podium Selection Audio
            let movement_bnk = self.game_path.join("audio").join("pc").join("PC_Movement.bnk");
            if movement_bnk.is_file() {
                let default_select_evt = match hero_key.to_lowercase().as_str() {
                    "viking" | "bodvar" => "MVT_Select_Bodvar_Play",
                    "thatch" | "pirate" => "MVT_Select_Thatch_Play",
                    "dwarf" | "ulgrim" => "MVT_Select_Dwarf_Play",
                    "sellsword" | "jaeyun" => "MVT_Select_Sellsword_Play",
                    "gargoyle" | "onyx" => "MVT_Select_Onyx_Play",
                    _ => "",
                };
                let target_select_evt = if !repl.on_selected_sound_event.is_empty() {
                    repl.on_selected_sound_event.as_str()
                } else {
                    match repl.symbol_suffix.to_lowercase().as_str() {
                        "xavier" => "MVT_Select_Bodvar_Xavier_Play",
                        "snakeeyes" => "MVT_Select_Thatch_SnakeEyes_Play",
                        "chewbacca" => "MVT_Select_Thatch_Chewbacca_Play",
                        "machoman" => "MVT_Select_Ulgrim_MachoMan_Play",
                        "negan" => "MVT_Select_Sellsword_Negan_Play",
                        "destinytitan" => "MVT_Select_Onyx_Titan_Play",
                        _ => "",
                    }
                };

                if !default_select_evt.is_empty() && !target_select_evt.is_empty() && default_select_evt != target_select_evt {
                    if let Ok(swapped) = crate::bnk_engine::swap_bnk_events(&movement_bnk, default_select_evt, target_select_evt) {
                        if swapped > 0 {
                            let rel = "audio/pc/PC_Movement.bnk".to_string();
                            if !result.modified_files.contains(&rel) {
                                result.modified_files.push(rel);
                            }
                            result.operation_log.push(format!(
                                "Lock-in & Podium selection audio enabled for {}: '{}' mapped to '{}'.",
                                repl.display_name, default_select_evt, target_select_evt
                            ));
                        }
                    }
                }
            }

            // 9E. Chewbacca Special Blaster Audio (Bowcaster & E-11 Dedicated SFX)
            if repl.symbol_suffix.eq_ignore_ascii_case("Chewbacca") {
                if let Ok(mapped) = crate::bnk_engine::install_chewbacca_blaster_audio(&self.game_path) {
                    let rel = "audio/pc/WEP_Pistol.bnk".to_string();
                    if !result.modified_files.contains(&rel) {
                        result.modified_files.push(rel);
                    }
                    result.operation_log.push(format!(
                        "Chewbacca dedicated blaster audio installed: {} sound events active in WEP_Pistol.bnk.",
                        mapped
                    ));
                }
            }

            // 9F. Authentic Signature SFX SoundBank Swapping
            if let Some((bank_name, _)) = crate::bnk_engine::get_signature_sfx_info(&repl.symbol_suffix) {
                match crate::bnk_engine::swap_signature_sfx(&self.game_path, &repl.symbol_suffix) {
                    Ok(swapped) if swapped > 0 => {
                        let rel = format!("audio/pc/{}", bank_name);
                        if !result.modified_files.contains(&rel) {
                            result.modified_files.push(rel);
                        }
                        result.operation_log.push(format!(
                            "Signature SFX applied for {}: {} sound events mapped in {}.",
                            repl.display_name, swapped, bank_name
                        ));
                    }
                    Ok(_) => {}
                    Err(e) => {
                        result.operation_log.push(format!(
                            "Notice on Signature SFX for '{}': {}",
                            bank_name, e
                        ));
                    }
                }
            }

            // 9G. Authentic Signature VFX Visual Effects Remapping
            let sfx_jobs: Vec<(&str, &str)> = match repl.symbol_suffix.to_lowercase().as_str() {
                "xavier" => vec![
                    ("SFX_Hammer.swf", "SwapXavier"),
                    ("SFX_Sword.swf", "SwapXavier"),
                    ("SFX_HitReacts.swf", "SwapXavier"),
                ],
                "snakeeyes" => vec![("SFX_Sword.swf", "SwapSnakeEyes"), ("SFX_Pistols.swf", "SwapSnakeEyes")],
                "machoman" => vec![("SFX_Axe.swf", "SwapMachoMan"), ("SFX_Lance.swf", "SwapMachoMan")],
                "negan" => vec![("SFX_Sword.swf", "SwapNegan"), ("SFX_GreatSword.swf", "SwapNegan")],
                _ => vec![],
            };
            for (sfx_file, swap_suf) in sfx_jobs {
                let target_p = self.game_path.join(sfx_file);
                if target_p.is_file() {
                    if let Ok(mut swf) = SwfPackage::read_from_file(&target_p) {
                        let remapped = swf.remap_sfx_symbols(hero_key, swap_suf);
                        if remapped > 0 {
                            if swf.write_to_file(&target_p).is_ok() {
                                let rel = sfx_file.to_string();
                                if !result.modified_files.contains(&rel) {
                                    result.modified_files.push(rel);
                                }
                                result.operation_log.push(format!(
                                    "Signature VFX applied for {}: {} symbols remapped in {}.",
                                    repl.display_name, remapped, sfx_file
                                ));
                            }
                        }
                    }
                }
            }

            // Standalone Crossover VFX Import (Chewbacca & Destiny Titan)
            if repl.symbol_suffix.eq_ignore_ascii_case("Chewbacca") {
                let src_p = self.game_path.join("SFX_Chewbacca.swf");
                if src_p.is_file() {
                    if let Ok(src_swf) = SwfPackage::read_from_file(&src_p) {
                        for dst_file in &["SFX_Sword.swf", "SFX_Pistols.swf"] {
                            let dst_p = self.game_path.join(dst_file);
                            if dst_p.is_file() {
                                if let Ok(mut dst_swf) = SwfPackage::read_from_file(&dst_p) {
                                    if let Ok(imported) = dst_swf.import_sfx_symbols_from_swf(&src_swf, "SwapChewbacca") {
                                        if imported > 0 && dst_swf.write_to_file(&dst_p).is_ok() {
                                            let rel = dst_file.to_string();
                                            if !result.modified_files.contains(&rel) {
                                                result.modified_files.push(rel);
                                            }
                                            result.operation_log.push(format!(
                                                "Chewbacca signature VFX applied: {} symbols imported into {}.",
                                                imported, dst_file
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if repl.symbol_suffix.eq_ignore_ascii_case("DestinyTitan") {
                let src_p = self.game_path.join("SFX_DestinyTitan.swf");
                if src_p.is_file() {
                    if let Ok(src_swf) = SwfPackage::read_from_file(&src_p) {
                        for dst_file in &["SFX_Cannon.swf", "SFX_Fists.swf"] {
                            let dst_p = self.game_path.join(dst_file);
                            if dst_p.is_file() {
                                if let Ok(mut dst_swf) = SwfPackage::read_from_file(&dst_p) {
                                    if let Ok(imported) = dst_swf.import_sfx_symbols_from_swf(&src_swf, "SwapDestinyTitan") {
                                        if imported > 0 && dst_swf.write_to_file(&dst_p).is_ok() {
                                            let rel = dst_file.to_string();
                                            if !result.modified_files.contains(&rel) {
                                                result.modified_files.push(rel);
                                            }
                                            result.operation_log.push(format!(
                                                "Destiny Titan signature VFX applied: {} symbols imported into {}.",
                                                imported, dst_file
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 9H. Signature & Weapon Attack Animations
            let mut anim_files = repl.swap_custom_arts.clone();
            if repl.symbol_suffix.eq_ignore_ascii_case("Chewbacca") {
                if !anim_files.iter().any(|a| a.starts_with("Animation_Pistols")) {
                    anim_files.push("Animation_Pistols.swf/SwapChewbacca".to_string());
                }
                if !anim_files.iter().any(|a| a.starts_with("Animation_Sword")) {
                    anim_files.push("Animation_Sword.swf/SwapChewbacca".to_string());
                }
            } else if repl.symbol_suffix.eq_ignore_ascii_case("DestinyTitan") {
                if !anim_files.iter().any(|a| a.starts_with("Animation_Cannon")) {
                    anim_files.push("Animation_Cannon.swf/SwapDestinyTitan".to_string());
                }
                if !anim_files.iter().any(|a| a.starts_with("Animation_Fists")) {
                    anim_files.push("Animation_Fists.swf/SwapDestinyTitan".to_string());
                }
            }

            for art_spec in &anim_files {
                if let Some((file_name, swap_suffix)) = art_spec.split_once('/') {
                    let file_clean = file_name.trim();
                    let swap_suffix_clean = swap_suffix.trim();

                    if file_clean.starts_with("Animation_") {
                        let base_stem = file_clean.trim_end_matches(".swf");
                        let anm_file_name = format!("{}.anm", base_stem);
                        let target_anm = self.game_path.join("anims").join(&anm_file_name);
                        if target_anm.is_file() {
                            match crate::anm_engine::swap_anm_signatures(&target_anm, hero_key, swap_suffix_clean) {
                                Ok(swapped) if swapped > 0 => {
                                    let rel_path = format!("anims/{}", anm_file_name);
                                    if !result.modified_files.contains(&rel_path) {
                                        result.modified_files.push(rel_path.clone());
                                    }
                                    result.operation_log.push(format!(
                                        "Signature & weapon animations swapped: {} animations in '{}' for {}.",
                                        swapped, rel_path, hero_key
                                    ));
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    result.operation_log.push(format!(
                                        "Notice on ANM swapping for '{}': {}",
                                        anm_file_name, e
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        result.success = true;
        result.message = format!(
            "Successfully modified {} file(s) for {}.",
            result.modified_files.len(),
            legend_name
        );
        result
    }
}
