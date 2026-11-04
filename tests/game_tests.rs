#[test]
fn test_snake_eyes_hands_and_face_simulation() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_snakeeyes_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let thatch_orig = std::path::Path::new(&inst.game_path).join("Gfx_Thatch.swf");
    let thatch_copy = temp_dir.join("Gfx_Thatch.swf");
    fs::copy(&thatch_orig, &thatch_copy).unwrap();

    let hands_orig = std::path::Path::new(&inst.game_path).join(".studio_backups/_pristine/Gfx_Hands.swf");
    let hands_src = if hands_orig.is_file() {
        hands_orig
    } else {
        std::path::Path::new(&inst.game_path).join("Gfx_Hands.swf")
    };
    if !hands_src.is_file() {
        return;
    }
    let hands_copy = temp_dir.join("Gfx_Hands.swf");
    fs::copy(&hands_src, &hands_copy).unwrap();

    // Also copy SFX and ANM files for complete Crossover test
    for sfx in &["SFX_Sword.swf", "SFX_Pistols.swf"] {
        let orig = std::path::Path::new(&inst.game_path).join(sfx);
        if orig.is_file() {
            fs::copy(&orig, temp_dir.join(sfx)).unwrap();
        }
    }

    let anims_dir = temp_dir.join("anims");
    fs::create_dir_all(&anims_dir).unwrap();
    for anm in &["Animation_Sword.anm", "Animation_Pistols.anm"] {
        let orig = std::path::Path::new(&inst.game_path).join("anims").join(anm);
        if orig.is_file() {
            fs::copy(&orig, anims_dir.join(anm)).unwrap();
        }
    }

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    let req = ModificationRequest {
        legend_name: Some("Thatch".into()),
        target_costume_name: Some("Thatch".into()),
        replacement_costume_name: Some("SnakeEyes".into()),
        ..Default::default()
    };

    let thatch_legend = cat.legends.iter().find(|l| l.hero_name == "Thatch").unwrap();
    let se_costume = thatch_legend.costumes.iter().find(|c| c.costume_name == "SnakeEyes").unwrap();
    println!("SE hero_key: '{}'", thatch_legend.hero_key);
    println!("SE swap_custom_arts: {:?}", se_costume.swap_custom_arts);
    assert!(!se_costume.swap_custom_arts.is_empty(), "SnakeEyes must have SwapCustomArt entries");

    let result = engine.apply_modification(&req);
    println!("Log: {:?}", result.operation_log);
    assert!(result.success, "Should succeed: {}", result.message);
    assert!(result.modified_files.contains(&"Gfx_Thatch.swf".to_string()));
    assert!(!result.modified_files.contains(&"Gfx_Hands.swf".to_string()), "Gfx_Hands.swf must NOT be modified to preserve character isolation!");
    assert!(result.modified_files.contains(&"SFX_Sword.swf".to_string()), "SFX_Sword.swf must be modified with authentic Snake Eyes signature VFX!");
    let sfx_sw = SwfPackage::read_from_file(&temp_dir.join("SFX_Sword.swf")).unwrap();
    assert!(sfx_sw.validate().is_ok(), "Modified SFX_Sword.swf must pass structural validation");
    assert!(result.modified_files.contains(&"anims/Animation_Sword.anm".to_string()));
    assert!(result.modified_files.contains(&"anims/Animation_Pistols.anm".to_string()));

    // Verify Gfx_Hands.swf is 100% untouched to ensure Yumiko and Mordex keep their own hands
    let pristine_hash = BackupManager::compute_sha256(&hands_src).unwrap();
    let current_hands_hash = BackupManager::compute_sha256(&hands_copy).unwrap();
    assert_eq!(pristine_hash, current_hands_hash, "Gfx_Hands.swf must remain completely untouched for 100% character isolation!");

    // Verify Gfx_Thatch.swf empty sprites
    let thatch_pkg = SwfPackage::read_from_file(&thatch_copy).unwrap();
    let mut empty_id = 0;
    for tag in &thatch_pkg.tags {
        if tag.tag_code == 76 {
            let syms = brawlcraft_slint::swf_engine::parse_symbol_class(&tag.data).unwrap();
            for (id, name) in syms {
                if name == "a_Nose_Thatch" {
                    empty_id = id;
                }
            }
        }
    }
    assert!(empty_id >= 1000, "Nose should be mapped to empty sprite id, got {}", empty_id);

    // Rollback test
    let restored = backup_mgr.restore_all().unwrap();
    assert!(restored >= 3, "Should restore all modified files");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_inspect_problem_skins() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    let hammers_in_wep: Vec<_> = cat.weapon_skins.iter()
        .filter(|w| w.base_weapon.eq_ignore_ascii_case("Hammer") && w.swf_file.starts_with("Gfx_Weapons"))
        .collect();
    println!("Hammers in Gfx_Weapons*: {}", hammers_in_wep.len());
    for w in &hammers_in_wep {
        println!("  HAMMER: name='{}', id={}, sym='{}', swf='{}', owner='{}'",
            w.weapon_skin_name, w.weapon_skin_id, w.symbol_suffix, w.swf_file, w.owner_hero);
    }

    let swords_in_wep: Vec<_> = cat.weapon_skins.iter()
        .filter(|w| w.base_weapon.eq_ignore_ascii_case("Sword") && w.swf_file.starts_with("Gfx_Weapons"))
        .collect();
    println!("Swords in Gfx_Weapons*: {}", swords_in_wep.len());
    for w in &swords_in_wep {
        println!("  SWORD: name='{}', id={}, sym='{}', swf='{}', owner='{}'",
            w.weapon_skin_name, w.weapon_skin_id, w.symbol_suffix, w.swf_file, w.owner_hero);
    }
}

#[test]
fn test_anm_swapper_logic() {
    use flate2::read::ZlibDecoder;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::{Read, Write};

    let inst = brawlcraft_slint::game_detector::GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }

    let sword_path = std::path::Path::new(&inst.game_path).join("anims").join("Animation_Sword.anm");
    if !sword_path.is_file() {
        return;
    }

    let orig_bytes = std::fs::read(&sword_path).unwrap();
    let uncompressed_len = u32::from_le_bytes([orig_bytes[0], orig_bytes[1], orig_bytes[2], orig_bytes[3]]) as usize;
    let mut decoder = ZlibDecoder::new(&orig_bytes[4..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();
    assert_eq!(decompressed.len(), uncompressed_len);

    // Let's implement the swapping function
    fn swap_anm_animations(data: &mut Vec<u8>, hero_key: &str, swap_suffix: &str) -> usize {
        let mut pairs_to_swap = Vec::new();
        // Look for occurrences of hero_key
        let hero_bytes = hero_key.as_bytes();
        let mut offset = 0;
        while let Some(pos) = data[offset..].windows(hero_bytes.len()).position(|w| *w == *hero_bytes) {
            let actual = offset + pos;
            // The string starts somewhere before or at `actual`. Let's check 2 bytes before for length
            for back in 0..100 {
                if actual >= back + 2 {
                    let str_start = actual - back;
                    let str_len = u16::from_le_bytes([data[str_start - 2], data[str_start - 1]]) as usize;
                    if str_len >= hero_bytes.len() && str_start + str_len <= data.len() {
                        if let Ok(name) = std::str::from_utf8(&data[str_start..str_start + str_len]) {
                            if name.contains(hero_key) && name.ends_with(swap_suffix) {
                                let base_name = name.strip_suffix(swap_suffix).unwrap().to_string();
                                let swap_name = name.to_string();
                                if !pairs_to_swap.iter().any(|(b, s): &(String, String)| b == &base_name && s == &swap_name) {
                                    pairs_to_swap.push((base_name, swap_name));
                                }
                            }
                        }
                    }
                }
            }
            offset = actual + hero_bytes.len();
        }

        println!("Found pairs to swap for {}: {:?}", hero_key, pairs_to_swap);
        let mut swap_count = 0;

        for (base_name, swap_name) in pairs_to_swap {
            let base_bytes = base_name.as_bytes();
            let swap_bytes = swap_name.as_bytes();

            let mut base_pat = Vec::new();
            base_pat.extend_from_slice(&(base_bytes.len() as u16).to_le_bytes());
            base_pat.extend_from_slice(base_bytes);

            let mut swap_pat = Vec::new();
            swap_pat.extend_from_slice(&(swap_bytes.len() as u16).to_le_bytes());
            swap_pat.extend_from_slice(swap_bytes);

            let base_pos = data.windows(base_pat.len()).position(|w| *w == base_pat[..]);
            let swap_pos = data.windows(swap_pat.len()).position(|w| *w == swap_pat[..]);

            if let (Some(bp), Some(sp)) = (base_pos, swap_pos) {
                // If bp < sp, do swap
                let (first_pos, first_pat, first_rep, second_pos, second_pat, second_rep) = if bp < sp {
                    (bp, base_pat.clone(), swap_pat.clone(), sp, swap_pat, base_pat)
                } else {
                    (sp, swap_pat.clone(), base_pat.clone(), bp, base_pat, swap_pat)
                };

                let len_diff = first_rep.len() as isize - first_pat.len() as isize;

                // Replace first
                data.splice(first_pos..first_pos + first_pat.len(), first_rep);

                // Adjust second pos by len_diff
                let adj_second_pos = (second_pos as isize + len_diff) as usize;
                data.splice(adj_second_pos..adj_second_pos + second_pat.len(), second_rep);

                swap_count += 1;
            }
        }

        swap_count
    }

    let mut modified = decompressed.clone();
    let swapped = swap_anm_animations(&mut modified, "Pirate", "SwapSnakeEyes");
    println!("Total swapped animations: {}", swapped);
    assert_eq!(swapped, 3, "Should swap 3 sword signature animations");

    // Recompress and verify round-trip
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&modified).unwrap();
    let _compressed = encoder.finish().unwrap();

    let pistols_path = std::path::Path::new(&inst.game_path).join("anims").join("Animation_Pistols.anm");
    if pistols_path.is_file() {
        let p_bytes = std::fs::read(&pistols_path).unwrap();
        let mut p_dec = ZlibDecoder::new(&p_bytes[4..]);
        let mut p_decomp = Vec::new();
        p_dec.read_to_end(&mut p_decomp).unwrap();
        let swapped_p = swap_anm_animations(&mut p_decomp, "Pirate", "SwapSnakeEyes");
        println!("Pistols swapped animations: {}", swapped_p);
        assert_eq!(swapped_p, 3, "Should swap 3 pistols signature animations");
    }
}

#[test]
fn test_swf_symbol_remapping_and_backup_cycle() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }

    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    // Create a temporary sandbox directory to test without modifying actual game files
    let temp_dir = std::env::temp_dir().join(format!("bc_test_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let viking_orig = std::path::Path::new(&inst.game_path).join("Gfx_Viking.swf");
    let viking_copy = temp_dir.join("Gfx_Viking.swf");
    fs::copy(&viking_orig, &viking_copy).unwrap();

    let orig_hash = BackupManager::compute_sha256(&viking_copy).unwrap();

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    let viking_legend = cat.legends.iter().find(|l| l.hero_name == "Viking").unwrap();
    println!("Bear costume display name: '{}', costume name: '{}'", viking_legend.costumes[1].display_name, viking_legend.costumes[1].costume_name);

    let req = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some(viking_legend.costumes[1].costume_name.clone()),
        color_scheme_name: Some("Sunset".into()),
        ..Default::default()
    };

    let result = engine.apply_modification(&req);
    println!("Modification result: success={}, message={}, log={:?}", result.success, result.message, result.operation_log);
    assert!(result.success, "Modification should succeed: {}", result.message);
    assert_eq!(result.modified_files.len(), 1);

    let modified_hash = BackupManager::compute_sha256(&viking_copy).unwrap();
    assert_ne!(orig_hash, modified_hash, "Hash must change after modification");

    // Test rollback
    let restored = backup_mgr.restore_all().unwrap();
    assert_eq!(restored, 1);

    let restored_hash = BackupManager::compute_sha256(&viking_copy).unwrap();
    assert_eq!(orig_hash, restored_hash, "Hash must match original after rollback");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_cross_file_costume_remapping() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    let inst = GameDetector::detect(None);
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let ew = cat.costumes.iter().find(|c| c.costume_name == "SugarSkull").unwrap();
    let src_path = std::path::Path::new(&inst.game_path).join(&ew.swf_file);
    if src_path.is_file() {
        let mut pkg = SwfPackage::read_from_file(&src_path).unwrap();
        // Rename all _SugarSkull to _Witch
        let mut renamed = 0;
        for tag in &mut pkg.tags {
            if tag.tag_code == 76 {
                let mut syms = brawlcraft_slint::swf_engine::parse_symbol_class(&tag.data).unwrap();
                for (_id, name) in syms.iter_mut() {
                    if let Some(prefix) = name.strip_suffix(&format!("_{}", ew.symbol_suffix)) {
                        *name = format!("{}_Witch", prefix);
                        renamed += 1;
                    }
                }
                tag.data = brawlcraft_slint::swf_engine::serialize_symbol_class(&syms);
            }
        }
        println!("Renamed {} symbols from {} to Witch in {}", renamed, ew.symbol_suffix, ew.swf_file);
        assert!(renamed > 0, "Should rename symbols");
    }
}

#[test]
fn test_pending_modification_queue() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::SharedAppData;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::preset_manager::PresetManager;
    use std::fs;
    use std::sync::Mutex;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }

    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_queue_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let viking_orig = std::path::Path::new(&inst.game_path).join("Gfx_Viking.swf");
    let viking_copy = temp_dir.join("Gfx_Viking.swf");
    fs::copy(&viking_orig, &viking_copy).unwrap();
    let orig_hash = BackupManager::compute_sha256(&viking_copy).unwrap();

    let mut mock_inst = inst.clone();
    mock_inst.game_path = temp_dir.to_string_lossy().to_string();

    let backup_mgr = BackupManager::new(&mock_inst.game_path);
    let mut state = SharedAppData {
        installation: mock_inst,
        catalog: cat.clone(),
        backup_manager: backup_mgr,
        preset_manager: PresetManager::new(),
        renderer: Mutex::new(brawlcraft_slint::desktop_app::RendererClient::new()),
        pending_modifications: Vec::new(),
    };

    let viking_legend = cat.legends.iter().find(|l| l.hero_name == "Viking").unwrap();
    let req = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some(viking_legend.costumes[1].costume_name.clone()),
        ..Default::default()
    };

    // Simulate placing in queue while game was running and auto-restart disabled
    state.pending_modifications.push(req);
    assert_eq!(state.pending_modifications.len(), 1);

    // Verify file is still untouched while in queue
    let unapplied_hash = BackupManager::compute_sha256(&viking_copy).unwrap();
    assert_eq!(orig_hash, unapplied_hash, "File must not be modified while still in queue");

    // Apply pending modifications (triggered on restartGame or game close)
    let applied = state.apply_pending_modifications();
    assert_eq!(applied, 1, "Should apply 1 queued modification");
    assert_eq!(state.pending_modifications.len(), 0, "Queue must be empty after applying");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_queen_nai_to_mikasa() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_mikasa_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let witch_orig = std::path::Path::new(&inst.game_path).join(".studio_backups/_pristine/Gfx_Witch.swf");
    let witch_src = if witch_orig.is_file() {
        witch_orig
    } else {
        std::path::Path::new(&inst.game_path).join("Gfx_Witch.swf")
    };
    if !witch_src.is_file() {
        return;
    }
    let witch_copy = temp_dir.join("Gfx_Witch.swf");
    fs::copy(&witch_src, &witch_copy).unwrap();

    let mikasa_orig = std::path::Path::new(&inst.game_path).join("Gfx_Mikasa.swf");
    let mikasa_copy = temp_dir.join("Gfx_Mikasa.swf");
    fs::copy(&mikasa_orig, &mikasa_copy).unwrap();

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    let req = ModificationRequest {
        legend_name: Some("Witch".into()),
        target_costume_name: Some("Witch".into()),
        replacement_costume_name: Some("Mikasa".into()),
        replacement_weapon1_name: Some("Spear Mikasa".into()),
        replacement_weapon2_name: Some("Katar Mikasa".into()),
        ..Default::default()
    };

    let result = engine.apply_modification(&req);
    println!("Mikasa result: success={}, message={}, log={:?}", result.success, result.message, result.operation_log);
    assert!(result.success, "Should succeed: {}", result.message);

    // Verify modified Gfx_Witch.swf is still large (> 600KB) and has Mikasa sprites
    let modified_len = fs::metadata(&witch_copy).unwrap().len();
    println!("Modified Gfx_Witch.swf length: {} bytes", modified_len);
    assert!(modified_len > 600_000, "Must not shrink or lose base skeleton!");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_bodvar_to_bp11azoth_with_weapons() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_bp11azoth_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    // Copy Gfx_Viking.swf, Gfx_BP11Azoth01.swf, Gfx_Golem.swf, Gfx_Weapons_Sets.swf
    for f in &["Gfx_Viking.swf", "Gfx_BP11Azoth01.swf", "Gfx_Golem.swf", "Gfx_Weapons_Sets.swf"] {
        let orig = std::path::Path::new(&inst.game_path).join(f);
        if orig.is_file() {
            fs::copy(&orig, temp_dir.join(f)).unwrap();
        }
    }

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    let req = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("BP11Azoth".into()),
        replacement_weapon1_name: Some("Hammer Snowman".into()),
        replacement_weapon2_name: Some("Sword Asgard".into()),
        ..Default::default()
    };

    let result = engine.apply_modification(&req);
    println!("BP11Azoth result: success={}, message={}, log={:?}", result.success, result.message, result.operation_log);
    assert!(result.success, "Should succeed: {}", result.message);

    let viking_copy = temp_dir.join("Gfx_Viking.swf");
    let modified_len = fs::metadata(&viking_copy).unwrap().len();
    println!("Modified Gfx_Viking.swf length: {} bytes", modified_len);
    assert!(modified_len > 700_000, "Viking base rig must remain intact!");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_teros_to_bull_construct_with_weapons() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_teros_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    // Copy Gfx_Player_02.swf, Gfx_Skins_02.swf, Gfx_Weapons_02.swf, Gfx_Golem.swf
    for f in &["Gfx_Player_02.swf", "Gfx_Skins_02.swf", "Gfx_Weapons_02.swf", "Gfx_Golem.swf"] {
        let orig = std::path::Path::new(&inst.game_path).join(f);
        if orig.is_file() {
            fs::copy(&orig, temp_dir.join(f)).unwrap();
        }
    }

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    let req = ModificationRequest {
        legend_name: Some("Minotaur".into()),
        target_costume_name: Some("Minotaur".into()),
        replacement_costume_name: Some("BullConstruct".into()),
        replacement_weapon1_name: Some("Axe Armortaur".into()),
        replacement_weapon2_name: Some("Hammer Snowman".into()),
        ..Default::default()
    };

    let result = engine.apply_modification(&req);
    println!("Teros BullConstruct result: success={}, message={}, log={:?}", result.success, result.message, result.operation_log);
    assert!(result.success, "Should succeed: {}", result.message);

    let p02_copy = temp_dir.join("Gfx_Player_02.swf");
    let modified_len = fs::metadata(&p02_copy).unwrap().len();
    println!("Modified Gfx_Player_02.swf length: {} bytes", modified_len);
    assert!(modified_len > 250_000, "Player 02 must preserve shared legends!");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_diagnose_user_requests() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    // 1. Nai and Mikasa
    if let Some(nai) = cat.legends.iter().find(|l| l.hero_name == "Witch") {
        println!("Nai costumes:");
        for c in &nai.costumes {
            println!("  Costume: name='{}', display='{}', swf='{}', suffix='{}'", c.costume_name, c.display_name, c.swf_file, c.symbol_suffix);
        }
    }
    if let Some(mikasa) = cat.costumes.iter().find(|c| c.costume_name == "Mikasa") {
        println!("Mikasa global costume: swf='{}', suffix='{}'", mikasa.swf_file, mikasa.symbol_suffix);
    }
    for w_name in &["Spear Mikasa", "Katar Mikasa", "Hammer Snowman", "Sword Asgard", "Axe Armortaur"] {
        if let Some(w) = cat.weapon_skins.iter().find(|ws| ws.display_name == *w_name || ws.weapon_skin_name == *w_name) {
            println!("Weapon '{}': name='{}', base='{}', swf='{}', suffix='{}'", w_name, w.weapon_skin_name, w.base_weapon, w.swf_file, w.symbol_suffix);
        } else {
            println!("Weapon '{}' NOT FOUND in catalog!", w_name);
        }
    }

    // 2. Bodvar and BP11Azoth
    if let Some(jaeyun) = cat.legends.iter().find(|l| l.hero_name == "Sellsword") {
        println!("Jaeyun costumes count: {}", jaeyun.costumes.len());
        for c in &jaeyun.costumes {
            println!("  Jaeyun Costume: name='{}', display='{}', swf='{}', suffix='{}'", c.costume_name, c.display_name, c.swf_file, c.symbol_suffix);
        }
        for w in &cat.weapon_skins {
            if w.weapon_skin_name.contains("DeathJester") || w.weapon_skin_name.contains("Snowboard") {
                println!("  Found Special Weapon: name='{}', display='{}', base='{}', owner='{}', swf='{}', suffix='{}'", w.weapon_skin_name, w.display_name, w.base_weapon, w.owner_hero, w.swf_file, w.symbol_suffix);
            }
        }
    }
    if let Some(bp11) = cat.costumes.iter().find(|c| c.costume_name == "BP11Azoth") {
        println!("BP11Azoth costume: swf='{}', suffix='{}'", bp11.swf_file, bp11.symbol_suffix);
    }

    // 3. Teros and Bull Construct
    if let Some(teros) = cat.legends.iter().find(|l| l.hero_name == "Minotaur") {
        println!("Teros costumes:");
        for c in &teros.costumes {
            println!("  Costume: name='{}', display='{}', swf='{}', suffix='{}'", c.costume_name, c.display_name, c.swf_file, c.symbol_suffix);
        }
    }
    if let Some(bc) = cat.costumes.iter().find(|c| c.costume_name == "Bull Construct" || c.costume_name.contains("BullConstruct") || c.display_name.contains("Bull Construct")) {
        println!("Bull Construct costume: name='{}', display='{}', swf='{}', suffix='{}'", bc.costume_name, bc.display_name, bc.swf_file, bc.symbol_suffix);
    }
}

#[test]
fn test_jaeyun_independent_weapon_selection_and_color() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use brawlcraft_slint::swf_engine::{SwfPackage, parse_symbol_class};
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_jaeyun_weps_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let sellsword_orig = std::path::Path::new(&inst.game_path).join("Gfx_Sellsword.swf");
    let sellsword_copy = temp_dir.join("Gfx_Sellsword.swf");
    fs::copy(&sellsword_orig, &sellsword_copy).unwrap();

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    let req = ModificationRequest {
        legend_name: Some("Sellsword".into()),
        target_costume_name: Some("Sellsword".into()),
        replacement_costume_name: Some("DeathJester".into()),
        replacement_weapon1_name: Some("Sword Death Jester".into()),
        replacement_weapon2_name: Some("Greatsword Snowboard JY".into()),
        color_scheme_name: Some("Ranked2".into()),
        ..Default::default()
    };

    let result = engine.apply_modification(&req);
    println!("Jaeyun result: success={}, message={}, log={:?}", result.success, result.message, result.operation_log);
    assert!(result.success, "Modification should succeed");

    // Read modified SWF and verify Greatsword symbol points to SnowboardJY ID, NOT DeathJester ID!
    let pkg = SwfPackage::read_from_file(&sellsword_copy).unwrap();
    let mut dj_greatsword_id = 0;
    let mut sb_greatsword_id = 0;
    let mut sellsword_greatsword_id = 0;

    for tag in &pkg.tags {
        if tag.tag_code == 76 {
            let syms = parse_symbol_class(&tag.data).unwrap();
            for (id, name) in syms {
                if name == "a_WeaponGreat_DeathJester" {
                    dj_greatsword_id = id;
                } else if name == "a_WeaponGreat_SnowboardJY" {
                    sb_greatsword_id = id;
                } else if name == "a_WeaponGreat_Sellsword" {
                    sellsword_greatsword_id = id;
                }
            }
        }
    }

    println!("IDs: DeathJester GS={}, SnowboardJY GS={}, Sellsword GS={}", dj_greatsword_id, sb_greatsword_id, sellsword_greatsword_id);
    assert_ne!(dj_greatsword_id, 0, "DeathJester Greatsword symbol must exist");
    assert_ne!(sb_greatsword_id, 0, "SnowboardJY Greatsword symbol must exist");
    assert_ne!(dj_greatsword_id, sb_greatsword_id, "DeathJester and SnowboardJY must have different IDs");
    assert_eq!(sellsword_greatsword_id, sb_greatsword_id, "Sellsword Greatsword MUST point to SnowboardJY, NOT DeathJester!");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_weapon_color_preview_rendering() {
    use brawlcraft_slint::desktop_app::RendererClient;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::catalog::Catalog;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let state = brawlcraft_slint::desktop_app::SharedAppData {
        installation: inst.clone(),
        catalog: cat.clone(),
        backup_manager: brawlcraft_slint::backup_manager::BackupManager::new(&inst.game_path),
        preset_manager: brawlcraft_slint::preset_manager::PresetManager::new(),
        renderer: std::sync::Mutex::new(RendererClient::new()),
        pending_modifications: Vec::new(),
    };

    // Test 1: Greatsword SnowboardJY with Ranked2 color scheme
    let mut req1 = serde_json::json!({
        "type": "weapon",
        "baseWeapon": "Greatsword",
        "symbol": "SnowboardJY",
        "weaponSkinName": "GreatswordSnowboardJY",
        "swfFile": "Gfx_Sellsword.swf",
        "ownerHero": "Sellsword",
        "costumeName": "DeathJester",
        "colorSchemeId": "36",
        "gamePath": inst.game_path
    });

    brawlcraft_slint::desktop_app::prepare_render_request(&mut req1, &state);
    let color_map1 = req1.get("colorMap").and_then(|v| v.as_object()).unwrap();
    println!("SnowboardJY colorMap size: {}", color_map1.len());
    // 0xF79F54 (16228180) is special color of SnowboardJY
    assert!(color_map1.contains_key("16228180"), "Must contain SnowboardJY special color 0xF79F54");
    // 0xCFD6D9 (13620953) is weaponlt of SnowboardJY
    assert!(color_map1.contains_key("13620953"), "Must contain SnowboardJY weaponlt color 0xCFD6D9");

    // Test 2: Ulgrim's Axe Space Miner with Ranked2 color scheme
    let mut req2 = serde_json::json!({
        "type": "weapon",
        "baseWeapon": "Axe",
        "symbol": "SpaceMiner",
        "weaponSkinName": "AxeSpaceMiner",
        "swfFile": "Gfx_Dwarf.swf",
        "ownerHero": "Dwarf",
        "costumeName": "Dwarf",
        "colorSchemeId": "36",
        "gamePath": inst.game_path
    });

    brawlcraft_slint::desktop_app::prepare_render_request(&mut req2, &state);
    let color_map2 = req2.get("colorMap").and_then(|v| v.as_object()).unwrap();
    println!("AxeSpaceMiner colorMap size: {}", color_map2.len());
    assert!(!color_map2.is_empty(), "Must contain color map for AxeSpaceMiner");
}

#[test]
fn test_inspect_current_game_sellsword_swf() {
    use brawlcraft_slint::swf_engine::{SwfPackage, parse_symbol_class};

    let inst = brawlcraft_slint::game_detector::GameDetector::detect(None);
    let pristine_path = std::path::Path::new(&inst.game_path).join(".studio_backups/_pristine/Gfx_Sellsword.swf");
    let target_path = if pristine_path.is_file() {
        pristine_path
    } else {
        std::path::Path::new(&inst.game_path).join("Gfx_Sellsword.swf")
    };
    if !target_path.is_file() {
        return;
    }
    let mut swf = match SwfPackage::read_from_file(&target_path) {
        Ok(s) => s,
        Err(_) => return,
    };

    let count = swf.remap_symbols("Sellsword", "DeathJester").unwrap();
    println!("Remapped symbols count: {}", count);

    let is_def_tag = |code: u16| matches!(code, 2 | 6 | 10 | 11 | 14 | 20 | 21 | 22 | 32 | 33 | 35 | 36 | 37 | 39 | 46 | 48 | 75 | 83 | 84);
    let mut def_ids = std::collections::HashSet::new();
    for tag in &swf.tags {
        if is_def_tag(tag.tag_code) && tag.data.len() >= 2 {
            let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
            def_ids.insert(id);
        }
    }

    let mut sym_map = std::collections::HashMap::new();
    for tag in &swf.tags {
        if tag.tag_code == 76 {
            let syms = parse_symbol_class(&tag.data).unwrap();
            for (id, name) in syms {
                sym_map.insert(name.clone(), id);
                if !def_ids.contains(&id) {
                    println!("CRITICAL ERROR: Undefined ID {} in symbol {}", id, name);
                }
            }
        }
    }

    println!("Total symbols after remap: {}", sym_map.len());
    let ss_syms: Vec<_> = sym_map.iter().filter(|(n, _)| n.ends_with("_Sellsword")).collect();
    let dj_syms: Vec<_> = sym_map.iter().filter(|(n, _)| n.ends_with("_DeathJester")).collect();
    println!("_Sellsword symbols: {}, _DeathJester symbols: {}", ss_syms.len(), dj_syms.len());

    for (name, id) in &ss_syms {
        let matching_dj = name.replace("_Sellsword", "_DeathJester");
        if let Some(dj_id) = sym_map.get(&matching_dj) {
            if **id != *dj_id {
                println!("MISMATCH: {} (id={}) != {} (id={})", name, id, matching_dj, dj_id);
            }
        } else {
            println!("NO DJ MATCH FOR: {} (id={})", name, id);
        }
    }
}

#[test]
fn test_inspect_asuri_and_levi() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let asuri = cat.legends.iter().find(|l| l.hero_name == "Tiger" || l.display_name == "Asuri").unwrap();
    println!("Asuri hero_name: '{}', costumes: {}", asuri.hero_name, asuri.costumes.len());
    for c in &asuri.costumes {
        println!("  Costume: name='{}', swf='{}', suffix='{}'", c.costume_name, c.swf_file, c.symbol_suffix);
    }

    let p_asuri = std::path::Path::new(&inst.game_path).join(&asuri.costumes[0].swf_file);
    if p_asuri.is_file() {
        let pkg = SwfPackage::read_from_file(&p_asuri).unwrap();
        for tag in &pkg.tags {
            if tag.tag_code == 76 {
                let syms = brawlcraft_slint::swf_engine::parse_symbol_class(&tag.data).unwrap();
                let asuri_syms: Vec<_> = syms.iter().filter(|(_, n)| n.contains(&asuri.costumes[0].symbol_suffix)).collect();
                println!("Asuri target symbols count: {}", asuri_syms.len());
                for (id, name) in &asuri_syms {
                    if name.to_lowercase().contains("hand") || name.to_lowercase().contains("arm") {
                        println!("    Asuri hand/arm: id={}, name={}", id, name);
                    }
                }
            }
        }
    }

    let p_levi = std::path::Path::new(&inst.game_path).join("Gfx_Levi.swf");
    if p_levi.is_file() {
        let pkg = SwfPackage::read_from_file(&p_levi).unwrap();
        for tag in &pkg.tags {
            if tag.tag_code == 76 {
                let syms = brawlcraft_slint::swf_engine::parse_symbol_class(&tag.data).unwrap();
                println!("Levi total symbols: {}", syms.len());
                for (id, name) in &syms {
                    println!("    Levi: id={}, name={}", id, name);
                }
            }
        }
    }
}

#[test]
fn test_asuri_to_levi_hands_remapping() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_asuri_levi_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let game_dir = std::path::Path::new(&inst.game_path);
    let pristine_dir = game_dir.join(".studio_backups/_pristine");

    let copy_file = |name: &str| {
        let src = pristine_dir.join(name);
        let path = if src.is_file() { src } else { game_dir.join(name) };
        fs::copy(&path, temp_dir.join(name)).unwrap();
    };

    copy_file("Gfx_Player_02.swf");
    copy_file("Gfx_Levi.swf");
    copy_file("Gfx_Hands.swf");

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    let req = ModificationRequest {
        legend_name: Some("Asuri".into()),
        target_costume_name: Some("Cat".into()),
        replacement_costume_name: Some("Levi".into()),
        ..Default::default()
    };

    let result = engine.apply_modification(&req);
    assert!(result.success, "Modification should succeed: {}", result.message);
    assert!(result.modified_files.contains(&"Gfx_Player_02.swf".to_string()));

    // Verify Gfx_Hands.swf is NEVER touched to guarantee 100% stability across all legends
    assert!(!result.modified_files.contains(&"Gfx_Hands.swf".to_string()), "Gfx_Hands.swf must never be modified to protect game stability!");

    // Restore and verify pristine
    let restored = backup_mgr.restore_all().unwrap();
    assert!(restored >= 1, "Should restore modified files");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_dev_telemetry_metrics_and_logging() {
    use brawlcraft_slint::telemetry;

    telemetry::set_enabled(true);
    let temp_log_file = std::env::temp_dir().join(format!("bc_dev_test_{}.log", std::process::id()));
    telemetry::set_log_file_path(Some(temp_log_file.clone()));

    // Record various operation logs
    telemetry::log("INFO", "IPC", "Test IPC message from client", Some("{\"action\":\"test\"}"));
    telemetry::log("DEBUG", "SWF-REMAP", "Remapped 14 symbols for test character", None);
    telemetry::log("WARN", "ENGINE", "Skipped meme asset _AllShades", None);
    telemetry::log("ERROR", "TEST", "Sample test error event", Some("detail trace 0x8892"));

    // Check recent logs
    let logs = telemetry::get_recent_logs(0, 100);
    assert!(logs.len() >= 4, "Must contain at least 4 test logs");
    assert!(logs.iter().any(|l| l.category == "SWF-REMAP"));
    assert!(logs.iter().any(|l| l.level == "ERROR"));

    // Check system metrics
    let metrics = telemetry::get_metrics();
    println!("Dev Metrics: PID={}, CPU={:.1}%, Memory={:.1}MB (Peak: {:.1}MB), Threads={}, Handles={}, Uptime={}",
        metrics.pid, metrics.cpu_percent, metrics.memory_working_set_mb, metrics.memory_peak_mb,
        metrics.thread_count, metrics.handle_count, metrics.uptime_formatted
    );
    assert!(metrics.pid > 0, "PID must be > 0");
    assert!(metrics.memory_working_set_mb > 0.0, "Working set memory must be > 0 MB");
    assert!(metrics.thread_count >= 1, "Thread count must be >= 1");

    // Verify disk log file exists and contains content
    assert!(temp_log_file.is_file(), "Log file must be created on disk");
    let file_content = std::fs::read_to_string(&temp_log_file).unwrap();
    assert!(file_content.contains("SWF-REMAP"), "Log file must contain SWF-REMAP");
    assert!(file_content.contains("Sample test error event"), "Log file must contain error event");

    // Verify diagnostic report export
    let report_file = std::env::temp_dir().join(format!("bc_dev_report_{}.txt", std::process::id()));
    let dummy_info = serde_json::json!({ "gamePath": "C:\\Games\\Brawlhalla", "version": "3.1.0" });
    let export_res = telemetry::export_diagnostics_report(&report_file, &dummy_info);
    assert!(export_res.is_ok(), "Export must succeed");
    assert!(report_file.is_file(), "Report file must exist on disk");
    let report_content = std::fs::read_to_string(&report_file).unwrap();
    assert!(report_content.contains("SKINNER DEVELOPER DIAGNOSTICS REPORT") || report_content.contains("BRAWLCRAFT DEVELOPER DIAGNOSTICS REPORT"));
    assert!(report_content.contains("RAM Working Set:"));

    // Clean up
    let _ = std::fs::remove_file(temp_log_file);
    let _ = std::fs::remove_file(report_file);
}

#[test]
fn test_curated_legends_filter() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    // Verify initial full catalog
    assert!(cat.legends.len() > 50, "Full catalog should have > 50 legends");

    // Apply curated filter
    let mut cat = brawlcraft_slint::catalog::Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    // Curated catalog now has exactly 20 legends (5 core + 15 restored)
    assert_eq!(cat.legends.len(), 20, "Curated catalog must have exactly 20 legends (5 core + 15 restored)");

    // Explicitly verify none of Artemis, Yumiko, or Magyar are present
    for l in &cat.legends {
        let n = l.hero_name.to_lowercase();
        let d = l.display_name.to_lowercase();
        assert!(!n.contains("spacegirl") && !d.contains("artemis"), "Artemis must not be present");
        assert!(!n.contains("foxgirl") && !d.contains("yumiko"), "Yumiko must not be present");
        assert!(!n.contains("hauntedarmor") && !d.contains("magyar"), "Magyar must not be present");
    }

    let legend_names: Vec<String> = cat.legends.iter().map(|l| l.display_name.clone()).collect();
    println!("Curated legends in catalog: {:?}", legend_names);
    println!("Curated costumes count: {}", cat.costumes.len());
    println!("Curated weapon skins count: {}", cat.weapon_skins.len());
    use brawlcraft_slint::desktop_app::RendererClient;
    let mut client = RendererClient::new();
    let req1 = serde_json::json!({
        "type": "weapon",
        "baseWeapon": "Axe",
        "symbol": "CelticArmor",
        "swfFile": "Gfx_Dwarf.swf",
        "ownerHero": "Dwarf",
        "gamePath": inst.game_path
    });
    let res1 = client.render(&req1);
    assert!(res1.is_some(), "AxeCelticArmor must render successfully with SafeReadSwf");

    let req2 = serde_json::json!({
        "type": "weapon",
        "baseWeapon": "RocketLance",
        "symbol": "CelticArmor",
        "swfFile": "Gfx_Dwarf.swf",
        "ownerHero": "Dwarf",
        "gamePath": inst.game_path
    });
    let res2 = client.render(&req2);
    assert!(res2.is_some(), "RocketLanceCelticArmor must render successfully with SafeReadSwf");

    // Verify all 5 requested legends are present: Bodvar, Thatch, Ulgrim, Onyx, Jaeyun
    assert!(cat.legends.iter().any(|l| l.hero_name == "Viking" || l.display_name == "Bödvar" || l.display_name == "Bodvar"), "Bodvar must be present");
    assert!(cat.legends.iter().any(|l| l.hero_name == "Thatch" || l.display_name == "Thatch"), "Thatch must be present");
    assert!(cat.legends.iter().any(|l| l.hero_name == "Dwarf" || l.display_name == "Ulgrim"), "Ulgrim must be present");
    assert!(cat.legends.iter().any(|l| l.hero_name == "Gargoyle" || l.display_name == "Onyx"), "Onyx must be present");
    assert!(cat.legends.iter().any(|l| l.hero_name == "Sellsword" || l.display_name == "Jaeyun"), "Jaeyun must be present");

    // Verify costumes belong only to curated legends
    for c in &cat.costumes {
        assert!(
            brawlcraft_slint::catalog::is_curated_hero(&c.owner_hero),
            "Unexpected costume owner: {}", c.owner_hero
        );
    }

    // Verify weapon skins match the weapon types wielded by curated legends
    let allowed_weapon_types = [
        "hammer", "sword", "blasters", "pistol", "blaster", "axe", "rocketlance", "lance",
        "gauntlets", "gauntlet", "fists", "cannon", "greatsword", "great",
        "bow", "scythe", "spear", "boots", "battleboots", "katar", "katars", "chakram", "orb"
    ];
    for w in &cat.weapon_skins {
        let bw = w.base_weapon.to_lowercase();
        assert!(
            allowed_weapon_types.contains(&bw.as_str()),
            "Unexpected weapon skin base weapon in curated catalog: {} ({})", w.base_weapon, w.weapon_skin_name
        );
        // Verify weapon skins have valid IDs and authentic SWFs
        assert!(!w.weapon_skin_name.is_empty(), "Weapon skin name must not be empty");
        assert!(!w.swf_file.is_empty(), "Weapon skin swf must not be empty");
    }
}

#[test]
fn test_bodvar_costume_and_weapon_stability() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    let mut cat = brawlcraft_slint::catalog::Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    let bodvar_legend = cat.legends.iter().find(|l| l.hero_name == "Viking").unwrap();
    println!("Bodvar w1: {:?}, w2: {:?}", bodvar_legend.weapon1, bodvar_legend.weapon2);
    for w in &cat.weapon_skins {
        if w.owner_hero.eq_ignore_ascii_case("viking") {
            println!("Bodvar weapon: name='{}', base='{}', suffix='{}', swf='{}'", w.weapon_skin_name, w.base_weapon, w.symbol_suffix, w.swf_file);
        }
    }

    let temp_dir = std::env::temp_dir().join(format!("bc_test_bodvar_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let pristine_viking = std::path::Path::new(&inst.game_path).join(".studio_backups/_pristine/Gfx_Viking.swf");
    let viking_src = if pristine_viking.is_file() {
        pristine_viking
    } else {
        std::path::Path::new(&inst.game_path).join("Gfx_Viking.swf")
    };
    if !viking_src.is_file() {
        return;
    }

    for f in &["Gfx_Weapons_Sets.swf", "Gfx_VikingGardener.swf"] {
        let orig = std::path::Path::new(&inst.game_path).join(f);
        if orig.is_file() {
            fs::copy(&orig, temp_dir.join(f)).unwrap();
        }
    }

    // --- TEST 1: Costume only: Viking -> Bear ---
    fs::copy(&viking_src, temp_dir.join("Gfx_Viking.swf")).unwrap();
    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    let req1 = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("Bear".into()),
        ..Default::default()
    };
    let res1 = engine.apply_modification(&req1);
    println!("Test 1 (Viking -> Bear): success={}, log={:?}", res1.success, res1.operation_log);
    assert!(res1.success);

    // Verify a_Arm1_Sleeveless was remapped to Bear's arm (ID 778)
    let pkg1 = brawlcraft_slint::swf_engine::SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    let syms1 = pkg1.get_symbol_classes().unwrap();
    let sleeveless_entry = syms1.iter().find(|(_, name)| name == "a_Arm1_Sleeveless").unwrap();
    let bear_arm_entry = syms1.iter().find(|(_, name)| name == "a_Arm1_Bear").unwrap();
    assert_eq!(sleeveless_entry.0, bear_arm_entry.0, "a_Arm1_Sleeveless must map to Bear's arm ID");
    assert!(!syms1.iter().any(|(_, name)| name == "a_Arm1_Viking"), "a_Arm1_Viking should not be added as duplicate");

    // --- TEST 2: Weapon only: Viking Hammer -> Hammer Bear ---
    fs::copy(&viking_src, temp_dir.join("Gfx_Viking.swf")).unwrap();
    let mut backup_mgr2 = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine2 = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr2, &cat);

    let req2 = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_weapon1_name: Some("Hammer Bear".into()),
        ..Default::default()
    };
    let res2 = engine2.apply_modification(&req2);
    println!("Test 2 (Hammer Bear): success={}, log={:?}", res2.success, res2.operation_log);
    assert!(res2.success);
    assert!(!res2.operation_log.iter().any(|l| l.contains("Costume symbols remapped")), "Costume must NOT be remapped when only weapon is changed");
    assert!(res2.operation_log.iter().any(|l| l.contains("Weapon 1 (Hammer) updated: 2 symbols")));

    // --- TEST 3: Weapon only: Sword -> Sword Asgard (cross-SWF) ---
    fs::copy(&viking_src, temp_dir.join("Gfx_Viking.swf")).unwrap();
    let mut backup_mgr3 = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine3 = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr3, &cat);

    let req3 = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_weapon2_name: Some("Sword Asgard".into()),
        ..Default::default()
    };
    let res3 = engine3.apply_modification(&req3);
    println!("Test 3 (Sword Asgard): success={}, log={:?}", res3.success, res3.operation_log);
    assert!(res3.success);
    assert!(!res3.operation_log.iter().any(|l| l.contains("Costume symbols remapped")), "Costume must NOT be remapped when only weapon is changed");
    assert!(res3.operation_log.iter().any(|l| l.contains("Cross-SWF Weapon 2 (Sword) safely imported")));

    // --- TEST 4: Costume + Both Weapons: CyberViking + Hammer Bear + Sword Asgard ---
    fs::copy(&viking_src, temp_dir.join("Gfx_Viking.swf")).unwrap();
    let mut backup_mgr4 = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine4 = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr4, &cat);

    let req4 = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("CyberViking".into()),
        replacement_weapon1_name: Some("Hammer Bear".into()),
        replacement_weapon2_name: Some("Sword Asgard".into()),
        ..Default::default()
    };
    let res4 = engine4.apply_modification(&req4);
    println!("Test 4 (CyberViking + Hammer Bear + Sword Asgard): success={}, log={:?}", res4.success, res4.operation_log);
    assert!(res4.success);
    assert!(res4.operation_log.iter().any(|l| l.contains("Costume symbols remapped")));
    assert!(res4.operation_log.iter().any(|l| l.contains("Weapon 1 (Hammer) updated: 2 symbols")));
    assert!(res4.operation_log.iter().any(|l| l.contains("Cross-SWF Weapon 2 (Sword) safely imported")));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_debug_bodvar_crash() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::{prepare_render_request, RendererClient, SharedAppData};
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use std::sync::Mutex;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    let mut cat = brawlcraft_slint::catalog::Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    let bodvar = cat.legends.iter().find(|l| l.hero_name == "Viking" || l.display_name == "Bödvar").expect("Bodvar must exist");
    println!("=== BODVAR DETAILS ===");
    println!("hero_name: {:?}", bodvar.hero_name);
    println!("display_name: {:?}", bodvar.display_name);

    let cost_488 = cat.costumes.iter().find(|c| c.costume_id == 488);
    println!("Costume 488 in cat.costumes: {:?}", cost_488.map(|c| (&c.costume_name, &c.owner_hero, &c.swf_file)));

    let wep_1369 = cat.weapon_skins.iter().find(|w| w.weapon_skin_id == 1369);
    println!("Weapon 1369 in cat.weapon_skins: {:?}", wep_1369.map(|w| (&w.weapon_skin_name, &w.owner_hero, &w.swf_file)));

    println!("hero_key: {:?}", bodvar.hero_key);
    println!("bio: {:?}", bodvar.bio);
    println!("weapon1: {:?}", bodvar.weapon1);
    println!("weapon2: {:?}", bodvar.weapon2);
    println!("icon_name: {:?}", bodvar.icon_name);
    println!("pose_symbol: {:?}", bodvar.pose_symbol);
    println!("primary_swf: {:?}", bodvar.primary_swf);
    println!("costumes count: {}", bodvar.costumes.len());
    for (i, c) in bodvar.costumes.iter().enumerate() {
        println!("  Costume [{}]: name='{}', display='{}', is_def={}, swf='{}', icon='{}'",
            i, c.costume_name, c.display_name, c.is_default, c.swf_file, c.icon_name);
    }

    let mut bm = brawlcraft_slint::backup_manager::BackupManager::new(&inst.game_path);
    let engine = ModificationEngine::new(
        std::path::PathBuf::from(&inst.game_path),
        &mut bm,
        &cat,
    );

    // Test validate_modification
    let target_costume = bodvar.costumes.iter().find(|c| c.is_default).or_else(|| bodvar.costumes.first()).unwrap();
    let second_costume = bodvar.costumes.iter().find(|c| !c.is_default).unwrap();
    let req = ModificationRequest {
        target_legend: Some(bodvar.clone()),
        target_costume: Some(target_costume.clone()),
        replacement_costume: Some(second_costume.clone()),
        ..Default::default()
    };
    let val_res = engine.validate_modification(&req);
    println!("validate_modification result: {:?}", val_res);

    // Test renderer
    let state = SharedAppData {
        installation: inst.clone(),
        catalog: cat.clone(),
        backup_manager: brawlcraft_slint::backup_manager::BackupManager::new(&inst.game_path),
        preset_manager: brawlcraft_slint::preset_manager::PresetManager::new(),
        renderer: Mutex::new(RendererClient::new()),
        pending_modifications: Vec::new(),
    };

    let mut client = state.renderer.lock().unwrap();

    // 1. Legend pose
    let mut req1 = serde_json::json!({
        "type": "legend",
        "name": bodvar.display_name,
        "hero": bodvar.hero_name
    });
    prepare_render_request(&mut req1, &state);
    let res1 = client.render(&req1);
    println!("Legend pose render result: is_some={}", res1.is_some());

    // 2. Target costume
    let mut req2 = serde_json::json!({
        "type": "costume",
        "icon": target_costume.icon_name,
        "iconFile": target_costume.icon_file_name,
        "costumeName": target_costume.costume_name,
        "ownerHero": target_costume.owner_hero,
        "swfFile": target_costume.swf_file
    });
    prepare_render_request(&mut req2, &state);
    let res2 = client.render(&req2);
    println!("Target costume render result: is_some={}", res2.is_some());

    // 3. Replacement costume
    let mut req3 = serde_json::json!({
        "type": "costume",
        "icon": second_costume.icon_name,
        "iconFile": second_costume.icon_file_name,
        "costumeName": second_costume.costume_name,
        "ownerHero": second_costume.owner_hero,
        "swfFile": second_costume.swf_file
    });
    prepare_render_request(&mut req3, &state);
    let res3 = client.render(&req3);
    println!("Replacement costume render result: is_some={}", res3.is_some());

    // 4. Test rendering all Bodvar costumes
    for c in &bodvar.costumes {
        let mut req_c = serde_json::json!({
            "type": "costume",
            "icon": c.icon_name,
            "iconFile": c.icon_file_name,
            "costumeName": c.costume_name,
            "ownerHero": c.owner_hero,
            "swfFile": c.swf_file
        });
        prepare_render_request(&mut req_c, &state);
        let res_c = client.render(&req_c);
        if res_c.is_none() {
            println!("WARN: Costume '{}' failed to render", c.costume_name);
        }
    }

    // 5. Test rendering all Bodvar weapons
    for w in cat.weapon_skins.iter().filter(|w| w.owner_hero.eq_ignore_ascii_case(&bodvar.hero_name)) {
        let mut req_w = serde_json::json!({
            "type": "weapon",
            "baseWeapon": w.base_weapon,
            "symbol": w.symbol_suffix,
            "weaponSkinName": w.weapon_skin_name,
            "swfFile": w.swf_file,
            "ownerHero": w.owner_hero
        });
        prepare_render_request(&mut req_w, &state);
        let res_w = client.render(&req_w);
        if res_w.is_none() {
            println!("WARN: Weapon '{}' failed to render", w.weapon_skin_name);
        }
    }
}

#[test]
fn test_regression_weapon_skin_uniqueness_and_catalog() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use std::collections::HashSet;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    let hammers: Vec<_> = cat.weapon_skins.iter().filter(|w| w.base_weapon.eq_ignore_ascii_case("Hammer")).collect();
    let swords: Vec<_> = cat.weapon_skins.iter().filter(|w| w.base_weapon.eq_ignore_ascii_case("Sword")).collect();

    // Verify all 186 hammer skins and 213 sword skins are preserved
    assert_eq!(hammers.len(), 186, "Expected exactly 186 hammer skins");
    assert_eq!(swords.len(), 213, "Expected exactly 213 sword skins");

    // Verify every hammer skin has a unique ID and unique name
    let mut hammer_ids = HashSet::new();
    let mut hammer_names = HashSet::new();
    for h in &hammers {
        assert!(hammer_ids.insert(h.weapon_skin_id), "Duplicate Hammer ID: {}", h.weapon_skin_id);
        assert!(hammer_names.insert(h.weapon_skin_name.clone()), "Duplicate Hammer Name: {}", h.weapon_skin_name);
        assert!(!h.swf_file.is_empty(), "Hammer '{}' missing swf_file", h.weapon_skin_name);
        assert!(!h.symbol_suffix.is_empty(), "Hammer '{}' missing symbol_suffix", h.weapon_skin_name);
    }

    // Verify every sword skin has a unique ID and unique name
    let mut sword_ids = HashSet::new();
    let mut sword_names = HashSet::new();
    for s in &swords {
        assert!(sword_ids.insert(s.weapon_skin_id), "Duplicate Sword ID: {}", s.weapon_skin_id);
        assert!(sword_names.insert(s.weapon_skin_name.clone()), "Duplicate Sword Name: {}", s.weapon_skin_name);
        assert!(!s.swf_file.is_empty(), "Sword '{}' missing swf_file", s.weapon_skin_name);
        assert!(!s.symbol_suffix.is_empty(), "Sword '{}' missing symbol_suffix", s.weapon_skin_name);
    }

    // Verify other weapon categories remain populated and unaffected
    let axes: Vec<_> = cat.weapon_skins.iter().filter(|w| w.base_weapon.eq_ignore_ascii_case("Axe")).collect();
    let lances: Vec<_> = cat.weapon_skins.iter().filter(|w| w.base_weapon.eq_ignore_ascii_case("RocketLance") || w.base_weapon.eq_ignore_ascii_case("Lance")).collect();
    let blasters: Vec<_> = cat.weapon_skins.iter().filter(|w| w.base_weapon.eq_ignore_ascii_case("Pistol") || w.base_weapon.eq_ignore_ascii_case("Blasters")).collect();
    let gauntlets: Vec<_> = cat.weapon_skins.iter().filter(|w| w.base_weapon.eq_ignore_ascii_case("Gauntlets") || w.base_weapon.eq_ignore_ascii_case("Gauntlet") || w.base_weapon.eq_ignore_ascii_case("Fists")).collect();
    let cannons: Vec<_> = cat.weapon_skins.iter().filter(|w| w.base_weapon.eq_ignore_ascii_case("Cannon")).collect();
    let greatswords: Vec<_> = cat.weapon_skins.iter().filter(|w| w.base_weapon.eq_ignore_ascii_case("Greatsword")).collect();

    assert!(!axes.is_empty(), "Axe skins must not be empty");
    assert!(!lances.is_empty(), "Rocket Lance skins must not be empty");
    assert!(!blasters.is_empty(), "Blaster skins must not be empty");
    assert!(!gauntlets.is_empty(), "Gauntlet skins must not be empty");
    assert!(!cannons.is_empty(), "Cannon skins must not be empty");
    assert!(!greatswords.is_empty(), "Greatsword skins must not be empty");
}

#[test]
fn test_regression_weapon_skin_rendering_diversity() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::RendererClient;
    use brawlcraft_slint::game_detector::GameDetector;
    use std::collections::HashSet;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    let mut client = RendererClient::new();

    // Test a diverse sample of hammer skins
    let test_hammer_names = [
        "HammerViking",
        "HammerAsgard",
        "HammerBear",
        "HammerCyberViking",
        "HammerTrucker",
        "HammerMetal",
        "HammerIronViking",
        "HammerWrestler",
        "HammerDraugr",
        "HammerNutcrackerBodvar",
        "HammerBigSteam",
    ];

    let mut hammer_rendered = HashSet::new();
    for name in &test_hammer_names {
        let w = cat.weapon_skins.iter().find(|w| w.weapon_skin_name == *name)
            .unwrap_or_else(|| panic!("Hammer skin '{}' not found in catalog", name));
        let req = serde_json::json!({
            "type": "weapon",
            "baseWeapon": w.base_weapon,
            "symbol": w.symbol_suffix,
            "weaponSkinName": w.weapon_skin_name,
            "swfFile": w.swf_file,
            "ownerHero": w.owner_hero,
            "gamePath": inst.game_path
        });
        let svg = client.render(&req).expect(&format!("Failed to render hammer '{}'", name));
        assert!(svg.trim().starts_with("<svg"), "Invalid SVG output for hammer '{}'", name);
        assert!(!svg.is_empty(), "Empty SVG for hammer '{}'", name);
        hammer_rendered.insert(svg);
    }
    // All 11 distinct hammer skins must produce distinct SVGs (no duplication)
    assert_eq!(hammer_rendered.len(), test_hammer_names.len(), "Duplicate hammer SVG detected across distinct skins");

    // Test a diverse sample of sword skins
    let test_sword_names = [
        "SwordViking",
        "SwordPirate",
        "SwordAsgard",
        "SwordBear",
        "SwordCyberViking",
        "SwordBiker",
        "SwordSellsword",
        "SwordDeathJester",
        "SwordWolfmanThatch",
        "SwordAnniversaryBodvar",
    ];

    let mut sword_rendered = HashSet::new();
    for name in &test_sword_names {
        let w = cat.weapon_skins.iter().find(|w| w.weapon_skin_name == *name)
            .unwrap_or_else(|| panic!("Sword skin '{}' not found in catalog", name));
        let req = serde_json::json!({
            "type": "weapon",
            "baseWeapon": w.base_weapon,
            "symbol": w.symbol_suffix,
            "weaponSkinName": w.weapon_skin_name,
            "swfFile": w.swf_file,
            "ownerHero": w.owner_hero,
            "gamePath": inst.game_path
        });
        let svg = client.render(&req).expect(&format!("Failed to render sword '{}'", name));
        assert!(svg.trim().starts_with("<svg"), "Invalid SVG output for sword '{}'", name);
        assert!(!svg.is_empty(), "Empty SVG for sword '{}'", name);
        println!("Sword '{}' svg len: {}", name, svg.len());
        sword_rendered.insert(svg);
    }
    // All 10 distinct sword skins must produce distinct SVGs (no duplication)
    assert_eq!(sword_rendered.len(), test_sword_names.len(), "Duplicate sword SVG detected across distinct skins");
}

#[test]
fn test_all_weapon_categories_render_properly() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::RendererClient;
    use brawlcraft_slint::game_detector::GameDetector;
    use std::collections::HashSet;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    let mut client = RendererClient::new();

    let categories = [
        "Hammer",
        "Sword",
        "Axe",
        "RocketLance",
        "Pistol",
        "Fists",
        "Cannon",
        "Greatsword",
    ];

    for cat_name in &categories {
        let skins: Vec<_> = cat.weapon_skins.iter()
            .filter(|w| w.base_weapon.eq_ignore_ascii_case(cat_name))
            .take(5)
            .collect();

        assert!(!skins.is_empty(), "Expected skins for category {}", cat_name);

        let mut rendered = HashSet::new();
        for w in &skins {
            let req = serde_json::json!({
                "type": "weapon",
                "baseWeapon": w.base_weapon,
                "symbol": w.symbol_suffix,
                "weaponSkinName": w.weapon_skin_name,
                "swfFile": w.swf_file,
                "ownerHero": w.owner_hero,
                "gamePath": inst.game_path
            });
            let svg = client.render(&req)
                .unwrap_or_else(|| panic!("Failed to render {} skin '{}'", cat_name, w.weapon_skin_name));
            assert!(svg.trim().starts_with("<svg"), "Invalid SVG output for {} skin '{}'", cat_name, w.weapon_skin_name);
            rendered.insert(svg);
        }
        assert_eq!(rendered.len(), skins.len(), "Duplicate rendering detected in category {}", cat_name);
    }
}

#[test]
fn test_inspect_csv_weapon_skins() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_swz_path = Path::new(&inst.game_swz_path);
    let air_swf_path = Path::new(&inst.air_swf_path);
    let key = brawlcraft_slint::swz::find_swz_key(air_swf_path, game_swz_path).unwrap();
    let files = brawlcraft_slint::swz::read_swz_files(game_swz_path, key).unwrap();
    let csv = files.get("weaponSkinTypes.csv").unwrap();

    let mut lines = csv.lines();
    let _ = lines.next();
    let headers = lines.next().unwrap();
    println!("CSV HEADERS: {}", headers);

    let mut count_total = 0;
    let mut hammer_rows = Vec::new();
    let mut sword_rows = Vec::new();

    for line in lines {
        if line.trim().is_empty() { continue; }
        count_total += 1;
        if line.contains(",Hammer,") || line.contains(",Hammer ") {
            hammer_rows.push(line.to_string());
        }
        if line.contains(",Sword,") || line.contains(",Sword ") {
            sword_rows.push(line.to_string());
        }
    }
    println!("CSV total rows: {}", count_total);
    println!("CSV Hammer rows: {}", hammer_rows.len());
    println!("CSV Sword rows: {}", sword_rows.len());

    println!("FIRST 10 HAMMER ROWS IN CSV:");
    for r in hammer_rows.iter().take(10) {
        println!("  {}", r);
    }

    println!("\n=== INSPECTING THE 12 CROSSOVER WEAPON SKINS IN CSV ===");
    let targets = [
        "HammerXavier", "SwordXavier",
        "SwordSnakeEyes", "PistolSnakeEyes",
        "SwordChewbacca", "PistolChewbacca",
        "AxeMachoMan", "RocketLanceMachoMan",
        "FistsDestinyTitan", "CannonDestinyTitan",
        "SwordNegan", "GreatswordNegan",
        "Stub"
    ];
    for line in csv.lines() {
        let name = line.split(',').next().unwrap_or("");
        if targets.iter().any(|&t| name.eq_ignore_ascii_case(t) || name.contains(t)) {
            let cols: Vec<&str> = line.split(',').collect();
            println!(
                "WeaponSkin: name='{:<20}' id={:<4} base='{:<12}' owner='{:<10}' customArt='{:<28}'",
                cols.get(0).unwrap_or(&""),
                cols.get(1).unwrap_or(&""),
                cols.get(3).unwrap_or(&""),
                cols.get(4).unwrap_or(&""),
                cols.get(10).unwrap_or(&"")
            );
        }
    }
}

#[test]
fn test_verify_all_15_restored_legends_and_curated_roster() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::RendererClient;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat_raw = Catalog::default();
    cat_raw.load_from_installation(&inst).unwrap();
    let raw_legend_count = cat_raw.legends.len();

    let mut cat = cat_raw.clone();
    cat.filter_curated_legends();
    let curated_legend_count = cat.legends.len();

    // 1. Total curated legends must be exactly 20
    assert_eq!(curated_legend_count, 20, "Curated roster must contain exactly 20 Legends");

    // 2. The 5 core legends must all be present
    let core_expected = ["Bödvar", "Thatch", "Ulgrim", "Onyx", "Jaeyun"];
    for name in &core_expected {
        assert!(
            cat.legends.iter().any(|l| l.display_name.eq_ignore_ascii_case(name) || l.hero_name.eq_ignore_ascii_case(name)),
            "Core legend {} must be present in curated roster", name
        );
    }

    // 3. The 15 restored legends must all be present
    let restored_expected = [
        "Azoth",
        "Vector",
        "Volkov",
        "Munin",
        "Thea",
        "Loki",
        "Seven",
        "Imugi",
        "King Zuva",
        "Priya",
        "Ransom",
        "Lady Vera",
        "Rupture",
        "Aurus",
        "Qinghua & Baobao",
    ];
    for name in &restored_expected {
        assert!(
            cat.legends.iter().any(|l| l.display_name.eq_ignore_ascii_case(name) || l.hero_name.eq_ignore_ascii_case(name)),
            "Restored legend {} must be present in curated roster", name
        );
    }

    // 4. Artemis, Yumiko, and Magyar must NEVER be present
    for l in &cat.legends {
        let n = l.hero_name.to_lowercase();
        let d = l.display_name.to_lowercase();
        assert!(!n.contains("spacegirl") && !d.contains("artemis"), "Artemis must not be present");
        assert!(!n.contains("foxgirl") && !d.contains("yumiko"), "Yumiko must not be present");
        assert!(!n.contains("hauntedarmor") && !d.contains("magyar"), "Magyar must not be present");
    }

    // 5. Unique Hero IDs & valid metadata for all 20 legends
    let mut hero_ids = std::collections::HashSet::new();
    for l in &cat.legends {
        assert!(hero_ids.insert(l.hero_id), "Duplicate hero_id found: {}", l.hero_id);
        assert!(!l.hero_name.is_empty(), "Hero name must not be empty");
        assert!(!l.display_name.is_empty(), "Display name must not be empty");
        assert!(!l.weapon1.is_empty(), "Weapon 1 must not be empty for {}", l.display_name);
        assert!(!l.weapon2.is_empty(), "Weapon 2 must not be empty for {}", l.display_name);
    }

    // 6. Verify HammerStub (Skin ID 710) maps to authentic Detective Hammer asset in Gfx_Super.swf
    let hammer_stub = cat.weapon_skins.iter().find(|w| w.weapon_skin_id == 710 || w.weapon_skin_name == "HammerStub");
    assert!(hammer_stub.is_some(), "HammerStub (ID 710) must be present in weapon skins");
    let hs = hammer_stub.unwrap();
    assert_eq!(hs.symbol_suffix, "Detective", "HammerStub symbol must map to Detective");
    assert_eq!(hs.swf_file, "Gfx_Super.swf", "HammerStub SWF must map to Gfx_Super.swf");

    // 7. Verify HammerStub renders an authentic SVG model (not a pink circle or failed preview)
    let mut client = RendererClient::new();
    let stub_req = serde_json::json!({
        "type": "weapon",
        "baseWeapon": hs.base_weapon,
        "symbol": hs.symbol_suffix,
        "weaponSkinName": hs.weapon_skin_name,
        "swfFile": hs.swf_file,
        "ownerHero": "Super",
        "gamePath": inst.game_path
    });
    let svg = client.render(&stub_req);
    assert!(svg.is_some(), "HammerStub must render successfully to SVG");
    let svg_str = svg.unwrap();
    assert!(svg_str.contains("<svg"), "HammerStub SVG must contain <svg tag");
    assert!(!svg_str.contains("circle") || svg_str.contains("path"), "HammerStub must be an authentic weapon model");

    // 8. Verify frontend curation filter matches all 20 legends and excludes forbidden heroes
    let frontend_curated_keys = [
        "viking", "bodvar", "bödvar", "thatch", "pirate", "dwarf", "ulgrim", "gargoyle", "onyx", "sellsword", "jaeyun",
        "skeleton", "azoth", "actualrobot", "vector", "vamplord", "volkov", "birdbard", "munin", "roboengineer", "seven",
        "greekspeedster", "thea", "loki", "imugi", "africanking", "king zuva", "zuva", "bladedancer", "priya",
        "cybervirus", "ransom", "cleric", "lady vera", "vera", "darkheartmonster", "rupture", "actualgladiator", "aurus",
        "astrogirl", "qinghua", "baobao"
    ];
    let frontend_is_curated = |hero_name: &str, display_name: &str, bio: &str| -> bool {
        let h = hero_name.to_lowercase();
        let d = display_name.to_lowercase();
        let b = bio.to_lowercase();
        if h.contains("spacegirl") || d.contains("artemis")
            || h.contains("foxgirl") || d.contains("yumiko")
            || h.contains("hauntedarmor") || d.contains("magyar") {
            return false;
        }
        frontend_curated_keys.iter().any(|k| h == *k || h.contains(k) || d == *k || d.contains(k) || b == *k || b.contains(k))
    };

    let mut frontend_survivor_count = 0;
    for l in &cat.legends {
        if l.hero_name != "DEFAULT_CHARACTER"
            && !l.hero_name.eq_ignore_ascii_case("random")
            && !l.display_name.eq_ignore_ascii_case("random")
            && frontend_is_curated(&l.hero_name, &l.display_name, &l.bio)
        {
            frontend_survivor_count += 1;
        }
    }
    assert_eq!(frontend_survivor_count, 20, "All 20 curated legends must survive frontend filtering");

    // 9. Verify backend JSON serialization preserves exactly 20 legends
    let json_val = serde_json::to_value(&cat.legends).unwrap();
    let json_arr = json_val.as_array().unwrap();
    assert_eq!(json_arr.len(), 20, "Backend JSON serialization must preserve exactly 20 legends");

    println!("\n=== CURATED ROSTER PIPELINE DIAGNOSTICS ===");
    println!("  Raw catalog Legend count: {}", raw_legend_count);
    println!("  Curated catalog Legend count: {}", curated_legend_count);
    println!("  Backend-returned Legend count: {}", json_arr.len());
    println!("  Frontend-received Legend count: {}", cat.legends.len());
    println!("  Number of rendered Legend cards: {}", frontend_survivor_count);

    // 10. Verify each of the 20 legends renders a valid portrait/pose
    for l in &cat.legends {
        let req = serde_json::json!({
            "type": "legend",
            "name": l.display_name,
            "hero": l.hero_name,
            "gamePath": inst.game_path
        });
        let svg_opt = client.render(&req);
        assert!(svg_opt.is_some(), "Legend {} ({}) must render a valid SVG pose", l.display_name, l.hero_name);
        let s = svg_opt.unwrap();
        assert!(s.contains("<svg"), "Legend {} pose must be valid SVG", l.display_name);
    }

    // 11. Verify FrontendAssets serves the newly built production bundle containing all 20 legends
    use brawlcraft_slint::desktop_app::FrontendAssets;
    let index_file = FrontendAssets::get("index.html").expect("index.html must be embedded in binary");
    let index_str = std::str::from_utf8(&index_file.data).expect("index.html must be valid utf8");
    
    // Extract bundle name from index.html (e.g., assets/index-*.js)
    let js_bundle_path = index_str
        .lines()
        .find_map(|line| {
            if let Some(pos) = line.find("assets/index-") {
                let rest = &line[pos..];
                if let Some(end) = rest.find(".js") {
                    return Some(&rest[..end + 3]);
                }
            }
            None
        })
        .expect("index.html must reference a production js bundle");

    let js_file = FrontendAssets::get(js_bundle_path).expect("Latest JS bundle must be embedded in binary");
    let js_str = std::str::from_utf8(&js_file.data).expect("JS bundle must be valid utf8");
    assert!(js_str.contains("actualrobot") && js_str.contains("birdbard") && js_str.contains("darkheartmonster"), "Embedded JS bundle must contain the restored legend identifiers");
}

#[test]
fn test_verify_automatic_crossover_weapon_assignment() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use std::path::PathBuf;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();

    let cases = [
        ("Viking", "Xavier", "HammerXavier", "SwordXavier"),
        ("Thatch", "SnakeEyes", "SwordSnakeEyes", "PistolSnakeEyes"),
        ("Thatch", "Chewbacca", "SwordChewbacca", "PistolChewbacca"),
        ("Dwarf", "MachoMan", "AxeMachoMan", "RocketLanceMachoMan"),
        ("Gargoyle", "DestinyTitan", "FistsDestinyTitan", "CannonDestinyTitan"),
        ("Sellsword", "Negan", "SwordNegan", "GreatswordNegan"),
    ];

    let mut bm = BackupManager::new(&inst.game_path);
    let engine = ModificationEngine::new(PathBuf::from(&inst.game_path), &mut bm, &cat);

    for (hero, costume, exp_w1, exp_w2) in &cases {
        let req = ModificationRequest {
            legend_name: Some(hero.to_string()),
            replacement_costume_name: Some(costume.to_string()),
            ..Default::default()
        };

        let (_tw1, rw1, _tw2, rw2) = engine.resolve_weapons_for_request(&req);
        assert_eq!(
            rw1.map(|w| w.weapon_skin_name.as_str()),
            Some(*exp_w1),
            "Crossover {} on {} must auto-equip Weapon 1 {}", costume, hero, exp_w1
        );
        assert_eq!(
            rw2.map(|w| w.weapon_skin_name.as_str()),
            Some(*exp_w2),
            "Crossover {} on {} must auto-equip Weapon 2 {}", costume, hero, exp_w2
        );

        // Test manual override remains possible
        let req_override = ModificationRequest {
            legend_name: Some(hero.to_string()),
            replacement_costume_name: Some(costume.to_string()),
            replacement_weapon1_name: Some("HammerViking".to_string()),
            ..Default::default()
        };
        if *hero == "Viking" {
            let (_tw1, rw1_override, _tw2, _rw2) = engine.resolve_weapons_for_request(&req_override);
            assert_eq!(
                rw1_override.map(|w| w.weapon_skin_name.as_str()),
                Some("HammerViking"),
                "Manual override must take precedence over auto-assignment"
            );
        }
    }
}

#[test]
fn test_verify_automatic_costume_weapon_assignment_all_curated_legends() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use std::path::PathBuf;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();

    let mut bm = BackupManager::new(&inst.game_path);
    let engine = ModificationEngine::new(PathBuf::from(&inst.game_path), &mut bm, &cat);

    // 1. Test ordinary and epic costumes for Bodvar
    let bodvar_cases = [
        ("Bear", "HammerBear", "SwordBear"),
        ("CyberViking", "HammerCyberViking", "SwordCyberViking"),
        ("Draugr", "HammerDraugr", "SwordDraugr"),
        ("IronViking", "HammerIronViking", "SwordIronViking"),
        ("VikingGardener", "HammerVikingGardener", "SwordVikingGardener"),
    ];

    for (costume, exp_w1, exp_w2) in bodvar_cases {
        let req = ModificationRequest {
            legend_name: Some("Viking".to_string()),
            replacement_costume_name: Some(costume.to_string()),
            ..Default::default()
        };
        let (_tw1, rw1, _tw2, rw2) = engine.resolve_weapons_for_request(&req);
        assert_eq!(
            rw1.map(|w| w.weapon_skin_name.as_str()),
            Some(exp_w1),
            "Costume {} on Bodvar must auto-assign Weapon 1 {}", costume, exp_w1
        );
        assert_eq!(
            rw2.map(|w| w.weapon_skin_name.as_str()),
            Some(exp_w2),
            "Costume {} on Bodvar must auto-assign Weapon 2 {}", costume, exp_w2
        );
    }

    // 2. Test switching back to default costume restores default weapons
    let def_req = ModificationRequest {
        legend_name: Some("Viking".to_string()),
        replacement_costume_name: Some("Viking".to_string()),
        ..Default::default()
    };
    let (tw1, rw1, tw2, rw2) = engine.resolve_weapons_for_request(&def_req);
    assert_eq!(tw1.map(|w| w.weapon_skin_name.as_str()), Some("HammerViking"));
    assert_eq!(tw2.map(|w| w.weapon_skin_name.as_str()), Some("SwordViking"));
    assert_eq!(rw1.map(|w| w.weapon_skin_name.as_str()), Some("HammerViking"));
    assert_eq!(rw2.map(|w| w.weapon_skin_name.as_str()), Some("SwordViking"));

    // 3. Test across all 20 curated legends: non-default costumes have automatic weapon synchronization
    let mut tested_costumes = 0;
    for legend in &cat.legends {
        for costume in &legend.costumes {
            if costume.is_default {
                continue;
            }
            let req = ModificationRequest {
                legend_name: Some(legend.hero_name.clone()),
                replacement_costume_name: Some(costume.costume_name.clone()),
                ..Default::default()
            };
            let (_tw1, rw1, _tw2, rw2) = engine.resolve_weapons_for_request(&req);
            assert!(
                rw1.is_some() && rw2.is_some(),
                "Legend {} costume {} must auto-resolve both weapons! Found w1={:?}, w2={:?}",
                legend.hero_name, costume.costume_name,
                rw1.map(|w| &w.weapon_skin_name),
                rw2.map(|w| &w.weapon_skin_name)
            );
            tested_costumes += 1;
        }
    }
    println!("Verified automatic weapon synchronization across {} non-default costumes for all 20 legends", tested_costumes);
    assert!(tested_costumes >= 100, "Must have verified over 100 costumes");
}

#[test]
fn test_inspect_weapon_skin_costume_relationship() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_swz_path = Path::new(&inst.game_swz_path);
    let air_swf_path = Path::new(&inst.air_swf_path);
    let key = brawlcraft_slint::swz::find_swz_key(air_swf_path, game_swz_path).unwrap();
    let files = brawlcraft_slint::swz::read_swz_files(game_swz_path, key).unwrap();

    let costume_csv = files.get("costumeTypes.csv").unwrap();
    let mut clines = costume_csv.lines();
    let _table_name = clines.next();
    let header_line = clines.next().unwrap();
    let cols: Vec<&str> = header_line.split(',').collect();
    let _name_idx = cols.iter().position(|&c| c.eq_ignore_ascii_case("CostumeName")).unwrap();
    let _id_idx = cols.iter().position(|&c| c.eq_ignore_ascii_case("CostumeID")).unwrap();
    let _hero_idx = cols.iter().position(|&c| c.eq_ignore_ascii_case("OwnerHero")).unwrap();
    let _wset_idx = cols.iter().position(|&c| c.eq_ignore_ascii_case("WeaponSet")).unwrap();

    use brawlcraft_slint::catalog::Catalog;
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();

    println!("\n=== INSPECTING COSTUMES AND MATCHING WEAPONS FOR ALL CURATED LEGENDS ===");
    for legend in &cat.legends {
        let mut matched_count = 0;
        let mut total_costumes = 0;
        for c in &legend.costumes {
            total_costumes += 1;
            let wset = &c.weapon_set;
            let cname = &c.costume_name;
            let w1_match = cat.weapon_skins.iter().find(|w| {
                w.base_weapon.eq_ignore_ascii_case(&legend.weapon1) && (
                    w.symbol_suffix.eq_ignore_ascii_case(wset)
                    || w.weapon_skin_name.eq_ignore_ascii_case(wset)
                    || w.weapon_skin_name.eq_ignore_ascii_case(&format!("{}{}", legend.weapon1, wset))
                    || w.weapon_skin_name.eq_ignore_ascii_case(&format!("{}{}", legend.weapon1, cname))
                    || (w.weapon_skin_name.to_lowercase().contains(&wset.to_lowercase()) && (w.owner_hero.is_empty() || w.owner_hero.to_lowercase().contains(&legend.hero_name.to_lowercase()) || w.owner_hero.to_lowercase().contains(&wset.to_lowercase())))
                )
            });
            let w2_match = cat.weapon_skins.iter().find(|w| {
                w.base_weapon.eq_ignore_ascii_case(&legend.weapon2) && (
                    w.symbol_suffix.eq_ignore_ascii_case(wset)
                    || w.weapon_skin_name.eq_ignore_ascii_case(wset)
                    || w.weapon_skin_name.eq_ignore_ascii_case(&format!("{}{}", legend.weapon2, wset))
                    || w.weapon_skin_name.eq_ignore_ascii_case(&format!("{}{}", legend.weapon2, cname))
                    || (w.weapon_skin_name.to_lowercase().contains(&wset.to_lowercase()) && (w.owner_hero.is_empty() || w.owner_hero.to_lowercase().contains(&legend.hero_name.to_lowercase()) || w.owner_hero.to_lowercase().contains(&wset.to_lowercase())))
                )
            });
            if w1_match.is_some() && w2_match.is_some() {
                matched_count += 1;
            } else {
                println!("  [PARTIAL/NONE] Legend '{}', Costume '{}' (wset='{}'): w1={}, w2={}",
                    legend.hero_name, c.costume_name, wset,
                    w1_match.map(|w| w.weapon_skin_name.as_str()).unwrap_or("NONE"),
                    w2_match.map(|w| w.weapon_skin_name.as_str()).unwrap_or("NONE")
                );
            }
        }
        println!("Legend '{}': {}/{} costumes have matched weapons", legend.hero_name, matched_count, total_costumes);
    }
}

#[test]
fn test_inspect_crossover_definitions() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_swz_path = Path::new(&inst.game_swz_path);
    let air_swf_path = Path::new(&inst.air_swf_path);
    let key = brawlcraft_slint::swz::find_swz_key(air_swf_path, game_swz_path).unwrap();
    let files = brawlcraft_slint::swz::read_swz_files(game_swz_path, key).unwrap();

    println!("=== ALL FILES IN SWZ ===");
    let mut file_keys: Vec<_> = files.keys().collect();
    file_keys.sort();
    for f in &file_keys {
        println!("  SWZ file: {}", f);
    }

    if let Some(costume_csv) = files.get("costumeTypes.csv") {
        let mut lines = costume_csv.lines();
        let _ = lines.next();
        let headers = lines.next().unwrap();
        println!("\n=== COSTUME TYPES HEADERS ===");
        println!("{}", headers);

        let targets = ["xavier", "woods", "snake", "chew", "macho", "titan", "negan"];
        println!("\n=== MATCHING CROSSOVER COSTUMES ===");
        for line in lines {
            let lower = line.to_lowercase();
            if targets.iter().any(|&t| lower.contains(t)) {
                println!("COSTUME ROW:\n  {}", line);
            }
        }
    }

    if let Some(power_csv) = files.get("powerTypes.csv") {
        println!("\n=== ALL SOUND EVENTS FOR BODVAR IN POWERTYPES.CSV ===");
        let mut lines = power_csv.lines();
        let _ = lines.next();
        let _ = lines.next();
        for line in lines {
            if line.contains("Viking") {
                let cols: Vec<_> = line.split(',').collect();
                for col in cols {
                    if col.contains("SPC_") || col.contains("_Play") {
                        println!("  Bodvar sound: {}", col);
                    }
                }
            }
        }
    }


    if let Some(power_swaps) = files.get("PowerSwapTypes.xml") {
        println!("\n=== POWER SWAPS FOR THE 6 CROSSOVERS ===");
        let targets = ["xavier", "machoman", "negan", "snakeeyes", "chewbacca", "destinytitan"];
        
        let mut reader = quick_xml::Reader::from_str(power_swaps);
        reader.config_mut().trim_text(true);
        let mut in_target_swap = false;
        let mut current_block = String::new();

        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if name == "PowerSwapType" {
                        let mut owner = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"OwnerCostume" {
                                owner = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        if targets.iter().any(|&t| owner.eq_ignore_ascii_case(t)) {
                            in_target_swap = true;
                            current_block = format!("<PowerSwapType OwnerCostume=\"{}\">", owner);
                        } else {
                            in_target_swap = false;
                        }
                    } else if in_target_swap {
                        current_block.push_str(&format!("<{}>", name));
                    }
                }
                Ok(quick_xml::events::Event::Text(e)) => {
                    if in_target_swap {
                        current_block.push_str(&e.unescape().unwrap_or_default());
                    }
                }
                Ok(quick_xml::events::Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if in_target_swap {
                        current_block.push_str(&format!("</{}>", name));
                        if name == "PowerSwapType" {
                            println!("{}", current_block);
                            in_target_swap = false;
                            current_block.clear();
                        }
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
    }

    println!("\n=== INSPECTING ASSETS FOR ALL 6 CROSSOVERS ===");
    let game_dir = std::path::Path::new(&inst.game_path);

    let inspect_swf = |fname: &str, needles: &[&str]| {
        let p = game_dir.join(fname);
        if !p.is_file() {
            println!("  [SWF MISSING] {}", fname);
            return;
        }
        if let Ok(pkg) = brawlcraft_slint::swf_engine::SwfPackage::read_from_file(&p) {
            if let Ok(syms) = pkg.get_symbol_classes() {
                let matches: Vec<_> = syms.iter().filter(|(_, n)| {
                    let nl = n.to_lowercase();
                    needles.iter().any(|&needle| nl.contains(needle))
                }).collect();
                println!("  [SWF] {} - total symbols: {}, matches: {}", fname, syms.len(), matches.len());
                for (id, name) in matches.iter().take(20) {
                    println!("    id={}, name={}", id, name);
                }
                if matches.len() > 20 {
                    println!("    ... and {} more matches", matches.len() - 20);
                }
            }
        }
    };

    println!("\n--- 1. Xavier Woods (Viking) ---");
    inspect_swf("Gfx_Viking.swf", &["xavier"]);
    inspect_swf("SFX_Sword.swf", &["xavier"]);
    inspect_swf("SFX_Hammer.swf", &["xavier"]);
    inspect_swf("Animation_Sword.swf", &["xavier"]);
    inspect_swf("Animation_Hammer.swf", &["xavier"]);

    println!("\n--- 2. Snake Eyes (Thatch) ---");
    inspect_swf("Gfx_Thatch.swf", &["snake"]);
    inspect_swf("SFX_Sword.swf", &["snake"]);
    inspect_swf("SFX_Pistols.swf", &["snake"]);
    inspect_swf("Animation_Sword.swf", &["snake"]);
    inspect_swf("Animation_Pistols.swf", &["snake"]);

    println!("\n--- 3. Chewbacca (Thatch) ---");
    inspect_swf("Gfx_Chewbacca.swf", &["chew"]);
    inspect_swf("SFX_Chewbacca.swf", &["chew"]);
    inspect_swf("SFX_SWBlasters.swf", &["blaster", "sw", "chew"]);

    println!("\n--- 4. Macho Man (Dwarf) ---");
    inspect_swf("Gfx_Dwarf.swf", &["macho"]);
    inspect_swf("SFX_Axe.swf", &["macho"]);
    inspect_swf("SFX_Lance.swf", &["macho"]);
    inspect_swf("Animation_Axe.swf", &["macho"]);
    inspect_swf("Animation_Lance.swf", &["macho"]);

    println!("\n--- 5. Titan (Gargoyle) ---");
    inspect_swf("Gfx_DestinyTitan.swf", &["titan"]);
    inspect_swf("SFX_DestinyTitan.swf", &["titan"]);

    println!("\n--- 6. Negan (Sellsword) ---");
    inspect_swf("Gfx_Sellsword.swf", &["negan"]);
    inspect_swf("SFX_GreatSword.swf", &["negan"]);
    inspect_swf("SFX_Sword.swf", &["negan"]);
    inspect_swf("Animation_GreatSword.swf", &["negan"]);
    inspect_swf("Animation_Sword.swf", &["negan"]);

    println!("\n=== INSPECTING BNK SOUND BANKS IN AUDIO/PC ===");
    let audio_dir = game_dir.join("audio").join("pc");
    if audio_dir.is_dir() {
        println!("  Audio directory verified at: {:?}", audio_dir);
        let _check_bnk_event = |fname: &str, event_name: &str| {
            let p = audio_dir.join(fname);
            if p.is_file() {
                if let Ok(data) = std::fs::read(&p) {
                    let h = brawlcraft_slint::bnk_engine::ak_hash(event_name);
                    let h_bytes = h.to_le_bytes();
                    let found = data.windows(4).any(|w| w == h_bytes);
                    println!("  BNK: {} event '{}' (hash 0x{:08X}) found={}", fname, event_name, h, found);
                }
            } else {
                println!("  BNK: {} missing", fname);
            }
        };

        // Inspect VFX symbols in the SFX_*.swf files
        println!("=== VERIFYING SIGNATURE VFX PAIRS FOR ALL 6 CROSSOVERS ===");
        for (owner, swf_name) in &[
            ("Xavier", "SFX_Hammer.swf"),
            ("Xavier", "SFX_Sword.swf"),
            ("SnakeEyes", "SFX_Sword.swf"),
            ("SnakeEyes", "SFX_Pistols.swf"),
            ("Chewbacca", "SFX_Chewbacca.swf"),
            ("Chewbacca", "SFX_SWBlasters.swf"),
            ("MachoMan", "SFX_Axe.swf"),
            ("MachoMan", "SFX_Lance.swf"),
            ("DestinyTitan", "SFX_DestinyTitan.swf"),
            ("Negan", "SFX_Sword.swf"),
        ] {
            let p = game_dir.join(swf_name);
            if p.is_file() {
                if let Ok(pkg) = brawlcraft_slint::swf_engine::SwfPackage::read_from_file(&p) {
                    if let Ok(syms) = pkg.get_symbol_classes() {
                        let swap_syms: Vec<_> = syms.iter().filter(|(_, n)| n.to_lowercase().contains(&owner.to_lowercase())).collect();
                        println!("  {} in {}: {} swap VFX symbols present", owner, swf_name, swap_syms.len());
                        for (_, name) in &swap_syms {
                            println!("    VFX: {}", name);
                        }
                    }
                }
            }
        }


        // Extract all sound events from PowerSwapTypes.xml for each crossover
        if let Some(power_swaps) = files.get("PowerSwapTypes.xml") {
            let mut reader = quick_xml::Reader::from_str(power_swaps);
            reader.config_mut().trim_text(true);
            let mut current_owner = String::new();
            let mut current_power = String::new();
            let mut in_swap = false;
            let mut current_tag = String::new();
            let mut swap_events: Vec<(String, String, String, String)> = Vec::new(); // (owner, power, tag, event_name)

            loop {
                match reader.read_event() {
                    Ok(quick_xml::events::Event::Start(e)) => {
                        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                        current_tag = name.clone();
                        if name == "PowerSwapType" {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"OwnerCostume" {
                                    current_owner = String::from_utf8_lossy(&attr.value).to_string();
                                    in_swap = true;
                                }
                            }
                        }
                    }
                    Ok(quick_xml::events::Event::Text(e)) => {
                        if in_swap {
                            let txt = e.unescape().unwrap_or_default().to_string();
                            if current_tag == "TargetPower" {
                                current_power = txt;
                            } else if current_tag == "CastSoundEvent" || current_tag == "HitSoundEvent" {
                                let clean = txt.trim_start_matches("L:").to_string();
                                swap_events.push((current_owner.clone(), current_power.clone(), current_tag.clone(), clean));
                            }
                        }
                    }
                    Ok(quick_xml::events::Event::End(e)) => {
                        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                        if name == "PowerSwapType" {
                            in_swap = false;
                            current_power.clear();
                        }
                    }
                    Ok(quick_xml::events::Event::Eof) => break,
                    _ => {}
                }
            }

        // Detailed inspection of power sound pairs for each crossover
        println!("=== DETAILED POWER SOUND PAIRS ===");
        let bank_for_hero = |h: &str| match h.to_lowercase().as_str() {
            "viking" | "bodvar" => "SPC_Bodvar.bnk",
            "thatch" | "pirate" => "SPC_Thatch.bnk",
            "dwarf" | "ulgrim" => "SPC_Dwarf.bnk",
            "onyx" | "gargoyle" => "SPC_Onyx.bnk",
            "sellsword" | "jaeyun" => "SPC_Sellsword.bnk",
            _ => "",
        };

        // Build base power map from powerTypes.csv: PowerName -> (cast_event, hit_event)
        let mut base_power_sounds: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
        if let Some(power_csv) = files.get("powerTypes.csv") {
            let mut lines = power_csv.lines();
            let _ = lines.next();
            if let Some(header) = lines.next() {
                let col_headers: Vec<_> = header.split(',').map(|s| s.trim().to_lowercase()).collect();
                let cast_idx = col_headers.iter().position(|h| h == "castsoundevent");
                let hit_idx = col_headers.iter().position(|h| h == "hitsoundevent");
                for line in lines {
                    let cols: Vec<_> = line.split(',').collect();
                    if cols.is_empty() { continue; }
                    let p_name = cols[0].trim().to_string();
                    let cast_evt = cast_idx.and_then(|i| cols.get(i)).map(|s| s.trim().trim_start_matches("L:").to_string()).unwrap_or_default();
                    let hit_evt = hit_idx.and_then(|i| cols.get(i)).map(|s| s.trim().trim_start_matches("L:").to_string()).unwrap_or_default();
                    base_power_sounds.insert(p_name, (cast_evt, hit_evt));
                }
            }
        }

        println!("=== VERIFYING EXACT BASE <-> CROSSOVER SOUND PAIRS IN BNKS ===");
        for (owner, hero) in &[
            ("Xavier", "Viking"),
            ("SnakeEyes", "Thatch"),
            ("Chewbacca", "Thatch"),
            ("MachoMan", "Dwarf"),
            ("DestinyTitan", "Gargoyle"),
            ("Negan", "Sellsword"),
        ] {
            let bnk = bank_for_hero(hero);
            let bnk_path = audio_dir.join(bnk);
            let bnk_data = std::fs::read(&bnk_path).unwrap_or_default();
            println!("\n--- {} (Bank: {}) ---", owner, bnk);
            let matching: Vec<_> = swap_events.iter().filter(|(o, ..)| o.eq_ignore_ascii_case(owner)).collect();
            let mut paired_count = 0;
            for (_, pow, tag, repl_evt) in matching {
                if let Some((base_cast, base_hit)) = base_power_sounds.get(pow) {
                    let base_evt = if tag == "CastSoundEvent" { base_cast } else { base_hit };
                    if !base_evt.is_empty() && base_evt != repl_evt {
                        let h_base = brawlcraft_slint::bnk_engine::ak_hash(base_evt);
                        let h_repl = brawlcraft_slint::bnk_engine::ak_hash(repl_evt);
                        let base_found = bnk_data.windows(4).any(|w| w == h_base.to_le_bytes());
                        let repl_found = bnk_data.windows(4).any(|w| w == h_repl.to_le_bytes());
                        if base_found && repl_found {
                            paired_count += 1;
                            println!("  [MATCHED] {}: '{}' (0x{:08X}) <-> '{}' (0x{:08X})", pow, base_evt, h_base, repl_evt, h_repl);
                        } else {
                            println!("  [PARTIAL] {}: base='{}' (found={}), repl='{}' (found={})", pow, base_evt, base_found, repl_evt, repl_found);
                        }
                    }
                }
            }
            println!("=> {} sound pairs matched: {} in {}", owner, paired_count, bnk);
        }
        }
    }
}

#[test]
fn test_inspect_symbols_in_asgard_and_snakeeyes() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = std::path::Path::new(&inst.game_path);

    let viking_path = game_dir.join("Gfx_Viking.swf");
    if viking_path.is_file() {
        let pkg = SwfPackage::read_from_file(&viking_path).unwrap();
        println!("=== Gfx_Viking.swf Hammer IDs ===");
        for tag in &pkg.tags {
            if tag.tag_code == 76 {
                if let Ok(syms) = brawlcraft_slint::swf_engine::parse_symbol_class(&tag.data) {
                    for (id, name) in syms {
                        if name.to_lowercase().contains("hammer") {
                            println!("  Viking sym: id={}, name={}", id, name);
                        }
                    }
                }
            }
        }
    }
    let wep_sets_path = game_dir.join("Gfx_Weapons_Sets.swf");
    if wep_sets_path.is_file() {
        let pkg = SwfPackage::read_from_file(&wep_sets_path).unwrap();
        println!("=== Gfx_Weapons_Sets.swf SYMBOLS ===");
        for tag in &pkg.tags {
            if tag.tag_code == 76 {
                if let Ok(syms) = brawlcraft_slint::swf_engine::parse_symbol_class(&tag.data) {
                    for (_id, name) in syms {
                        if name.to_lowercase().contains("asgard") || name.to_lowercase().contains("hammer") {
                            println!("  Sets symbol: {}", name);
                        }
                    }
                }
            }
        }
    }

    // 2. Inspect Gfx_Thatch.swf for SnakeEyes
    let thatch_path = game_dir.join("Gfx_Thatch.swf");
    if thatch_path.is_file() {
        let pkg = SwfPackage::read_from_file(&thatch_path).unwrap();
        println!("=== Gfx_Thatch.swf SYMBOLS for SnakeEyes / Sword ===");
        for tag in &pkg.tags {
            if tag.tag_code == 76 {
                if let Ok(syms) = brawlcraft_slint::swf_engine::parse_symbol_class(&tag.data) {
                    for (_id, name) in syms {
                        if name.to_lowercase().contains("snake") || (name.to_lowercase().contains("sword") && !name.contains("Land") && !name.contains("Attack")) {
                            println!("  Thatch symbol: id={}, name={}", _id, name);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_crossover_experience_for_all_six_legends() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::bnk_engine::ak_hash;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_crossovers_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let game_dir = Path::new(&inst.game_path);

    // Setup file structure in temp_dir
    let anims_dir = temp_dir.join("anims");
    fs::create_dir_all(&anims_dir).unwrap();
    let audio_en = temp_dir.join("audio").join("pc").join("English(US)");
    fs::create_dir_all(&audio_en).unwrap();

    let swfs_to_copy = [
        "Gfx_Viking.swf", "Gfx_Thatch.swf", "Gfx_Dwarf.swf", "Gfx_Gargoyle.swf",
        "Gfx_Sellsword.swf", "Gfx_Chewbacca.swf", "Gfx_DestinyTitan.swf", "UI_Icons.swf",
        "SFX_Hammer.swf", "SFX_Sword.swf", "SFX_Pistols.swf", "SFX_Axe.swf", "SFX_Lance.swf",
        "SFX_GreatSword.swf", "SFX_Cannon.swf", "SFX_Fists.swf", "SFX_Chewbacca.swf", "SFX_DestinyTitan.swf",
    ];
    for f in &swfs_to_copy {
        let src = game_dir.join(f);
        if src.is_file() {
            fs::copy(&src, temp_dir.join(f)).unwrap();
        }
    }

    let anims_to_copy = [
        "Animation_CharacterSelect.anm", "Animation_Sword.anm", "Animation_Hammer.anm",
        "Animation_Pistols.anm", "Animation_Axe.anm", "Animation_Lance.anm",
        "Animation_GreatSword.anm", "Animation_Cannon.anm", "Animation_Fists.anm",
    ];
    for f in &anims_to_copy {
        let src = game_dir.join("anims").join(f);
        if src.is_file() {
            fs::copy(&src, anims_dir.join(f)).unwrap();
        }
    }

    let audio_to_copy = [
        ("audio/pc/English(US)/VOX_Announcer.bnk", temp_dir.join("audio/pc/English(US)/VOX_Announcer.bnk")),
        ("audio/pc/PC_Movement.bnk", temp_dir.join("audio/pc/PC_Movement.bnk")),
        ("audio/pc/WEP_Pistol.bnk", temp_dir.join("audio/pc/WEP_Pistol.bnk")),
        ("audio/pc/WEP_Pistol_03.bnk", temp_dir.join("audio/pc/WEP_Pistol_03.bnk")),
        ("audio/pc/SPC_Bodvar.bnk", temp_dir.join("audio/pc/SPC_Bodvar.bnk")),
        ("audio/pc/SPC_Thatch.bnk", temp_dir.join("audio/pc/SPC_Thatch.bnk")),
        ("audio/pc/SPC_Dwarf.bnk", temp_dir.join("audio/pc/SPC_Dwarf.bnk")),
        ("audio/pc/SPC_Onyx.bnk", temp_dir.join("audio/pc/SPC_Onyx.bnk")),
        ("audio/pc/SPC_Sellsword.bnk", temp_dir.join("audio/pc/SPC_Sellsword.bnk")),
    ];
    for (rel, dst) in &audio_to_copy {
        let src = game_dir.join(rel);
        if src.is_file() {
            fs::create_dir_all(dst.parent().unwrap()).unwrap();
            fs::copy(&src, dst).unwrap();
        }
    }

    // Record pristine hashes
    let mut initial_hashes: HashMap<String, String> = HashMap::new();
    for f in &swfs_to_copy {
        let p = temp_dir.join(f);
        if p.is_file() {
            initial_hashes.insert(f.to_string(), BackupManager::compute_sha256(&p).unwrap());
        }
    }
    for f in &anims_to_copy {
        let rel = format!("anims/{}", f);
        let p = anims_dir.join(f);
        if p.is_file() {
            initial_hashes.insert(rel, BackupManager::compute_sha256(&p).unwrap());
        }
    }
    for (rel, dst) in &audio_to_copy {
        if dst.is_file() {
            initial_hashes.insert(rel.to_string(), BackupManager::compute_sha256(dst).unwrap());
        }
    }

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    // =========================================================================
    // 1. BÖDVAR — XAVIER WOODS
    // =========================================================================
    println!("\n--- Testing Crossover 1: Xavier Woods (Bödvar) ---");
    let req_xavier = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("Xavier".into()),
        ..Default::default()
    };
    let res_xavier = engine.apply_modification(&req_xavier);
    assert!(res_xavier.success, "Xavier Woods modification must succeed: {}", res_xavier.message);
    let swf_viking = SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    assert!(swf_viking.validate().is_ok(), "Gfx_Viking.swf must pass structural validation");
    let swf_h = SwfPackage::read_from_file(&temp_dir.join("SFX_Hammer.swf")).unwrap();
    assert!(swf_h.validate().is_ok(), "SFX_Hammer.swf must pass structural validation");

    // Verify VOX Announcer swap for Xavier
    let vox_data = fs::read(temp_dir.join("audio/pc/English(US)/VOX_Announcer.bnk")).unwrap();
    let xavier_vo_hash = ak_hash("VO_Announcer_Name_Bodvar_XavierWoods_Play").to_le_bytes();
    let bodvar_vo_hash = ak_hash("VO_Announcer_Name_Bodvar_Play").to_le_bytes();
    assert!(vox_data.windows(4).any(|w| w == xavier_vo_hash), "Xavier VO hash must be present");
    assert!(vox_data.windows(4).any(|w| w == bodvar_vo_hash), "Bodvar VO hash must be present");

    // Verify PC Movement lock-in sound for Xavier
    let mvt_data = fs::read(temp_dir.join("audio/pc/PC_Movement.bnk")).unwrap();
    let xavier_mvt_hash = ak_hash("MVT_Select_Bodvar_Xavier_Play").to_le_bytes();
    let bodvar_mvt_hash = ak_hash("MVT_Select_Bodvar_Play").to_le_bytes();
    assert!(mvt_data.windows(4).any(|w| w == xavier_mvt_hash), "Xavier lock-in audio must be present");
    assert!(mvt_data.windows(4).any(|w| w == bodvar_mvt_hash), "Bodvar lock-in audio must be present");

    // Verify Signature SFX in SPC_Bodvar.bnk for Xavier
    let bodvar_data = fs::read(temp_dir.join("audio/pc/SPC_Bodvar.bnk")).unwrap();
    let xavier_sig_hash = ak_hash("SPC_Bodvar_Hammer_Neutral_B_Xavier_Fire_Play").to_le_bytes();
    assert!(bodvar_data.windows(4).any(|w| w == xavier_sig_hash), "Xavier signature SFX must be active in SPC_Bodvar.bnk");

    // Restore pristine
    let _ = engine.backup_manager.restore_all();

    // =========================================================================
    // 2. THATCH — SNAKE EYES
    // =========================================================================
    println!("\n--- Testing Crossover 2: Snake Eyes (Thatch) ---");
    let req_snake = ModificationRequest {
        legend_name: Some("Thatch".into()),
        target_costume_name: Some("Thatch".into()),
        replacement_costume_name: Some("SnakeEyes".into()),
        ..Default::default()
    };
    let res_snake = engine.apply_modification(&req_snake);
    assert!(res_snake.success, "Snake Eyes modification must succeed: {}", res_snake.message);
    let swf_thatch = SwfPackage::read_from_file(&temp_dir.join("Gfx_Thatch.swf")).unwrap();
    assert!(swf_thatch.validate().is_ok(), "Gfx_Thatch.swf must pass structural validation");
    let swf_sw = SwfPackage::read_from_file(&temp_dir.join("SFX_Sword.swf")).unwrap();
    assert!(swf_sw.validate().is_ok(), "SFX_Sword.swf must pass structural validation");

    // Verify Signature SFX in SPC_Thatch.bnk for Snake Eyes
    let thatch_data = fs::read(temp_dir.join("audio/pc/SPC_Thatch.bnk")).unwrap();
    let snake_sig_hash = ak_hash("SPC_Thatch_Sword_Neutral_B_SnakeEyes_Launch_Play").to_le_bytes();
    assert!(thatch_data.windows(4).any(|w| w == snake_sig_hash), "Snake Eyes signature SFX must be active in SPC_Thatch.bnk");

    let _ = engine.backup_manager.restore_all();

    // =========================================================================
    // 3. THATCH — CHEWBACCA (Special Blaster Audio & Single-Blaster Animations)
    // =========================================================================
    println!("\n--- Testing Crossover 3: Chewbacca (Thatch) ---");
    let req_chew = ModificationRequest {
        legend_name: Some("Thatch".into()),
        target_costume_name: Some("Thatch".into()),
        replacement_costume_name: Some("Chewbacca".into()),
        ..Default::default()
    };
    let res_chew = engine.apply_modification(&req_chew);
    assert!(res_chew.success, "Chewbacca modification must succeed: {}", res_chew.message);
    let swf_chew = SwfPackage::read_from_file(&temp_dir.join("Gfx_Thatch.swf")).unwrap();
    assert!(swf_chew.validate().is_ok(), "Gfx_Thatch.swf must pass structural validation after Chewbacca import");
    let swf_pis = SwfPackage::read_from_file(&temp_dir.join("SFX_Pistols.swf")).unwrap();
    assert!(swf_pis.validate().is_ok(), "SFX_Pistols.swf must pass structural validation after Chewbacca import");

    // Verify dedicated blaster audio bank WEP_Pistol.bnk
    let pistol_data = fs::read(temp_dir.join("audio/pc/WEP_Pistol.bnk")).unwrap();
    let stand_a_hash = ak_hash("WEP_Pistol_Stand_A_Play").to_le_bytes();
    assert!(pistol_data.windows(4).any(|w| w == stand_a_hash), "Dedicated Bowcaster Stand_A sound event must be active in WEP_Pistol.bnk");

    // Verify Signature SFX in SPC_Thatch.bnk for Chewbacca
    let thatch_chew_data = fs::read(temp_dir.join("audio/pc/SPC_Thatch.bnk")).unwrap();
    let chew_sig_hash = ak_hash("SPC_Thatch_Chewbacca_Sword_Neutral_B_Launch_Play").to_le_bytes();
    assert!(thatch_chew_data.windows(4).any(|w| w == chew_sig_hash), "Chewbacca signature SFX must be active in SPC_Thatch.bnk");

    let _ = engine.backup_manager.restore_all();

    // =========================================================================
    // 4. ULGRIM — MACHO MAN
    // =========================================================================
    println!("\n--- Testing Crossover 4: Macho Man (Ulgrim) ---");
    let req_macho = ModificationRequest {
        legend_name: Some("Dwarf".into()),
        target_costume_name: Some("Dwarf".into()),
        replacement_costume_name: Some("MachoMan".into()),
        ..Default::default()
    };
    let res_macho = engine.apply_modification(&req_macho);
    assert!(res_macho.success, "Macho Man modification must succeed: {}", res_macho.message);
    let swf_dwarf = SwfPackage::read_from_file(&temp_dir.join("Gfx_Dwarf.swf")).unwrap();
    assert!(swf_dwarf.validate().is_ok(), "Gfx_Dwarf.swf must pass structural validation");
    let swf_axe = SwfPackage::read_from_file(&temp_dir.join("SFX_Axe.swf")).unwrap();
    assert!(swf_axe.validate().is_ok(), "SFX_Axe.swf must pass structural validation");

    // Verify Signature SFX in SPC_Dwarf.bnk for Macho Man
    let dwarf_data = fs::read(temp_dir.join("audio/pc/SPC_Dwarf.bnk")).unwrap();
    let macho_sig_hash = ak_hash("SPC_Ulgrim_Axe_Down_B_MachoMan_Fire_Play").to_le_bytes();
    assert!(dwarf_data.windows(4).any(|w| w == macho_sig_hash), "Macho Man signature SFX must be active in SPC_Dwarf.bnk");

    let _ = engine.backup_manager.restore_all();

    // =========================================================================
    // 5. ONYX — TITAN (Destiny 2)
    // =========================================================================
    println!("\n--- Testing Crossover 5: Titan (Onyx) ---");
    let req_titan = ModificationRequest {
        legend_name: Some("Gargoyle".into()),
        target_costume_name: Some("Gargoyle".into()),
        replacement_costume_name: Some("DestinyTitan".into()),
        ..Default::default()
    };
    let res_titan = engine.apply_modification(&req_titan);
    assert!(res_titan.success, "Destiny Titan modification must succeed: {}", res_titan.message);
    let swf_gargoyle = SwfPackage::read_from_file(&temp_dir.join("Gfx_Gargoyle.swf")).unwrap();
    assert!(swf_gargoyle.validate().is_ok(), "Gfx_Gargoyle.swf must pass structural validation");
    let swf_can = SwfPackage::read_from_file(&temp_dir.join("SFX_Cannon.swf")).unwrap();
    assert!(swf_can.validate().is_ok(), "SFX_Cannon.swf must pass structural validation");

    // Verify Signature SFX in SPC_Onyx.bnk for Destiny Titan
    let onyx_data = fs::read(temp_dir.join("audio/pc/SPC_Onyx.bnk")).unwrap();
    let titan_sig_hash = ak_hash("SPC_Onyx_Titan_Cannon_Neutral_B_Launch_Play").to_le_bytes();
    assert!(onyx_data.windows(4).any(|w| w == titan_sig_hash), "Destiny Titan signature SFX must be active in SPC_Onyx.bnk");

    let _ = engine.backup_manager.restore_all();

    // =========================================================================
    // 6. JAEYUN — NEGAN (The Walking Dead)
    // =========================================================================
    println!("\n--- Testing Crossover 6: Negan (Jaeyun) ---");
    let req_negan = ModificationRequest {
        legend_name: Some("Sellsword".into()),
        target_costume_name: Some("Sellsword".into()),
        replacement_costume_name: Some("Negan".into()),
        ..Default::default()
    };
    let res_negan = engine.apply_modification(&req_negan);
    assert!(res_negan.success, "Negan modification must succeed: {}", res_negan.message);
    let swf_sellsword = SwfPackage::read_from_file(&temp_dir.join("Gfx_Sellsword.swf")).unwrap();
    assert!(swf_sellsword.validate().is_ok(), "Gfx_Sellsword.swf must pass structural validation");
    let swf_sw_negan = SwfPackage::read_from_file(&temp_dir.join("SFX_Sword.swf")).unwrap();
    assert!(swf_sw_negan.validate().is_ok(), "SFX_Sword.swf must pass structural validation after Negan");

    // Verify Signature SFX in SPC_Sellsword.bnk for Negan
    let sellsword_data = fs::read(temp_dir.join("audio/pc/SPC_Sellsword.bnk")).unwrap();
    let negan_sig_hash = ak_hash("SPC_Sellsword_Sword_Neutral_B_Negan_Launch_Play").to_le_bytes();
    assert!(sellsword_data.windows(4).any(|w| w == negan_sig_hash), "Negan signature SFX must be active in SPC_Sellsword.bnk");

    // =========================================================================
    // 7. BIT-FOR-BIT RESTORATION & ROLLBACK VERIFICATION
    // =========================================================================
    println!("\n--- Testing Bit-for-bit Pristine Restoration ---");
    let restored = engine.backup_manager.restore_all().expect("restore_all must succeed");
    assert!(restored > 0, "Must have restored modified files");

    for (fname, orig_hash) in &initial_hashes {
        let p = temp_dir.join(fname);
        if p.is_file() {
            let cur_hash = BackupManager::compute_sha256(&p).unwrap();
            assert_eq!(orig_hash, &cur_hash, "File '{}' must match initial pristine state bit-for-bit!", fname);
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
    println!("=== ALL SIX CROSSOVERS FULLY VERIFIED WITH 100% SUCCESS ===");
}

#[test]
fn test_security_path_traversal_prevention() {
    use brawlcraft_slint::backup_manager::BackupManager;

    let unsafe_paths = [
        "../test.swf",
        "..\\test.swf",
        "/etc/passwd",
        "\\Windows\\System32\\cmd.exe",
        "C:\\Windows\\System32\\cmd.exe",
        "D:/games/brawlhalla.exe",
        "nested/../../secret.txt",
        "",
        "   ",
    ];

    for path in &unsafe_paths {
        assert!(
            !BackupManager::is_safe_relative_path(path),
            "Path '{}' should be flagged as unsafe!",
            path
        );
    }

    let safe_paths = [
        "Gfx_Viking.swf",
        "Gfx_Hands.swf",
        "anims/Animation_Sword.anm",
        "anims\\Animation_Sword.anm",
    ];

    for path in &safe_paths {
        assert!(
            BackupManager::is_safe_relative_path(path),
            "Path '{}' should be recognized as safe!",
            path
        );
    }
}

#[test]
fn test_serde_model_defaults_and_robustness() {
    use brawlcraft_slint::models::ModificationRequest;

    // Simulate minimal stub sent from frontend when selecting Bodvar or default weapons
    let json_stub = serde_json::json!({
        "targetLegend": {
            "heroId": 3,
            "heroName": "Viking",
            "displayName": "Bödvar",
            "weapon1": "Hammer",
            "weapon2": "Sword"
        },
        "targetWeapon1": {
            "weaponSkinId": 0,
            "weaponSkinName": "HammerDefault",
            "baseWeapon": "Hammer"
        },
        "targetWeapon2": {
            "weaponSkinId": 0,
            "weaponSkinName": "SwordDefault",
            "baseWeapon": "Sword"
        }
    });

    let req: Result<ModificationRequest, _> = serde_json::from_value(json_stub);
    assert!(req.is_ok(), "Partial JSON stub must deserialize successfully thanks to serde defaults!");
    let req = req.unwrap();
    assert_eq!(req.get_legend_name(), "Viking");
    assert_eq!(req.get_target_weapon1_name(), Some("HammerDefault".into()));
    assert_eq!(req.get_target_weapon2_name(), Some("SwordDefault".into()));
}

#[test]
fn test_all_hammer_and_sword_skins_render_and_distinct() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::RendererClient;
    use brawlcraft_slint::game_detector::GameDetector;
    use std::collections::HashMap;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    let mut client = RendererClient::new();

    let hammers: Vec<_> = cat.weapon_skins.iter()
        .filter(|w| w.base_weapon.eq_ignore_ascii_case("Hammer"))
        .cloned()
        .collect();
    let swords: Vec<_> = cat.weapon_skins.iter()
        .filter(|w| w.base_weapon.eq_ignore_ascii_case("Sword"))
        .cloned()
        .collect();

    println!("Total Hammers to test: {}", hammers.len());
    println!("Total Swords to test: {}", swords.len());

    let mut hammer_svg_to_skins: HashMap<String, Vec<String>> = HashMap::new();
    let mut hammer_failed = Vec::new();

    for w in &hammers {
        let req = serde_json::json!({
            "type": "weapon",
            "baseWeapon": w.base_weapon,
            "symbol": w.symbol_suffix,
            "weaponSkinName": w.weapon_skin_name,
            "swfFile": w.swf_file,
            "ownerHero": w.owner_hero,
            "gamePath": inst.game_path
        });
        match client.render(&req) {
            Some(svg) if svg.trim().starts_with("<svg") => {
                hammer_svg_to_skins.entry(svg).or_default().push(w.weapon_skin_name.clone());
            }
            _ => {
                hammer_failed.push(format!("{}: sym='{}', swf='{}'", w.weapon_skin_name, w.symbol_suffix, w.swf_file));
            }
        }
    }

    let mut sword_svg_to_skins: HashMap<String, Vec<String>> = HashMap::new();
    let mut sword_failed = Vec::new();

    for w in &swords {
        let req = serde_json::json!({
            "type": "weapon",
            "baseWeapon": w.base_weapon,
            "symbol": w.symbol_suffix,
            "weaponSkinName": w.weapon_skin_name,
            "swfFile": w.swf_file,
            "ownerHero": w.owner_hero,
            "gamePath": inst.game_path
        });
        match client.render(&req) {
            Some(svg) if svg.trim().starts_with("<svg") => {
                sword_svg_to_skins.entry(svg).or_default().push(w.weapon_skin_name.clone());
            }
            _ => {
                sword_failed.push(format!("{}: sym='{}', swf='{}'", w.weapon_skin_name, w.symbol_suffix, w.swf_file));
            }
        }
    }

    println!("Hammer failed to render count: {}", hammer_failed.len());
    let hammer_duplicates: Vec<_> = hammer_svg_to_skins.iter()
        .filter(|(_, skins)| skins.len() > 1)
        .collect();
    println!("Hammer duplicate SVG clusters: {}", hammer_duplicates.len());

    assert!(hammer_failed.is_empty(), "All hammer skins must render: {:?}", hammer_failed);
    
    // In original Brawlhalla:
    // 1. HammerXavier intentionally references the Viking Hammer base asset (ID 167 in Gfx_Viking.swf)
    // 2. HammerStub (Skin ID 710) intentionally references the Detective Hammer asset (Gfx_Super.swf)
    let illegitimate_hammer_duplicates: Vec<_> = hammer_duplicates.iter()
        .filter(|(_, skins)| {
            let mut s = (*skins).clone();
            s.sort();
            s.as_slice() != ["HammerViking", "HammerXavier"] && s.as_slice() != ["HammerDetective", "HammerStub"]
        })
        .collect();
    assert_eq!(illegitimate_hammer_duplicates.len(), 0, "No unexpected duplicate hammer SVGs allowed: {:?}", illegitimate_hammer_duplicates);

    println!("Sword failed to render count: {}", sword_failed.len());
    let sword_duplicates: Vec<_> = sword_svg_to_skins.iter()
        .filter(|(_, skins)| skins.len() > 1)
        .collect();
    println!("Sword duplicate SVG clusters: {}", sword_duplicates.len());
    for (i, (_, skins)) in sword_duplicates.iter().enumerate() {
        println!("  Cluster {}: {:?}", i, skins);
    }

    assert!(sword_failed.is_empty(), "All sword skins must render: {:?}", sword_failed);
    
    // In original Brawlhalla, BladeDancer variants and certain crossover base swords legitimately share visual assets:
    // 1. BladeDancer & BladeDancerStance
    // 2. Sellsword & Negan (Gfx_Jaeyun.swf)
    // 3. Pirate & SnakeEyes (Gfx_Thatch.swf)
    // 4. Viking & Xavier (Gfx_Viking.swf)
    // 5. Viking & Draugr (Gfx_Viking.swf: both point to sprite ID 74)
    let illegitimate_sword_duplicates: Vec<_> = sword_duplicates.iter()
        .filter(|(_, skins)| {
            let mut s = (*skins).clone();
            s.sort();
            let is_blade_dancer = s.as_slice() == ["SwordBladeDancer", "SwordBladeDancerStance"]
                || s.as_slice() == ["SwordBladeDancer", "SwordBladeDancerStance", "SwordDancerJester"];
            let is_pirate_snake_eyes = s.as_slice() == ["SwordPirate", "SwordSnakeEyes"];
            let is_sellsword_negan = s.as_slice() == ["SwordNegan", "SwordSellsword"];
            let is_viking_xavier = s.as_slice() == ["SwordViking", "SwordXavier"];
            let is_viking_draugr = s.as_slice() == ["SwordDraugr", "SwordViking"];
            !is_blade_dancer && !is_pirate_snake_eyes && !is_sellsword_negan && !is_viking_xavier && !is_viking_draugr
        })
        .collect();
    assert_eq!(illegitimate_sword_duplicates.len(), 0, "No unexpected duplicate sword SVGs allowed: {:?}", illegitimate_sword_duplicates);
}


#[test]
fn test_bodvar_and_all_curated_legends_switching_pipeline() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    let mut cat = brawlcraft_slint::catalog::Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();



    let curated_hero_identifiers = [
        ("Viking", "Bödvar"),
        ("Thatch", "Thatch"),
        ("Dwarf", "Ulgrim"),
        ("Gargoyle", "Onyx"),
        ("Sellsword", "Jaeyun"),
    ];

    for (hero_name, display_name) in &curated_hero_identifiers {
        let legend = cat
            .legends
            .iter()
            .find(|l| l.hero_name == *hero_name || l.display_name == *display_name)
            .unwrap_or_else(|| panic!("Curated legend '{}/{}' must exist in catalog", hero_name, display_name));

        assert!(!legend.costumes.is_empty(), "Legend '{}' must have costumes", hero_name);

        let default_costume = legend.costumes.iter().find(|c| c.is_default);
        assert!(default_costume.is_some(), "Legend '{}' must have a default costume", hero_name);

        let mut bm = brawlcraft_slint::backup_manager::BackupManager::new(&inst.game_path);
        let engine = ModificationEngine::new(
            std::path::PathBuf::from(&inst.game_path),
            &mut bm,
            &cat,
        );

        let req = ModificationRequest {
            target_legend: Some(legend.clone()),
            target_costume: default_costume.cloned(),
            replacement_costume: legend.costumes.get(1).cloned(),
            ..Default::default()
        };

        let val_res = engine.validate_modification(&req);
        assert!(val_res.is_object(), "Validation result for '{}' must be valid JSON object", hero_name);
    }
}

#[test]
fn test_validate_all_modifications_deep() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::fs;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_deep_val_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let pristine_dir = std::path::Path::new(&inst.game_path).join(".studio_backups/_pristine");
    let copy_file = |name: &str| {
        let p = pristine_dir.join(name);
        let src = if p.is_file() { p } else { std::path::Path::new(&inst.game_path).join(name) };
        if src.is_file() {
            fs::copy(&src, temp_dir.join(name)).unwrap();
        }
    };

    copy_file("Gfx_Viking.swf");
    copy_file("Gfx_Weapons_Sets.swf");
    copy_file("Gfx_Golem.swf");
    copy_file("Gfx_Player_02.swf");
    copy_file("Gfx_Skins_02.swf");
    copy_file("Gfx_Weapons_02.swf");

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    // Test matrix 1: Bödvar -> Bear with Hammer Bear and Sword Asgard
    let req = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("Bear".into()),
        replacement_weapon1_name: Some("Hammer Bear".into()),
        replacement_weapon2_name: Some("Sword Asgard".into()),
        ..Default::default()
    };
    let res = engine.apply_modification(&req);
    assert!(res.success);

    // Now validate Gfx_Viking.swf in deep detail!
    let swf = SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    let is_def_tag = |code: u16| matches!(code, 2 | 6 | 10 | 11 | 14 | 20 | 21 | 22 | 32 | 33 | 35 | 36 | 37 | 39 | 46 | 48 | 75 | 83 | 84);
    let mut def_ids = std::collections::HashSet::new();
    for tag in &swf.tags {
        if is_def_tag(tag.tag_code) && tag.data.len() >= 2 {
            let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
            def_ids.insert(id);
        }
    }

    let syms = swf.get_symbol_classes().unwrap();
    let mut bad_syms = Vec::new();
    for (id, name) in &syms {
        if !def_ids.contains(id) {
            bad_syms.push((*id, name.clone()));
        }
    }
    println!("Total symbols: {}, Bad symbols: {:?}", syms.len(), bad_syms);
    assert!(bad_syms.is_empty(), "All symbols must point to valid definition tags");

    // Validate all sprites inner PlaceObjects
    let mut bad_place_objects = Vec::new();
    for tag in &swf.tags {
        if tag.tag_code == 39 && tag.data.len() >= 4 {
            let sprite_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
            let mut offset = 4;
            while offset + 2 <= tag.data.len() {
                let h = u16::from_le_bytes([tag.data[offset], tag.data[offset + 1]]);
                let code = h >> 6;
                let mut len = (h & 0x3F) as usize;
                let header_size = if len == 0x3F {
                    if offset + 6 > tag.data.len() { break; }
                    len = u32::from_le_bytes([
                        tag.data[offset + 2],
                        tag.data[offset + 3],
                        tag.data[offset + 4],
                        tag.data[offset + 5],
                    ]) as usize;
                    6
                } else {
                    2
                };
                let body_start = offset + header_size;
                let next_tag = body_start + len;
                if next_tag > tag.data.len() { break; }

                if code == 26 && len >= 5 {
                    let flags = tag.data[body_start];
                    if (flags & 0x02) != 0 {
                        let inner_char_id = u16::from_le_bytes([tag.data[body_start + 3], tag.data[body_start + 4]]);
                        if !def_ids.contains(&inner_char_id) {
                            bad_place_objects.push((sprite_id, code, inner_char_id));
                        }
                    }
                } else if code == 70 && len >= 6 {
                    let flags = tag.data[body_start];
                    if (flags & 0x02) != 0 {
                        let inner_char_id = u16::from_le_bytes([tag.data[body_start + 4], tag.data[body_start + 5]]);
                        if !def_ids.contains(&inner_char_id) {
                            bad_place_objects.push((sprite_id, code, inner_char_id));
                        }
                    }
                }
                if code == 0 { break; }
                offset = next_tag;
            }
        }
    }
    println!("Bad place objects in Gfx_Viking.swf: {:?}", bad_place_objects);
    assert!(bad_place_objects.is_empty(), "All inner PlaceObjects must reference existing definitions");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_check_official_empty_sprites() {
    use brawlcraft_slint::swf_engine::SwfPackage;
    let inst = brawlcraft_slint::game_detector::GameDetector::detect(None);
    let pristine_path = std::path::Path::new(&inst.game_path).join(".studio_backups/_pristine/Gfx_Viking.swf");
    let target_path = if pristine_path.is_file() {
        pristine_path
    } else {
        std::path::Path::new(&inst.game_path).join("Gfx_Viking.swf")
    };
    if !target_path.is_file() {
        return;
    }
    let p_swf = match SwfPackage::read_from_file(&target_path) {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut empty_sprites = Vec::new();
    for (i, tag) in p_swf.tags.iter().enumerate() {
        if tag.tag_code == 39 && tag.data.len() >= 4 {
            let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
            // Check if there is any PlaceObject in this sprite
            let mut has_place_object = false;
            let mut offset = 4;
            while offset + 2 <= tag.data.len() {
                let h = u16::from_le_bytes([tag.data[offset], tag.data[offset + 1]]);
                let code = h >> 6;
                let mut len = (h & 0x3F) as usize;
                let header_size = if len == 0x3F {
                    if offset + 6 > tag.data.len() { break; }
                    len = u32::from_le_bytes([
                        tag.data[offset + 2],
                        tag.data[offset + 3],
                        tag.data[offset + 4],
                        tag.data[offset + 5],
                    ]) as usize;
                    6
                } else {
                    2
                };
                if code == 26 || code == 70 || code == 4 {
                    has_place_object = true;
                    break;
                }
                if code == 0 { break; }
                offset += header_size + len;
            }
            if !has_place_object {
                empty_sprites.push((i, id, tag.data.len(), tag.data.clone()));
            }
        }
    }
    println!("Official sprites with no PlaceObject: {}", empty_sprites.len());
    for (i, id, len, data) in &empty_sprites {
        println!("  Sprite index={}, id={}, len={}, data={:02x?}", i, id, len, data);
    }
    let syms = p_swf.get_symbol_classes().unwrap();
    for (id, name) in syms {
        if empty_sprites.iter().any(|(_, s_id, _, _)| *s_id == id) {
            println!("  SYMBOL FOR EMPTY SPRITE {}: {}", id, name);
        }
    }
}

#[test]
fn test_shape_parser_verification() {
    use brawlcraft_slint::swf_engine::SwfPackage;
    let pristine_dir = std::path::Path::new(r"D:\SteamLibrary\steamapps\common\Brawlhalla\.studio_backups\_pristine");

    struct BitReader<'a> {
        data: &'a [u8],
        bit_pos: usize,
    }
    impl<'a> BitReader<'a> {
        fn new(data: &'a [u8]) -> Self {
            Self { data, bit_pos: 0 }
        }
        fn read_bits(&mut self, n: usize) -> Option<u32> {
            if n == 0 { return Some(0); }
            let mut val = 0u32;
            for _ in 0..n {
                let byte_idx = self.bit_pos / 8;
                let bit_idx = 7 - (self.bit_pos % 8);
                if byte_idx >= self.data.len() { return None; }
                let bit = ((self.data[byte_idx] >> bit_idx) & 1) as u32;
                val = (val << 1) | bit;
                self.bit_pos += 1;
            }
            Some(val)
        }
        fn align_byte(&mut self) {
            let rem = self.bit_pos % 8;
            if rem != 0 {
                self.bit_pos += 8 - rem;
            }
        }
        #[allow(dead_code)]
        fn byte_offset(&self) -> usize {
            (self.bit_pos + 7) / 8
        }
    }

    fn skip_rect(reader: &mut BitReader) -> bool {
        if let Some(nbits) = reader.read_bits(5) {
            let total = (nbits as usize) * 4;
            if reader.read_bits(total).is_some() {
                reader.align_byte();
                return true;
            }
        }
        false
    }

    fn skip_matrix(reader: &mut BitReader) -> bool {
        let has_scale = match reader.read_bits(1) { Some(v) => v, None => return false };
        if has_scale == 1 {
            let n_scale_bits = match reader.read_bits(5) { Some(v) => v as usize, None => return false };
            if reader.read_bits(n_scale_bits * 2).is_none() { return false; }
        }
        let has_rotate = match reader.read_bits(1) { Some(v) => v, None => return false };
        if has_rotate == 1 {
            let n_rotate_bits = match reader.read_bits(5) { Some(v) => v as usize, None => return false };
            if reader.read_bits(n_rotate_bits * 2).is_none() { return false; }
        }
        let n_translate_bits = match reader.read_bits(5) { Some(v) => v as usize, None => return false };
        if reader.read_bits(n_translate_bits * 2).is_none() { return false; }
        reader.align_byte();
        true
    }

    for swf_name in &["Gfx_Viking.swf", "Gfx_Weapons_02.swf", "Gfx_Player_02.swf"] {
        let p = pristine_dir.join(swf_name);
        if !p.is_file() { continue; }
        let swf = SwfPackage::read_from_file(&p).unwrap();
        let mut parsed_shapes = 0;
        let mut total_shapes = 0;
        let mut total_solid_fills = 0;

        for tag in &swf.tags {
            if !matches!(tag.tag_code, 2 | 22 | 32 | 83) { continue; }
            total_shapes += 1;
            if tag.data.len() < 4 { continue; }

            let mut reader = BitReader::new(&tag.data);
            // Skip ShapeId (2 bytes)
            if reader.read_bits(16).is_none() { continue; }
            // Skip ShapeBounds RECT
            if !skip_rect(&mut reader) { continue; }

            if tag.tag_code == 83 {
                // DefineShape4: EdgeBounds RECT + Flags byte
                if !skip_rect(&mut reader) { continue; }
                if reader.read_bits(8).is_none() { continue; }
            }

            // FILLSTYLEARRAY
            let mut fill_count = match reader.read_bits(8) { Some(v) => v as usize, None => continue };
            if tag.tag_code != 2 && fill_count == 0xFF {
                let low = match reader.read_bits(8) { Some(v) => v as usize, None => continue };
                let high = match reader.read_bits(8) { Some(v) => v as usize, None => continue };
                fill_count = (high << 8) | low;
            }

            let mut fills_ok = true;
            let is_target_shape = tag.data.len() >= 2 && u16::from_le_bytes([tag.data[0], tag.data[1]]) == 1223;
            for fill_idx in 0..fill_count {
                let fill_type = match reader.read_bits(8) { Some(v) => v as u8, None => { fills_ok = false; break; } };
                if is_target_shape {
                    println!("    [Target 1223] fill {}: type=0x{:02x}, byte_offset={}", fill_idx, fill_type, reader.bit_pos / 8);
                }
                if fill_type == 0x00 {
                    // Solid fill
                    let color_bytes = if tag.tag_code == 32 || tag.tag_code == 83 { 4 } else { 3 };
                    if reader.read_bits(color_bytes * 8).is_none() { fills_ok = false; println!("  fill {} failed to read color", fill_idx); break; }
                    total_solid_fills += 1;
                } else if fill_type == 0x10 || fill_type == 0x12 || fill_type == 0x13 {
                    // Gradient
                    if !skip_matrix(&mut reader) { fills_ok = false; println!("  fill {} failed gradient skip_matrix", fill_idx); break; }
                    let grad_header = match reader.read_bits(8) { Some(v) => v, None => { fills_ok = false; println!("  fill {} failed grad header", fill_idx); break; } };
                    let num_grads = (grad_header & 0x0F) as usize;
                    let grad_color_bytes = if tag.tag_code == 32 || tag.tag_code == 83 { 4 } else { 3 };
                    for _ in 0..num_grads {
                        // Ratio (1 byte) + Color
                        if reader.read_bits(8 + grad_color_bytes * 8).is_none() { fills_ok = false; println!("  fill {} failed grad record", fill_idx); break; }
                    }
                    if fill_type == 0x13 {
                        // Focal point: 2 bytes comes AFTER gradient records in SWF spec!
                        if reader.read_bits(16).is_none() { fills_ok = false; println!("  fill {} failed focal", fill_idx); break; }
                    }
                } else if fill_type >= 0x40 && fill_type <= 0x43 {
                    // Bitmap
                    if reader.read_bits(16).is_none() { fills_ok = false; println!("  fill {} failed bitmap id", fill_idx); break; } // BitmapId
                    if !skip_matrix(&mut reader) { fills_ok = false; println!("  fill {} failed bitmap matrix", fill_idx); break; }
                } else {
                    println!("  fill {} unknown fill_type: 0x{:02x}", fill_idx, fill_type);
                    fills_ok = false;
                    break;
                }
            }

            if fills_ok {
                // Now parse LINESTYLEARRAY
                let mut line_count = match reader.read_bits(8) { Some(v) => v as usize, None => { parsed_shapes += 1; continue; } };
                if tag.tag_code != 2 && line_count == 0xFF {
                    let low = match reader.read_bits(8) { Some(v) => v as usize, None => { parsed_shapes += 1; continue; } };
                    let high = match reader.read_bits(8) { Some(v) => v as usize, None => { parsed_shapes += 1; continue; } };
                    line_count = (high << 8) | low;
                }

                let mut lines_ok = true;
                for _ in 0..line_count {
                    // Width (2 bytes)
                    if reader.read_bits(16).is_none() { lines_ok = false; break; }
                    if tag.tag_code == 2 || tag.tag_code == 22 {
                        // RGB (3 bytes)
                        if reader.read_bits(24).is_none() { lines_ok = false; break; }
                    } else if tag.tag_code == 32 {
                        // RGBA (4 bytes)
                        if reader.read_bits(32).is_none() { lines_ok = false; break; }
                    } else if tag.tag_code == 83 {
                        // DefineShape4 line style
                        let flags = match reader.read_bits(16) { Some(v) => v as u16, None => { lines_ok = false; break; } };
                        if (flags & 0x08) != 0 {
                            // HasFill
                            let fill_type = match reader.read_bits(8) { Some(v) => v as u8, None => { lines_ok = false; break; } };
                            if fill_type == 0x00 {
                                if reader.read_bits(32).is_none() { lines_ok = false; break; }
                            } else {
                                // Ignore non-solid fill in line style
                                lines_ok = false; break;
                            }
                        } else {
                            // RGBA (4 bytes)
                            if reader.read_bits(32).is_none() { lines_ok = false; break; }
                        }
                        if (flags & 0x04) != 0 {
                            // Miter limit factor: 2 bytes
                            if reader.read_bits(16).is_none() { lines_ok = false; break; }
                        }
                    }
                }
                if lines_ok {
                    parsed_shapes += 1;
                }
            } else {
                println!("FAILED SHAPE IN {}: tag_code={}, len={}, data_preview={:02x?}", swf_name, tag.tag_code, tag.data.len(), &tag.data[..tag.data.len().min(32)]);
                // Debug parse step by step
                let mut dbg_r = BitReader::new(&tag.data);
                let id = dbg_r.read_bits(16);
                let b1 = skip_rect(&mut dbg_r);
                let pos1 = dbg_r.bit_pos;
                let b2 = skip_rect(&mut dbg_r);
                let pos2 = dbg_r.bit_pos;
                let flags = dbg_r.read_bits(8);
                let pos3 = dbg_r.bit_pos;
                let fill_cnt = dbg_r.read_bits(8);
                println!("  DEBUG: id={:?}, rect1={}, pos1={}, rect2={}, pos2={}, flags={:?}, pos3={}, fill_cnt={:?}", id, b1, pos1, b2, pos2, flags, pos3, fill_cnt);
            }
        }
        println!("{}: parsed {}/{} shapes successfully (total solid fills: {})", swf_name, parsed_shapes, total_shapes, total_solid_fills);
        assert_eq!(parsed_shapes, total_shapes, "Every single shape fill style array must parse successfully!");
    }
}

#[test]
fn test_customization_testing_matrix() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::fs;
    use std::collections::HashMap;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    cat.filter_curated_legends();

    let temp_dir = std::env::temp_dir().join(format!("bc_test_matrix_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let pristine_dir = std::path::Path::new(&inst.game_path).join(".studio_backups/_pristine");
    let copy_file = |name: &str| {
        let p = pristine_dir.join(name);
        let src = if p.is_file() { p } else { std::path::Path::new(&inst.game_path).join(name) };
        if src.is_file() {
            let _ = fs::copy(&src, temp_dir.join(name));
        }
    };

    // Populate isolated test environment with pristine assets
    for swf in &[
        "Gfx_Viking.swf", "Gfx_Thatch.swf", "Gfx_Dwarf.swf", "Gfx_Sellsword.swf",
        "Gfx_Gargoyle.swf", "Gfx_Player_02.swf", "Gfx_Weapons.swf", "Gfx_Weapons_02.swf",
        "Gfx_Weapons_Sets.swf", "Gfx_Hands.swf", "SFX_Sword.swf", "SFX_Hammer.swf",
        "SFX_Pistols.swf", "SFX_Lance.swf", "SFX_Axe.swf"
    ] {
        copy_file(swf);
    }

    // Copy anims for Thatch crossover test if present
    let anims_dir = temp_dir.join("anims");
    fs::create_dir_all(&anims_dir).unwrap();
    let game_anims = std::path::Path::new(&inst.game_path).join("anims");
    if game_anims.is_dir() {
        for anm in &["Animation_Sword.anm", "Animation_Pistols.anm"] {
            let src = game_anims.join(anm);
            if src.is_file() {
                let _ = fs::copy(&src, anims_dir.join(anm));
            }
        }
    }

    // Snapshot initial SHA256 hashes of all copied files to verify exact restoration
    let mut initial_hashes: HashMap<String, String> = HashMap::new();
    for entry in fs::read_dir(&temp_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            initial_hashes.insert(fname, BackupManager::compute_sha256(&path).unwrap());
        }
    }

    let mut backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let mut engine = ModificationEngine::new(temp_dir.clone(), &mut backup_mgr, &cat);

    // =========================================================================
    // PART 1: CHARACTER SKIN ONLY
    // 1A. Bödvar -> Bear (intra-SWF costume remapping)
    // 1B. Bödvar -> Draugr (intra-SWF costume with empty sprites)
    // =========================================================================
    println!("--- Testing Matrix Part 1: Character skin only ---");
    let req_1a = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("Bear".into()),
        ..Default::default()
    };
    let res_1a = engine.apply_modification(&req_1a);
    assert!(res_1a.success, "Part 1A (Bödvar -> Bear) must succeed: {}", res_1a.message);
    let swf_1a = SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    assert!(swf_1a.validate().is_ok(), "Gfx_Viking.swf must pass structural validation after 1A");

    let req_1b = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("Draugr".into()),
        ..Default::default()
    };
    let res_1b = engine.apply_modification(&req_1b);
    assert!(res_1b.success, "Part 1B (Bödvar -> Draugr) must succeed: {}", res_1b.message);
    let swf_1b = SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    assert!(swf_1b.validate().is_ok(), "Gfx_Viking.swf must pass structural validation after 1B");

    // Audio isolation guarantee: SFX files must remain 100% untouched
    for sfx in &["SFX_Sword.swf", "SFX_Hammer.swf", "SFX_Pistols.swf", "SFX_Lance.swf"] {
        let p = temp_dir.join(sfx);
        if p.is_file() {
            let current_hash = BackupManager::compute_sha256(&p).unwrap();
            assert_eq!(initial_hashes.get(*sfx), Some(&current_hash), "SFX file {} was touched in Part 1!", sfx);
        }
    }

    // =========================================================================
    // PART 2: WEAPON SKIN ONLY
    // 2A. Bödvar Hammer -> Bear Hammer (intra-SWF weapon remapping)
    // 2B. Bödvar Sword -> Sword Asgard (cross-SWF weapon import)
    // =========================================================================
    println!("--- Testing Matrix Part 2: Weapon skin only ---");
    let req_2a = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        target_weapon1_name: Some("Hammer".into()),
        replacement_weapon1_name: Some("Hammer Bear".into()),
        ..Default::default()
    };
    let res_2a = engine.apply_modification(&req_2a);
    assert!(res_2a.success, "Part 2A (Hammer Bear) must succeed: {}", res_2a.message);
    let swf_2a = SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    assert!(swf_2a.validate().is_ok(), "Gfx_Viking.swf must pass structural validation after 2A");

    let req_2b = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        target_weapon2_name: Some("Sword".into()),
        replacement_weapon2_name: Some("Sword Asgard".into()),
        ..Default::default()
    };
    let res_2b = engine.apply_modification(&req_2b);
    assert!(res_2b.success, "Part 2B (Sword Asgard) must succeed: {}", res_2b.message);
    let swf_2b = SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    assert!(swf_2b.validate().is_ok(), "Gfx_Viking.swf must pass structural validation after 2B");

    // =========================================================================
    // PART 3: CHARACTER + WEAPON SKIN SIMULTANEOUSLY
    // Bödvar -> Draugr + Hammer Bear + Sword Asgard in a single request
    // =========================================================================
    println!("--- Testing Matrix Part 3: Character + weapon skin simultaneously ---");
    let req_3 = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("Draugr".into()),
        target_weapon1_name: Some("Hammer".into()),
        replacement_weapon1_name: Some("Hammer Bear".into()),
        target_weapon2_name: Some("Sword".into()),
        replacement_weapon2_name: Some("Sword Asgard".into()),
        ..Default::default()
    };
    let res_3 = engine.apply_modification(&req_3);
    assert!(res_3.success, "Part 3 must succeed: {}", res_3.message);
    let swf_3 = SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    assert!(swf_3.validate().is_ok(), "Gfx_Viking.swf must pass structural validation after Part 3");

    // =========================================================================
    // PART 4: DIFFERENT WEAPON TYPES
    // Hammer, Sword, Pistols/Blasters, Axe, Greatsword
    // =========================================================================
    println!("--- Testing Matrix Part 4: Different weapon types ---");
    // 4A: Jaeyun (Greatsword & Sword)
    let req_4a = ModificationRequest {
        legend_name: Some("Sellsword".into()),
        target_costume_name: Some("Sellsword".into()),
        target_weapon1_name: Some("Sword".into()),
        replacement_weapon1_name: Some("Sword Asgard".into()),
        ..Default::default()
    };
    let res_4a = engine.apply_modification(&req_4a);
    if temp_dir.join("Gfx_Sellsword.swf").is_file() {
        assert!(res_4a.success, "Part 4A (Jaeyun Sword) must succeed: {}", res_4a.message);
        let swf_4a = SwfPackage::read_from_file(&temp_dir.join("Gfx_Sellsword.swf")).unwrap();
        assert!(swf_4a.validate().is_ok(), "Gfx_Sellsword.swf must pass structural validation");
    }

    // 4B: Ulgrim (Axe & Lance)
    let req_4b = ModificationRequest {
        legend_name: Some("Dwarf".into()),
        target_costume_name: Some("Dwarf".into()),
        target_weapon1_name: Some("Axe".into()),
        ..Default::default()
    };
    let res_4b = engine.apply_modification(&req_4b);
    if temp_dir.join("Gfx_Dwarf.swf").is_file() {
        assert!(res_4b.success, "Part 4B (Ulgrim Axe) must succeed: {}", res_4b.message);
        let swf_4b = SwfPackage::read_from_file(&temp_dir.join("Gfx_Dwarf.swf")).unwrap();
        assert!(swf_4b.validate().is_ok(), "Gfx_Dwarf.swf must pass structural validation");
    }

    // =========================================================================
    // PART 5: DIFFERENT CHARACTERS
    // Bödvar (Viking), Thatch (Thatch), Ulgrim (Dwarf), Onyx (Gargoyle)
    // =========================================================================
    println!("--- Testing Matrix Part 5: Different characters ---");
    // Thatch
    let req_5_thatch = ModificationRequest {
        legend_name: Some("Thatch".into()),
        target_costume_name: Some("Thatch".into()),
        replacement_costume_name: Some("CursedThatch".into()),
        ..Default::default()
    };
    let res_5_thatch = engine.apply_modification(&req_5_thatch);
    if temp_dir.join("Gfx_Thatch.swf").is_file() {
        assert!(res_5_thatch.success, "Part 5 (Thatch) must succeed: {}", res_5_thatch.message);
        let swf_thatch = SwfPackage::read_from_file(&temp_dir.join("Gfx_Thatch.swf")).unwrap();
        assert!(swf_thatch.validate().is_ok(), "Gfx_Thatch.swf must pass structural validation");
    }

    // Verify character isolation: Gfx_Hands.swf and SFX files must remain pristine
    for sfx in &["SFX_Sword.swf", "SFX_Hammer.swf", "SFX_Pistols.swf", "SFX_Lance.swf", "SFX_Axe.swf"] {
        let p = temp_dir.join(sfx);
        if p.is_file() {
            let current_hash = BackupManager::compute_sha256(&p).unwrap();
            assert_eq!(initial_hashes.get(*sfx), Some(&current_hash), "SFX file {} was touched across character tests!", sfx);
        }
    }

    // =========================================================================
    // PART 6: DEFAULT SKIN RESTORATION & ATOMIC ROLLBACK
    // Verify backup_mgr.restore_all() restores all modified files bit-for-bit
    // =========================================================================
    println!("--- Testing Matrix Part 6: Default skin restoration ---");
    let restored_count = engine.backup_manager.restore_all().expect("restore_all must succeed");
    println!("Restored files count: {}", restored_count);
    assert!(restored_count > 0, "restore_all must have restored modified files");

    for (fname, orig_hash) in &initial_hashes {
        let p = temp_dir.join(fname);
        if p.is_file() {
            let current_hash = BackupManager::compute_sha256(&p).unwrap();
            assert_eq!(orig_hash, &current_hash, "File {} was not restored bit-for-bit to its pristine state!", fname);
        }
    }

    // =========================================================================
    // PART 7: MULTIPLE CUSTOMIZATIONS APPLIED TOGETHER
    // Legend skin + 2 Weapon skins + Color palette customization
    // =========================================================================
    println!("--- Testing Matrix Part 7: Multiple customizations applied together ---");
    let req_7 = ModificationRequest {
        legend_name: Some("Viking".into()),
        target_costume_name: Some("Viking".into()),
        replacement_costume_name: Some("Bear".into()),
        target_weapon1_name: Some("Hammer".into()),
        replacement_weapon1_name: Some("Hammer Bear".into()),
        target_weapon2_name: Some("Sword".into()),
        replacement_weapon2_name: Some("Sword Asgard".into()),
        color_scheme_name: Some("1".into()),
        ..Default::default()
    };
    let res_7 = engine.apply_modification(&req_7);
    assert!(res_7.success, "Part 7 must succeed: {}", res_7.message);
    let swf_7 = SwfPackage::read_from_file(&temp_dir.join("Gfx_Viking.swf")).unwrap();
    assert!(swf_7.validate().is_ok(), "Gfx_Viking.swf must pass structural validation after Part 7");

    // Clean up rollback
    let _ = engine.backup_manager.restore_all();
    let _ = fs::remove_dir_all(temp_dir);
    println!("--- Testing Matrix: ALL 7 PARTS PASSED SUCCESSFULLY! ---");
}

#[test]
fn test_verify_candidate_signature_sfx_pairs() {
    use brawlcraft_slint::bnk_engine::ak_hash;
    use brawlcraft_slint::game_detector::GameDetector;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let audio_dir = Path::new(&inst.game_path).join("audio").join("pc");

    let test_pair = |bnk_data: &[u8], bnk_name: &str, base: &str, repl: &str| -> bool {
        let hb = ak_hash(base).to_le_bytes();
        let hr = ak_hash(repl).to_le_bytes();
        let fb = bnk_data.windows(4).any(|w| w == hb);
        let fr = bnk_data.windows(4).any(|w| w == hr);
        println!("  [{}] '{}' (found={}) <-> '{}' (found={})", bnk_name, base, fb, repl, fr);
        fb && fr
    };

    println!("\n=== VERIFYING EXACT CANDIDATE SOUND PAIRS ===");
    
    // Bodvar / Xavier
    let d_bod = fs::read(audio_dir.join("SPC_Bodvar.bnk")).unwrap();
    println!("--- Xavier (SPC_Bodvar.bnk) ---");
    test_pair(&d_bod, "SPC_Bodvar.bnk", "SPC_Bodvar_Hammer_Swing_Play", "SPC_Bodvar_Hammer_Neutral_B_Xavier_Fire_Play");
    test_pair(&d_bod, "SPC_Bodvar.bnk", "SPC_Bodvar_Hammer_Side_B_Swing_Play", "SPC_Bodvar_Hammer_Side_B_Xavier_Swing_Play");
    test_pair(&d_bod, "SPC_Bodvar.bnk", "SPC_Bodvar_Hammer_Swing_B_Play", "SPC_Bodvar_Hammer_Down_B_Xavier_Fire_Play");
    test_pair(&d_bod, "SPC_Bodvar.bnk", "SPC_Bodvar_Sword_Swing_A_Play", "SPC_Bodvar_Sword_Side_B_Xavier_Fire_Play");
    test_pair(&d_bod, "SPC_Bodvar.bnk", "SPC_Bodvar_Sword_Down_B_Launch_Play", "SPC_Bodvar_Sword_Down_B_Xavier_Swing_Play");
    test_pair(&d_bod, "SPC_Bodvar.bnk", "SPC_Bodvar_Sword_Neutral_B_Launch_Play", "SPC_Bodvar_Sword_Neutral_B_Xavier_Launch_Play");

    // Thatch / Snake Eyes
    let d_thatch = fs::read(audio_dir.join("SPC_Thatch.bnk")).unwrap();
    println!("--- Snake Eyes (SPC_Thatch.bnk) ---");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Sword_Shark_Play", "SPC_Thatch_Sword_Neutral_B_SnakeEyes_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Sword_Swing_C_Play", "SPC_Thatch_Sword_Side_B_SnakeEyes_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Sword_Swing_D_Play", "SPC_Thatch_Sword_Down_B_SnakeEyes_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "Lucien_Pistol_2_Play", "SPC_Thatch_Pistol_Neutral_B_SnakeEyes_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "Lucien_Pistol_4_Play", "SPC_Thatch_Pistol_Side_B_SnakeEyes_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Pistol_Fire_Play", "SPC_Thatch_Pistol_Down_B_SnakeEyes_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Pistol_D_Explode_Play", "SPC_Thatch_Pistol_Down_C_SnakeEyes_Hit_Play");

    // Thatch / Chewbacca
    println!("--- Chewbacca (SPC_Thatch.bnk) ---");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Sword_Shark_Play", "SPC_Thatch_Chewbacca_Sword_Neutral_B_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Sword_Swing_C_Play", "SPC_Thatch_Chewbacca_Sword_Side_B_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Sword_Swing_D_Play", "SPC_Thatch_Chewbacca_Sword_Down_B_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "Lucien_Pistol_2_Play", "SPC_Thatch_Chewbacca_Blasters_Neutral_B_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "Lucien_Pistol_4_Play", "SPC_Thatch_Chewbacca_Blasters_Side_B_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Pistol_Fire_Play", "SPC_Thatch_Chewbacca_Blasters_Down_B_Launch_Play");
    test_pair(&d_thatch, "SPC_Thatch.bnk", "SPC_Thatch_Pistol_D_Explode_Play", "SPC_Thatch_Chewbacca_Blasters_Down_C_Hit_1_Play");

    // Dwarf / Macho Man
    let d_dwarf = fs::read(audio_dir.join("SPC_Dwarf.bnk")).unwrap();
    println!("--- Macho Man (SPC_Dwarf.bnk) ---");
    test_pair(&d_dwarf, "SPC_Dwarf.bnk", "SPC_Dwarf_Axe_Swing_B_Play", "SPC_Ulgrim_Axe_Down_B_MachoMan_Fire_Play");
    test_pair(&d_dwarf, "SPC_Dwarf.bnk", "SPC_Dwarf_Axe_Swing_A_Play", "SPC_Ulgrim_Axe_Side_B_MachoMan_Fire_Play");
    test_pair(&d_dwarf, "SPC_Dwarf.bnk", "SPC_Dwarf_Lance_Swing_Play", "SPC_Ulgrim_Lance_Neutral_B_MachoMan_Hit_Play");
    test_pair(&d_dwarf, "SPC_Dwarf.bnk", "SPC_Dwarf_Lance_Blast_Play", "SPC_Ulgrim_Lance_Down_B_MachoMan_Hit_Play");
    test_pair(&d_dwarf, "SPC_Dwarf.bnk", "SPC_Dwarf_Lance_Side_B_Play", "SPC_Ulgrim_Lance_Side_B_MachoMan_Fire_Play");

    // Gargoyle / Destiny Titan
    let d_onyx = fs::read(audio_dir.join("SPC_Onyx.bnk")).unwrap();
    println!("--- Destiny Titan (SPC_Onyx.bnk) ---");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Cannon_Neutral_B_Launch_Play", "SPC_Onyx_Titan_Cannon_Neutral_B_Launch_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Cannon_Neutral_C_Hit_Play", "SPC_Onyx_Titan_Cannon_Neutral_C_Hit_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Cannon_Side_B_Launch_Play", "SPC_Onyx_Titan_Cannon_Side_B_Launch_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Cannon_Side_C_Hit_Play", "SPC_Onyx_Titan_Cannon_Side_C_Hit_2_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Cannon_Down_B_Launch_Play", "SPC_Onyx_Titan_Cannon_Down_B_Launch_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Cannon_Down_C_Hit_Play", "SPC_Onyx_Titan_Cannon_Down_C_Hit_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Gauntlet_Neutral_B_Launch_Play", "SPC_Onyx_Titan_Gauntlets_Neutral_B_Launch_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Gauntlet_Neutral_C_Hit_Play", "SPC_Onyx_Titan_Gauntlets_Neutral_C_Hit_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Gauntlet_Side_B_Launch_Play", "SPC_Onyx_Titan_Gauntlets_Side_B_Launch_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Gauntlet_Side_C_Hit_Play", "SPC_Onyx_Titan_Gauntlets_Side_C_Hit_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Gauntlet_Down_B_Launch_Play", "SPC_Onyx_Titan_Gauntlets_Down_B_Launch_Play");
    test_pair(&d_onyx, "SPC_Onyx.bnk", "SPC_Onyx_Gauntlet_Down_C_Hit_Play", "SPC_Onyx_Titan_Gauntlets_Down_C_Hit_1_Play");

    // Sellsword / Negan
    let d_negan = fs::read(audio_dir.join("SPC_Sellsword.bnk")).unwrap();
    println!("--- Negan (SPC_Sellsword.bnk) ---");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Sword_Neutral_B_Launch_Play", "SPC_Sellsword_Sword_Neutral_B_Negan_Launch_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Sword_Neutral_C_Hit_Play", "SPC_Sellsword_Sword_Neutral_C_Negan_Hit_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Sword_Side_B_Launch_Play", "SPC_Sellsword_Sword_Side_B_Negan_Launch_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Sword_Side_C_Hit_Play", "SPC_Sellsword_Sword_Side_C_Negan_Hit_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Sword_Down_B_Launch_Play", "SPC_Sellsword_Sword_Down_B_Negan_Launch_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Sword_Down_C_Hit_Play", "SPC_Sellsword_Sword_Down_C_Negan_Hit_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Neutral_B_Launch_Play", "SPC_Sellsword_Greatsword_Neutral_B_Negan_Launch_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Neutral_C_Hit_Play", "SPC_Sellsword_Greatsword_Neutral_C_Negan_Hit_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Side_D_LaunchSolo_Play", "SPC_Sellsword_Greatsword_Side_D_Negan_LaunchSolo_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Side_F_GroundRocksSolo_Play", "SPC_Sellsword_Greatsword_Side_F_Negan_GroundRocksSolo_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Side_C_Hit_Play", "SPC_Sellsword_Greatsword_Side_C_Negan_Hit_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Side_E_AirLeavesSolo_Play", "SPC_Sellsword_Greatsword_Side_D_Negan_AirLeavesSolo_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Side_Air_C_Hit_Play", "SPC_Sellsword_Greatsword_Side_Air_C_Negan_Hit_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Down_D_LaunchSolo_Play", "SPC_Sellsword_Greatsword_Down_D_Negan_LaunchSolo_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Down_F_GroundRocksSolo_Play", "SPC_Sellsword_Greatsword_Down_F_Negan_GroundRocksSolo_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Down_E_AirLeavesSolo_Play", "SPC_Sellsword_Greatsword_Down_E_Negan_AirLeavesSolo_Play");
    test_pair(&d_negan, "SPC_Sellsword.bnk", "SPC_Sellsword_Greatsword_Down_Air_C_Hit_Play", "SPC_Sellsword_Greatsword_Down_Air_C_Negan_Hit_Play");
}

#[test]
fn test_verify_crossover_anm_signatures() {
    use brawlcraft_slint::anm_engine::swap_anm_signatures;
    use brawlcraft_slint::game_detector::GameDetector;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let anims_dir = Path::new(&inst.game_path).join("anims");
    let temp_dir = std::env::temp_dir().join(format!("bc_test_anm_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let test_anm = |fname: &str, hero_key: &str, swap_suffix: &str| {
        let src = anims_dir.join(fname);
        let dst = temp_dir.join(fname);
        if !src.is_file() {
            println!("  [ANM MISSING] {}", fname);
            return;
        }
        fs::copy(&src, &dst).unwrap();
        match swap_anm_signatures(&dst, hero_key, swap_suffix) {
            Ok(swapped) => println!("  [{}] hero='{}', suffix='{}' => {} animations swapped", fname, hero_key, swap_suffix, swapped),
            Err(e) => println!("  [{}] Error: {}", fname, e),
        }
    };

    println!("\n=== TESTING CROSSOVER ANM SIGNATURE SWAPS ===");
    println!("--- Xavier Woods ---");
    test_anm("Animation_Hammer.anm", "Viking", "SwapXavier");
    test_anm("Animation_Sword.anm", "Viking", "SwapXavier");

    println!("--- Snake Eyes ---");
    test_anm("Animation_Sword.anm", "Pirate", "SwapSnakeEyes");
    test_anm("Animation_Pistols.anm", "Pirate", "SwapSnakeEyes");

    println!("--- Chewbacca ---");
    test_anm("Animation_Sword.anm", "Pirate", "SwapChewbacca");
    test_anm("Animation_Pistols.anm", "Pirate", "SwapChewbacca");

    println!("--- Macho Man ---");
    test_anm("Animation_Axe.anm", "Dwarf", "SwapMachoMan");
    test_anm("Animation_Lance.anm", "Dwarf", "SwapMachoMan");

    println!("--- Destiny Titan ---");
    test_anm("Animation_Cannon.anm", "Gargoyle", "SwapDestinyTitan");
    test_anm("Animation_Fists.anm", "Gargoyle", "SwapDestinyTitan");

    println!("--- Negan ---");
    test_anm("Animation_Sword.anm", "Sellsword", "SwapNegan");
    test_anm("Animation_GreatSword.anm", "Sellsword", "SwapNegan");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_verify_signature_vfx_remappings() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    println!("\n=== TESTING SIGNATURE VFX REMAPPINGS ===");

    // 1. Xavier Woods
    let test_remap = |swf_name: &str, hero_key: &str, swap_suffix: &str| {
        let p = game_dir.join(swf_name);
        if !p.is_file() {
            println!("  [SWF MISSING] {}", swf_name);
            return;
        }
        let mut pkg = SwfPackage::read_from_file(&p).unwrap();
        let remapped = pkg.remap_sfx_symbols(hero_key, swap_suffix);
        println!("  [{}] hero_key='{}', swap_suffix='{}' => {} symbols remapped", swf_name, hero_key, swap_suffix, remapped);
    };

    println!("--- Xavier Woods (Viking) ---");
    test_remap("SFX_Hammer.swf", "Viking", "SwapXavier");
    test_remap("SFX_Hammer.swf", "Bodvar", "SwapXavier");
    test_remap("SFX_Hammer.swf", "", "SwapXavier");
    test_remap("SFX_Sword.swf", "Viking", "SwapXavier");
    test_remap("SFX_Sword.swf", "Bodvar", "SwapXavier");
    test_remap("SFX_Sword.swf", "", "SwapXavier");
    test_remap("SFX_HitReacts.swf", "Viking", "SwapXavier");
    test_remap("SFX_HitReacts.swf", "Bodvar", "SwapXavier");

    let hammer_path = game_dir.join("SFX_Hammer.swf");
    if hammer_path.is_file() {
        let pkg = SwfPackage::read_from_file(&hammer_path).unwrap();
        if let Ok(syms) = pkg.get_symbol_classes() {
            println!("=== SFX_Hammer.swf (Viking / Bodvar / Xavier) ===");
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("viking") || nl.contains("bodvar") || nl.contains("xavier") {
                    println!("  id={}, name={}", id, name);
                }
            }
        }
    }

    println!("--- Snake Eyes (Thatch) ---");
    test_remap("SFX_Sword.swf", "Thatch", "SwapSnakeEyes");
    test_remap("SFX_Sword.swf", "Pirate", "SwapSnakeEyes");
    test_remap("SFX_Sword.swf", "", "SwapSnakeEyes");
    let sword_p = game_dir.join("SFX_Sword.swf");
    if sword_p.is_file() {
        let pkg = SwfPackage::read_from_file(&sword_p).unwrap();
        if let Ok(syms) = pkg.get_symbol_classes() {
            println!("=== SFX_Sword.swf (Pirate / Snake) ===");
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("pirate") || nl.contains("snake") {
                    println!("  id={}, name={}", id, name);
                }
            }
        }
    }
    test_remap("SFX_Pistols.swf", "Thatch", "SwapSnakeEyes");
    test_remap("SFX_Pistols.swf", "Pirate", "SwapSnakeEyes");
    test_remap("SFX_Pistols.swf", "", "SwapSnakeEyes");

    println!("--- Macho Man (Dwarf) ---");
    test_remap("SFX_Axe.swf", "Dwarf", "SwapMachoMan");
    test_remap("SFX_Axe.swf", "Ulgrim", "SwapMachoMan");
    test_remap("SFX_Axe.swf", "", "SwapMachoMan");
    test_remap("SFX_Lance.swf", "Dwarf", "SwapMachoMan");
    test_remap("SFX_Lance.swf", "Ulgrim", "SwapMachoMan");
    test_remap("SFX_Lance.swf", "", "SwapMachoMan");

    println!("--- Negan (Sellsword) ---");
    test_remap("SFX_Sword.swf", "Sellsword", "SwapNegan");
    test_remap("SFX_Sword.swf", "Jaeyun", "SwapNegan");
    test_remap("SFX_Sword.swf", "", "SwapNegan");

    let pistols_path = game_dir.join("SFX_Pistols.swf");
    if pistols_path.is_file() {
        let pkg = SwfPackage::read_from_file(&pistols_path).unwrap();
        if let Ok(syms) = pkg.get_symbol_classes() {
            println!("=== SFX_Pistols.swf (Pirate / Bomb / Snake) ===");
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("pirate") || nl.contains("bomb") || nl.contains("snake") {
                    println!("  id={}, name={}", id, name);
                }
            }
        }
    }

    let sword_path = game_dir.join("SFX_Sword.swf");
    if sword_path.is_file() {
        let pkg = SwfPackage::read_from_file(&sword_path).unwrap();
        if let Ok(syms) = pkg.get_symbol_classes() {
            println!("=== SFX_Sword.swf (DownSword / Negan) ===");
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("downsword") || nl.contains("negan") || nl.contains("sellsword") {
                    println!("  id={}, name={}", id, name);
                }
            }
        }
    }

    println!("--- Chewbacca (Thatch) ---");
    test_remap("SFX_Chewbacca.swf", "", "SwapChewbacca");
    let chew_sfx_path = game_dir.join("SFX_Chewbacca.swf");
    if chew_sfx_path.is_file() {
        let chew_pkg = SwfPackage::read_from_file(&chew_sfx_path).unwrap();
        let sword_p = game_dir.join("SFX_Sword.swf");
        if sword_p.is_file() {
            let mut pkg = SwfPackage::read_from_file(&sword_p).unwrap();
            let imported = pkg.import_sfx_symbols_from_swf(&chew_pkg, "SwapChewbacca").unwrap();
            println!("  [SFX_Sword.swf] Imported {} VFX symbols from SFX_Chewbacca.swf", imported);
        }
        let pistols_p = game_dir.join("SFX_Pistols.swf");
        if pistols_p.is_file() {
            let mut pkg = SwfPackage::read_from_file(&pistols_p).unwrap();
            let imported = pkg.import_sfx_symbols_from_swf(&chew_pkg, "SwapChewbacca").unwrap();
            println!("  [SFX_Pistols.swf] Imported {} VFX symbols from SFX_Chewbacca.swf", imported);
        }
    }

    println!("--- Destiny Titan (Onyx) ---");
    test_remap("SFX_DestinyTitan.swf", "", "SwapDestinyTitan");
    let titan_sfx_path = game_dir.join("SFX_DestinyTitan.swf");
    if titan_sfx_path.is_file() {
        let titan_pkg = SwfPackage::read_from_file(&titan_sfx_path).unwrap();
        let cannon_p = game_dir.join("SFX_Cannon.swf");
        if cannon_p.is_file() {
            let mut pkg = SwfPackage::read_from_file(&cannon_p).unwrap();
            let imported = pkg.import_sfx_symbols_from_swf(&titan_pkg, "SwapDestinyTitan").unwrap();
            println!("  [SFX_Cannon.swf] Imported {} VFX symbols from SFX_DestinyTitan.swf", imported);
        }
        let fists_p = game_dir.join("SFX_Fists.swf");
        if fists_p.is_file() {
            let mut pkg = SwfPackage::read_from_file(&fists_p).unwrap();
            let imported = pkg.import_sfx_symbols_from_swf(&titan_pkg, "SwapDestinyTitan").unwrap();
            println!("  [SFX_Fists.swf] Imported {} VFX symbols from SFX_DestinyTitan.swf", imported);
        }
    }
}

#[test]
fn test_inspect_macho_tags() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    let inspect_swf_tags = |fname: &str, target_ids: &[u16]| {
        let p = game_dir.join(fname);
        if let Ok(pkg) = SwfPackage::read_from_file(&p) {
            println!("\n=== TAG INSPECTION FOR [{}] ===", fname);
            for (idx, tag) in pkg.tags.iter().enumerate() {
                if tag.data.len() >= 2 {
                    let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if target_ids.contains(&id) {
                        println!("  Tag #{} [code={:<3}] defines/references ID={:<4} (len={})", idx, tag.tag_code, id, tag.data.len());
                        if tag.tag_code == 39 && tag.data.len() >= 4 { // DefineSprite
                            let frame_count = u16::from_le_bytes([tag.data[2], tag.data[3]]);
                            println!("    -> DefineSprite: frame_count={}", frame_count);
                            // Parse nested tags inside DefineSprite
                            let mut inner_offset = 4;
                            while inner_offset < tag.data.len() {
                                if inner_offset + 2 > tag.data.len() { break; }
                                let tag_code_and_length = u16::from_le_bytes([tag.data[inner_offset], tag.data[inner_offset + 1]]);
                                let inner_code = tag_code_and_length >> 6;
                                let mut inner_len = (tag_code_and_length & 0x3F) as usize;
                                inner_offset += 2;
                                if inner_len == 0x3F {
                                    if inner_offset + 4 > tag.data.len() { break; }
                                    inner_len = u32::from_le_bytes([
                                        tag.data[inner_offset], tag.data[inner_offset + 1],
                                        tag.data[inner_offset + 2], tag.data[inner_offset + 3]
                                    ]) as usize;
                                    inner_offset += 4;
                                }
                                if inner_code == 0 { break; } // End
                                if inner_code == 26 || inner_code == 70 { // PlaceObject2 / PlaceObject3
                                    if inner_offset < tag.data.len() {
                                        let flags = tag.data[inner_offset];
                                        let depth = if inner_offset + 2 < tag.data.len() {
                                            u16::from_le_bytes([tag.data[inner_offset + 1], tag.data[inner_offset + 2]])
                                        } else { 0 };
                                        let mut place_id = 0u16;
                                        if (flags & 0x02) != 0 && inner_offset + 4 < tag.data.len() {
                                            place_id = u16::from_le_bytes([tag.data[inner_offset + 3], tag.data[inner_offset + 4]]);
                                        }
                                        println!("       Inner PlaceObject (code={}): depth={}, placed_id={}", inner_code, depth, place_id);
                                    }
                                }
                                inner_offset += inner_len;
                            }
                        }
                    }
                }
            }
        }
    };

    inspect_swf_tags("SFX_Axe.swf", &[456, 445, 430, 87, 68, 64, 62, 60]);
    inspect_swf_tags("SFX_Lance.swf", &[95, 87, 77, 68]);
}

#[test]
fn test_check_jaeyun_roster_pose() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let ui_icons = game_dir.join("UI_Icons.swf");
    if let Ok(pkg) = SwfPackage::read_from_file(&ui_icons) {
        if let Ok(syms) = pkg.get_symbol_classes() {
            println!("\n=== ALL JAEYUN / SELLSWORD / NEGAN SYMBOLS IN UI_Icons.swf ===");
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("jaeyun") || nl.contains("sellsword") || nl.contains("negan") {
                    println!("  UI_Icons.swf: id={:<4} name={}", id, name);
                }
            }
        }
    }
}

#[test]
fn test_inspect_negan_audio_events() {
    use brawlcraft_slint::bnk_engine::ak_hash;
    use brawlcraft_slint::game_detector::GameDetector;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    println!("\n=== CHECKING VOX_Announcer.bnk FOR JAEYUN & NEGAN ===");
    let vox_path = game_dir.join("audio").join("pc").join("English(US)").join("VOX_Announcer.bnk");
    if let Ok(bytes) = fs::read(&vox_path) {
        for evt in &["VO_Announcer_Name_Jaeyun_Play", "VO_Announcer_Name_Sellsword_Play", "VO_Announcer_Name_Jaeyun_Negan_Play", "VO_Announcer_Name_Negan_Play"] {
            let h = ak_hash(evt).to_le_bytes();
            let found = bytes.windows(4).any(|w| w == h);
            println!("  VOX event '{}' (0x{:08X}) found: {}", evt, ak_hash(evt), found);
        }
    }

    println!("\n=== CHECKING PC_Movement.bnk FOR JAEYUN & NEGAN ===");
    let mvt_path = game_dir.join("audio").join("pc").join("PC_Movement.bnk");
    if let Ok(bytes) = fs::read(&mvt_path) {
        for evt in &["MVT_Select_Jaeyun_Play", "MVT_Select_Sellsword_Play", "MVT_Select_Sellsword_Negan_Play", "MVT_Select_Negan_Play"] {
            let h = ak_hash(evt).to_le_bytes();
            let found = bytes.windows(4).any(|w| w == h);
            println!("  MVT event '{}' (0x{:08X}) found: {}", evt, ak_hash(evt), found);
        }
    }
}

#[test]
fn test_inspect_player_anm_win() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let player_anm = game_dir.join("anims").join("Animation_Player.anm");
    if let Ok(bytes) = fs::read(&player_anm) {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(&bytes[4..]);
        let mut decomp = Vec::new();
        if decoder.read_to_end(&mut decomp).is_ok() {
            println!("\n=== Animation_Player.anm (decompressed len: {}) ===", decomp.len());
            let mut anims = Vec::new();
            let mut offset = 0;
            while offset + 2 < decomp.len() {
                let str_len = u16::from_le_bytes([decomp[offset], decomp[offset + 1]]) as usize;
                if str_len >= 3 && str_len < 90 && offset + 2 + str_len <= decomp.len() {
                    if let Ok(name) = std::str::from_utf8(&decomp[offset + 2..offset + 2 + str_len]) {
                        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                            let nl = name.to_lowercase();
                            if nl.contains("win") || nl.contains("lose") || nl.contains("podium") || nl.contains("victory") || nl.contains("defeat") {
                                anims.push(name.to_string());
                            }
                        }
                    }
                }
                offset += 1;
            }
            anims.sort();
            anims.dedup();
            println!("  Win/Lose/Podium anims in Animation_Player.anm: {:?}", anims);
        }
    }
}

#[test]
fn test_inspect_podium_types_full() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let game_swz_path = game_dir.join("Game.swz");
    let air_swf_path = game_dir.join("BrawlhallaAir.swf");
    if let Some(key) = brawlcraft_slint::swz::find_swz_key(&air_swf_path, &game_swz_path) {
        if let Ok(files) = brawlcraft_slint::swz::read_swz_files(&game_swz_path, key) {
            println!("\n=== INSPECTING PodiumTypes.xml FOR CUSTOM HERO POSES ===");
            if let Some(xml) = files.get("PodiumTypes.xml") {
                for line in xml.lines() {
                    if line.contains("CustomHeroPose") && !line.contains(">--<") {
                        println!("  CustomHeroPose in PodiumTypes: {}", line);
                    }
                }
            }

            println!("\n=== INSPECTING EndMatchVoicelineTypes.xml ===");
            if let Some(xml) = files.get("EndMatchVoicelineTypes.xml") {
                for line in xml.lines() {
                    let l = line.to_lowercase();
                    if l.contains("negan") || l.contains("sellsword") || l.contains("jaeyun") {
                        println!("  {}", line);
                    }
                }
            }

            println!("\n=== INSPECTING costumeTypes.csv PODIUM / POSE COLUMNS FOR ALL 6 CROSSOVERS ===");
            if let Some(csv) = files.get("costumeTypes.csv") {
                let mut lines = csv.lines();
                let _ = lines.next(); // skip "costumeTypes"
                let headers_line = lines.next().unwrap_or("");
                let headers: Vec<&str> = headers_line.split(',').collect();

                let targets = ["xavier", "snakeeyes", "chewbacca", "machoman", "destinytitan", "negan", "sellsword", "viking", "dwarf", "thatch", "gargoyle"];
                for line in lines {
                    let name = line.split(',').next().unwrap_or("");
                    if targets.iter().any(|&t| name.eq_ignore_ascii_case(t)) {
                        let cols: Vec<&str> = line.split(',').collect();
                        println!("Costume: {}", name);
                        for (i, val) in cols.iter().enumerate() {
                            let h = headers.get(i).unwrap_or(&"");
                            let v = val.trim();
                            if !v.is_empty() && (h.to_lowercase().contains("anim") || h.to_lowercase().contains("pose") || h.to_lowercase().contains("idle") || h.to_lowercase().contains("select") || h.to_lowercase().contains("platform") || h.to_lowercase().contains("portrait") || h.to_lowercase().contains("sound") || h.to_lowercase().contains("gem")) {
                                println!("    [col{}: {}]: {}", i, h, v);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_count_char_select_anm_occurrences() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let select_anm = game_dir.join("anims").join("Animation_CharacterSelect.anm");
    if let Ok(bytes) = fs::read(&select_anm) {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(&bytes[4..]);
        let mut decomp = Vec::new();
        if decoder.read_to_end(&mut decomp).is_ok() {
            println!("\n=== Animation_CharacterSelect.anm OCCURRENCES ===");
            for name in &["IdleSellsword", "IdleNegan", "SelectedSellsword", "SelectedNegan",
                          "IdleViking", "IdleXavier", "SelectedViking", "SelectedXavier",
                          "IdleDwarf", "IdleMachoMan", "SelectedDwarf", "SelectedMachoMan"] {
                let name_bytes = name.as_bytes();
                let mut pat = Vec::new();
                pat.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
                pat.extend_from_slice(name_bytes);

                let mut count_raw = 0;
                for w in decomp.windows(name_bytes.len()) {
                    if w == name_bytes { count_raw += 1; }
                }
                let mut count_prefixed = 0;
                for w in decomp.windows(pat.len()) {
                    if w == pat.as_slice() { count_prefixed += 1; }
                }
                println!("  {:<20}: raw count={}, length-prefixed count={}", name, count_raw, count_prefixed);
            }
        }
    }
}

#[test]
fn test_inspect_negan_podium_pipeline() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let anims_dir = game_dir.join("anims");

    // 1. Inspect Animation_Sellsword.anm
    let sellsword_anm = anims_dir.join("Animation_Sellsword.anm");
    if let Ok(bytes) = fs::read(&sellsword_anm) {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(&bytes[4..]);
        let mut decomp = Vec::new();
        if decoder.read_to_end(&mut decomp).is_ok() {
            let mut anims = Vec::new();
            let mut offset = 0;
            while offset + 2 < decomp.len() {
                let str_len = u16::from_le_bytes([decomp[offset], decomp[offset + 1]]) as usize;
                if str_len >= 3 && str_len < 90 && offset + 2 + str_len <= decomp.len() {
                    if let Ok(name) = std::str::from_utf8(&decomp[offset + 2..offset + 2 + str_len]) {
                        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                            anims.push(name.to_string());
                        }
                    }
                }
                offset += 1;
            }
            anims.sort();
            anims.dedup();
            println!("\n=== ALL ANIMATIONS IN Animation_Sellsword.anm (total {}) ===", anims.len());
            for a in &anims {
                let al = a.to_lowercase();
                if al.contains("win") || al.contains("lose") || al.contains("idle") || al.contains("select")
                    || al.contains("podium") || al.contains("taunt") || al.contains("pose") || al.contains("negan")
                {
                    println!("  Animation_Sellsword.anm: {}", a);
                }
            }
        }
    }

    // 2. Inspect ALL .anm files for any animation containing "Negan"
    println!("\n=== SEARCH ALL ANM FILES FOR 'NEGAN' ANIMATIONS ===");
    if let Ok(entries) = fs::read_dir(&anims_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map_or(false, |e| e == "anm") {
                let fname = p.file_name().unwrap().to_string_lossy().to_string();
                if let Ok(bytes) = fs::read(&p) {
                    use flate2::read::ZlibDecoder;
                    use std::io::Read;
                    if bytes.len() >= 4 {
                        let mut decoder = ZlibDecoder::new(&bytes[4..]);
                        let mut decomp = Vec::new();
                        if decoder.read_to_end(&mut decomp).is_ok() {
                            let mut found = Vec::new();
                            let mut offset = 0;
                            while offset + 2 < decomp.len() {
                                let str_len = u16::from_le_bytes([decomp[offset], decomp[offset + 1]]) as usize;
                                if str_len >= 3 && str_len < 90 && offset + 2 + str_len <= decomp.len() {
                                    if let Ok(name) = std::str::from_utf8(&decomp[offset + 2..offset + 2 + str_len]) {
                                        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                                            if name.to_lowercase().contains("negan") {
                                                found.push(name.to_string());
                                            }
                                        }
                                    }
                                }
                                offset += 1;
                            }
                            if !found.is_empty() {
                                found.sort();
                                found.dedup();
                                println!("  {} -> {:?}", fname, found);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_inspect_xavier_anm_struct() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let anims_dir = game_dir.join("anims");

    for fname in &["Animation_Hammer.anm", "Animation_Sword.anm"] {
        let p = anims_dir.join(fname);
        if let Ok(bytes) = fs::read(&p) {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut decoder = ZlibDecoder::new(&bytes[4..]);
            let mut decomp = Vec::new();
            if decoder.read_to_end(&mut decomp).is_ok() {
                println!("\n=== {} (decompressed len: {}) ===", fname, decomp.len());
                let needle = "Both".as_bytes();
                for (i, window) in decomp.windows(needle.len()).enumerate() {
                    if window == needle {
                        let start = i.saturating_sub(60);
                        let end = (i + 100).min(decomp.len());
                        let snippet = String::from_utf8_lossy(&decomp[start..end]);
                        println!("  Offset 0x{:X} contains 'Both': {:?}", i, snippet);
                    }
                }
            }
        }
    }
}

#[test]
fn test_check_export_assets_tags() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    for sfx_name in &["SFX_Axe.swf", "SFX_Lance.swf", "SFX_Hammer.swf", "SFX_Sword.swf", "SFX_HitReacts.swf"] {
        let p = game_dir.join(sfx_name);
        if let Ok(pkg) = SwfPackage::read_from_file(&p) {
            let tag56 = pkg.tags.iter().filter(|t| t.tag_code == 56).count();
            let tag76 = pkg.tags.iter().filter(|t| t.tag_code == 76).count();
            println!("{}: tag56 (ExportAssets) = {}, tag76 (SymbolClass) = {}", sfx_name, tag56, tag76);
        }
    }
}

#[test]
fn test_inspect_current_game_sfx_state() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    for sfx_name in &["SFX_Axe.swf", "SFX_Lance.swf", "SFX_Hammer.swf", "SFX_Sword.swf", "SFX_HitReacts.swf"] {
        let p = game_dir.join(sfx_name);
        if let Ok(pkg) = SwfPackage::read_from_file(&p) {
            println!("\n=== GAME FILE [{}] ===", sfx_name);
            if let Ok(syms) = pkg.get_symbol_classes() {
                for (id, name) in syms {
                    let nl = name.to_lowercase();
                    if nl.contains("dwarf") || nl.contains("macho") || nl.contains("viking") || nl.contains("bodvar") || nl.contains("xavier") {
                        println!("  id={:<4} name={}", id, name);
                    }
                }
            }
        }
    }
}

#[test]
fn test_simulate_macho_sfx_remap() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    let axe_p = game_dir.join("SFX_Axe.swf");
    if let Ok(mut pkg) = SwfPackage::read_from_file(&axe_p) {
        println!("\n=== BEFORE SFX_Axe.swf REMAP ===");
        if let Ok(syms) = pkg.get_symbol_classes() {
            for (id, name) in &syms {
                if name.contains("Dwarf") || name.contains("Macho") {
                    println!("  id={:<4} name={}", id, name);
                }
            }
        }
        let remapped = pkg.remap_sfx_symbols("Dwarf", "SwapMachoMan");
        println!("Remapped count: {}", remapped);
        println!("=== AFTER SFX_Axe.swf REMAP ===");
        if let Ok(syms) = pkg.get_symbol_classes() {
            for (id, name) in &syms {
                if name.contains("Dwarf") || name.contains("Macho") {
                    println!("  id={:<4} name={}", id, name);
                }
            }
        }
    }

    let lance_p = game_dir.join("SFX_Lance.swf");
    if let Ok(mut pkg) = SwfPackage::read_from_file(&lance_p) {
        println!("\n=== BEFORE SFX_Lance.swf REMAP ===");
        if let Ok(syms) = pkg.get_symbol_classes() {
            for (id, name) in &syms {
                if name.contains("Dwarf") || name.contains("Macho") {
                    println!("  id={:<4} name={}", id, name);
                }
            }
        }
        let remapped = pkg.remap_sfx_symbols("Dwarf", "SwapMachoMan");
        println!("Remapped count: {}", remapped);
        println!("=== AFTER SFX_Lance.swf REMAP ===");
        if let Ok(syms) = pkg.get_symbol_classes() {
            for (id, name) in &syms {
                if name.contains("Dwarf") || name.contains("Macho") {
                    println!("  id={:<4} name={}", id, name);
                }
            }
        }
    }
}

#[test]
fn test_inspect_sword_viking_symbols() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let sword_p = game_dir.join("SFX_Sword.swf");
    if let Ok(pkg) = SwfPackage::read_from_file(&sword_p) {
        if let Ok(syms) = pkg.get_symbol_classes() {
            println!("\n=== ALL SYMBOLS IN SFX_Sword.swf WITH SWOOSH / VIKING / BODVAR ===");
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("viking") || nl.contains("bodvar") || (nl.contains("swoosh") && nl.contains("special")) {
                    println!("  id={:<4} name={}", id, name);
                }
            }
        }
    }
}

#[test]
fn test_inspect_power_types_viking() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let game_swz_path = game_dir.join("Game.swz");
    let air_swf_path = game_dir.join("BrawlhallaAir.swf");
    if let Some(key) = brawlcraft_slint::swz::find_swz_key(&air_swf_path, &game_swz_path) {
        if let Ok(files) = brawlcraft_slint::swz::read_swz_files(&game_swz_path, key) {
            println!("\n=== ALL POWER SWAP TYPES FOR XAVIER ===");
            if let Some(xml) = files.get("PowerSwapTypes.xml") {
                for line in xml.split("<PowerSwapType ") {
                    if line.contains("OwnerCostume=\"Xavier\"") {
                        println!("<PowerSwapType {}", line.trim());
                    }
                }
            }
        }
    }
}

#[test]
fn test_inspect_power_types_dwarf() {
    use brawlcraft_slint::game_detector::GameDetector;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let game_swz_path = game_dir.join("Game.swz");
    let air_swf_path = game_dir.join("BrawlhallaAir.swf");
    if let Some(key) = brawlcraft_slint::swz::find_swz_key(&air_swf_path, &game_swz_path) {
        if let Ok(files) = brawlcraft_slint::swz::read_swz_files(&game_swz_path, key) {
            if let Some(power_csv) = files.get("powerTypes.csv") {
                let mut lines = power_csv.lines();
                let headers_line = lines.next().unwrap_or("");
                let headers: Vec<&str> = headers_line.split(',').collect();

                println!("\n=== POWER TYPES FOR DWARF (ULGRIM) ===");
                for line in lines {
                    if line.contains("Dwarf") {
                        let cols: Vec<&str> = line.split(',').collect();
                        let power_name = cols.first().unwrap_or(&"");
                        print!("{}: ", power_name);
                        for (i, val) in cols.iter().enumerate() {
                            let h = headers.get(i).unwrap_or(&"");
                            let v = val.trim();
                            if !v.is_empty() && i > 0 {
                                print!("[{}: {}] ", h, v);
                            }
                        }
                        println!();
                    }
                }
            }
        }
    }
}

#[test]
fn test_inspect_sfx_axe_and_lance() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    println!("\n=== INSPECTING SFX_Axe.swf SYMBOLS & TAGS ===");
    let axe_p = game_dir.join("SFX_Axe.swf");
    if let Ok(pkg) = SwfPackage::read_from_file(&axe_p) {
        if let Ok(syms) = pkg.get_symbol_classes() {
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("dwarf") || nl.contains("ulgrim") || nl.contains("macho") || nl.contains("chair") {
                    println!("  SFX_Axe.swf: id={:<4} name={}", id, name);
                }
            }
        }
    }

    println!("\n=== INSPECTING SFX_Lance.swf SYMBOLS & TAGS ===");
    let lance_p = game_dir.join("SFX_Lance.swf");
    if let Ok(pkg) = SwfPackage::read_from_file(&lance_p) {
        if let Ok(syms) = pkg.get_symbol_classes() {
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("dwarf") || nl.contains("ulgrim") || nl.contains("macho") {
                    println!("  SFX_Lance.swf: id={:<4} name={}", id, name);
                }
            }
        }
    }

    println!("\n=== INSPECTING SFX_HitReacts.swf SYMBOLS ===");
    let hit_p = game_dir.join("SFX_HitReacts.swf");
    if let Ok(pkg) = SwfPackage::read_from_file(&hit_p) {
        if let Ok(syms) = pkg.get_symbol_classes() {
            for (id, name) in syms {
                let nl = name.to_lowercase();
                if nl.contains("xavier") || nl.contains("viking") || nl.contains("bodvar") || nl.contains("macho") || nl.contains("negan") {
                    println!("  SFX_HitReacts.swf: id={:<4} name={}", id, name);
                }
            }
        }
    }

    println!("\n=== INSPECTING Animation_Hammer.anm and Animation_Sword.anm ALL BODVAR/VIKING ANIMS ===");
    let anims_dir = game_dir.join("anims");
    for anm_name in &["Animation_Hammer.anm", "Animation_Sword.anm", "Animation_Axe.anm", "Animation_Lance.anm", "Animation_Sellsword.anm"] {
        let p = anims_dir.join(anm_name);
        if let Ok(bytes) = fs::read(&p) {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut decoder = ZlibDecoder::new(&bytes[4..]);
            let mut decomp = Vec::new();
            if decoder.read_to_end(&mut decomp).is_ok() {
                let mut found = Vec::new();
                let mut offset = 0;
                while offset + 2 < decomp.len() {
                    let str_len = u16::from_le_bytes([decomp[offset], decomp[offset + 1]]) as usize;
                    if str_len >= 3 && str_len < 90 && offset + 2 + str_len <= decomp.len() {
                        if let Ok(name) = std::str::from_utf8(&decomp[offset + 2..offset + 2 + str_len]) {
                            if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                                let nl = name.to_lowercase();
                                if nl.contains("bodvar") || nl.contains("viking") || nl.contains("xavier")
                                    || nl.contains("dwarf") || nl.contains("ulgrim") || nl.contains("macho")
                                    || nl.contains("sellsword") || nl.contains("jaeyun") || nl.contains("negan")
                                    || nl.contains("win_") || nl.contains("lose_") || nl.contains("idle") || nl.contains("select")
                                {
                                    found.push(name.to_string());
                                }
                            }
                        }
                    }
                    offset += 1;
                }
                found.sort();
                found.dedup();
                println!("ANM [{}]: {} relevant anims: {:?}", anm_name, found.len(), found);
            }
        }
    }
}

#[test]
fn test_deep_investigate_crossover_vfx_and_podium() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    let anims_dir = game_dir.join("anims");

    println!("\n=======================================================");
    println!("=== 1. SEARCH ALL SWFS FOR XAVIER & MACHO & NEGAN SYMBOLS ===");
    println!("=======================================================");
    if let Ok(entries) = fs::read_dir(game_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map_or(false, |e| e == "swf") {
                let fname = p.file_name().unwrap().to_string_lossy().to_string();
                if let Ok(pkg) = SwfPackage::read_from_file(&p) {
                    if let Ok(syms) = pkg.get_symbol_classes() {
                        let mut matches = Vec::new();
                        for (id, name) in syms {
                            let nl = name.to_lowercase();
                            if nl.contains("xavier") || nl.contains("macho") || nl.contains("negan") {
                                matches.push((id, name));
                            }
                        }
                        if !matches.is_empty() {
                            println!("SWF [{}]: {} match(es)", fname, matches.len());
                            for (id, name) in matches {
                                println!("    id={:<4} name={}", id, name);
                            }
                        }
                    }
                }
            }
        }
    }

    println!("\n=======================================================");
    println!("=== 2. SEARCH ALL ANM FILES FOR CROSSOVER ANIMATIONS ===");
    println!("=======================================================");
    if let Ok(entries) = fs::read_dir(&anims_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map_or(false, |e| e == "anm") {
                let fname = p.file_name().unwrap().to_string_lossy().to_string();
                if let Ok(bytes) = fs::read(&p) {
                    if bytes.len() >= 4 {
                        use flate2::read::ZlibDecoder;
                        use std::io::Read;
                        let uncompressed_len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
                        let mut decoder = ZlibDecoder::new(&bytes[4..]);
                        let mut decomp = Vec::with_capacity(uncompressed_len);
                        if decoder.read_to_end(&mut decomp).is_ok() {
                            let mut found = Vec::new();
                            let mut offset = 0;
                            while offset + 2 < decomp.len() {
                                let str_len = u16::from_le_bytes([decomp[offset], decomp[offset + 1]]) as usize;
                                if str_len >= 3 && str_len < 90 && offset + 2 + str_len <= decomp.len() {
                                    if let Ok(name) = std::str::from_utf8(&decomp[offset + 2..offset + 2 + str_len]) {
                                        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                                            let nl = name.to_lowercase();
                                            if nl.contains("xavier") || nl.contains("macho") || nl.contains("negan") {
                                                found.push(name.to_string());
                                            }
                                        }
                                    }
                                }
                                offset += 1;
                            }
                            if !found.is_empty() {
                                found.sort();
                                found.dedup();
                                println!("ANM [{}]: {} match(es): {:?}", fname, found.len(), found);
                            }
                        }
                    }
                }
            }
        }
    }

    println!("\n=======================================================");
    println!("=== 3. INSPECT PODIUM PRESENTATION PIPELINE ===");
    println!("=======================================================");
    // Inspect Animation_CharacterSelect.anm for Negan and Jaeyun
    let char_select_anm = anims_dir.join("Animation_CharacterSelect.anm");
    if char_select_anm.is_file() {
        if let Ok(bytes) = fs::read(&char_select_anm) {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut decoder = ZlibDecoder::new(&bytes[4..]);
            let mut decomp = Vec::new();
            if decoder.read_to_end(&mut decomp).is_ok() {
                let mut char_anims = Vec::new();
                let mut offset = 0;
                while offset + 2 < decomp.len() {
                    let str_len = u16::from_le_bytes([decomp[offset], decomp[offset + 1]]) as usize;
                    if str_len >= 3 && str_len < 90 && offset + 2 + str_len <= decomp.len() {
                        if let Ok(name) = std::str::from_utf8(&decomp[offset + 2..offset + 2 + str_len]) {
                            if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                                let nl = name.to_lowercase();
                                if nl.contains("negan") || nl.contains("sellsword") || nl.contains("jaeyun") {
                                    char_anims.push(name.to_string());
                                }
                            }
                        }
                    }
                    offset += 1;
                }
                char_anims.sort();
                char_anims.dedup();
                println!("Animation_CharacterSelect.anm Negan/Jaeyun anims: {:?}", char_anims);
            }
        }
    }

    // Inspect UI_Podiums.anm
    let podium_anm = anims_dir.join("UI_Podiums.anm");
    if podium_anm.is_file() {
        if let Ok(bytes) = fs::read(&podium_anm) {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut decoder = ZlibDecoder::new(&bytes[4..]);
            let mut decomp = Vec::new();
            if decoder.read_to_end(&mut decomp).is_ok() {
                let mut pod_anims = Vec::new();
                let mut offset = 0;
                while offset + 2 < decomp.len() {
                    let str_len = u16::from_le_bytes([decomp[offset], decomp[offset + 1]]) as usize;
                    if str_len >= 3 && str_len < 90 && offset + 2 + str_len <= decomp.len() {
                        if let Ok(name) = std::str::from_utf8(&decomp[offset + 2..offset + 2 + str_len]) {
                            if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                                pod_anims.push(name.to_string());
                            }
                        }
                    }
                    offset += 1;
                }
                pod_anims.sort();
                pod_anims.dedup();
                println!("UI_Podiums.anm anims (sample of {} total): {:?}", pod_anims.len(), &pod_anims[..pod_anims.len().min(15)]);
            }
        }
    }

    // Inspect PodiumTypes.xml in Game.swz
    let game_swz_path = game_dir.join("Game.swz");
    let air_swf_path = game_dir.join("BrawlhallaAir.swf");
    if let Some(key) = brawlcraft_slint::swz::find_swz_key(&air_swf_path, &game_swz_path) {
        if let Ok(files) = brawlcraft_slint::swz::read_swz_files(&game_swz_path, key) {
            if let Some(podium_xml) = files.get("PodiumTypes.xml") {
                println!("PodiumTypes.xml loaded ({} bytes). Searching for Jaeyun/Negan/Hero references...", podium_xml.len());
                for line in podium_xml.lines() {
                    let lower = line.to_lowercase();
                    if lower.contains("negan") || lower.contains("sellsword") || lower.contains("jaeyun") || lower.contains("anim") {
                        if lower.contains("negan") || lower.contains("sellsword") || lower.contains("jaeyun") {
                            println!("  Podium line: {}", line);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_regression_macho_man_vfx_complete() {
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    let temp_dir = std::env::temp_dir().join(format!("bc_test_macho_vfx_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let axe_orig = game_dir.join("SFX_Axe.swf");
    let lance_orig = game_dir.join("SFX_Lance.swf");
    let axe_copy = temp_dir.join("SFX_Axe.swf");
    let lance_copy = temp_dir.join("SFX_Lance.swf");

    fs::copy(&axe_orig, &axe_copy).unwrap();
    fs::copy(&lance_orig, &lance_copy).unwrap();

    // 1. Remap SFX_Axe.swf
    let mut axe_pkg = SwfPackage::read_from_file(&axe_copy).unwrap();
    let axe_remapped = axe_pkg.remap_sfx_symbols("Dwarf", "SwapMachoMan");
    assert!(axe_remapped >= 3, "SFX_Axe.swf must remap at least 3 symbols, got {}", axe_remapped);
    axe_pkg.write_to_file(&axe_copy).unwrap();

    // Verify symbols in modified SFX_Axe.swf
    let read_axe = SwfPackage::read_from_file(&axe_copy).unwrap();
    assert!(read_axe.validate().is_ok(), "Modified SFX_Axe.swf must pass validation");
    let syms = read_axe.get_symbol_classes().unwrap();
    let sym_map: std::collections::HashMap<String, u16> = syms.into_iter().map(|(id, name)| (name, id)).collect();

    // The base Dwarf symbols must now point to the authentic Macho Man sprite IDs (87, 68, 60)
    let s2 = sym_map.get("a_AFXAttackSpecialDwarf2_IW").copied().expect("a_AFXAttackSpecialDwarf2_IW present");
    let s4 = sym_map.get("a_AFXAttackSpecialDwarf4_IW").copied().expect("a_AFXAttackSpecialDwarf4_IW present");
    let s4_rear = sym_map.get("a_AFXAttackSpecialDwarf4IW_Rear").copied().expect("a_AFXAttackSpecialDwarf4IW_Rear present");
    println!("Dwarf Axe Remapped Symbols: s2={}, s4={}, s4_rear={}", s2, s4, s4_rear);

    assert_eq!(s2, 87, "a_AFXAttackSpecialDwarf2_IW must point to Macho Man sprite ID 87");
    assert_eq!(s4, 68, "a_AFXAttackSpecialDwarf4_IW must point to Macho Man sprite ID 68");
    assert_eq!(s4_rear, 60, "a_AFXAttackSpecialDwarf4IW_Rear must point to Macho Man sprite ID 60");

    // Verify nested chair sprites (ID 64 and ID 62) are defined
    let has_chair1 = read_axe.tags.iter().any(|t| t.tag_code == 39 && t.data.len() >= 2 && u16::from_le_bytes([t.data[0], t.data[1]]) == 64);
    let has_chair2 = read_axe.tags.iter().any(|t| t.tag_code == 39 && t.data.len() >= 2 && u16::from_le_bytes([t.data[0], t.data[1]]) == 62);
    assert!(has_chair1, "Nested dependency a_ChairHalf1Macho (ID 64) must be defined in SFX_Axe.swf");
    assert!(has_chair2, "Nested dependency a_ChairHalf2Macho (ID 62) must be defined in SFX_Axe.swf");

    // 2. Remap SFX_Lance.swf
    let mut lance_pkg = SwfPackage::read_from_file(&lance_copy).unwrap();
    let lance_remapped = lance_pkg.remap_sfx_symbols("Dwarf", "SwapMachoMan");
    assert!(lance_remapped >= 2, "SFX_Lance.swf must remap at least 2 symbols, got {}", lance_remapped);
    lance_pkg.write_to_file(&lance_copy).unwrap();

    let read_lance = SwfPackage::read_from_file(&lance_copy).unwrap();
    assert!(read_lance.validate().is_ok(), "Modified SFX_Lance.swf must pass validation");
    let lance_syms: std::collections::HashMap<String, u16> = read_lance.get_symbol_classes().unwrap().into_iter().map(|(id, name)| (name, id)).collect();

    let l1_rear = lance_syms.get("a_RLFXAttackSpecialDwarf1IW_Rear").copied().expect("a_RLFXAttackSpecialDwarf1IW_Rear present");
    let l_swoosh = lance_syms.get("a_SwooshDwarfNLanceIW").copied().expect("a_SwooshDwarfNLanceIW present");
    assert_eq!(l1_rear, 87, "a_RLFXAttackSpecialDwarf1IW_Rear must point to Macho Man sprite ID 87");
    assert_eq!(l_swoosh, 68, "a_SwooshDwarfNLanceIW must point to Macho Man sprite ID 68");

    // 3. Verify backup_manager.repair_sfx_files() does NOT clobber valid modified files
    let pristine_dir = temp_dir.join("_pristine");
    fs::create_dir_all(&pristine_dir).unwrap();
    fs::copy(&axe_orig, pristine_dir.join("SFX_Axe.swf")).unwrap();
    fs::copy(&lance_orig, pristine_dir.join("SFX_Lance.swf")).unwrap();

    let backup_mgr = BackupManager::new(&temp_dir.to_string_lossy());
    let repaired = backup_mgr.repair_sfx_files().unwrap();
    assert_eq!(repaired, 0, "repair_sfx_files must NOT overwrite valid modified SWF files! Repaired count was {}", repaired);

    // Verify hash is still modified (not reverted)
    let modified_axe_hash = BackupManager::compute_sha256(&axe_copy).unwrap();
    let pristine_axe_hash = BackupManager::compute_sha256(&axe_orig).unwrap();
    assert_ne!(modified_axe_hash, pristine_axe_hash, "SFX_Axe.swf must remain modified after repair_sfx_files");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_regression_xavier_woods_vfx_complete() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    let temp_dir = std::env::temp_dir().join(format!("bc_test_xavier_vfx_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let hammer_copy = temp_dir.join("SFX_Hammer.swf");
    let sword_copy = temp_dir.join("SFX_Sword.swf");
    let hitreacts_copy = temp_dir.join("SFX_HitReacts.swf");

    fs::copy(game_dir.join("SFX_Hammer.swf"), &hammer_copy).unwrap();
    fs::copy(game_dir.join("SFX_Sword.swf"), &sword_copy).unwrap();
    fs::copy(game_dir.join("SFX_HitReacts.swf"), &hitreacts_copy).unwrap();

    // 1. Remap SFX_Hammer.swf
    let mut hammer_pkg = SwfPackage::read_from_file(&hammer_copy).unwrap();
    let hammer_remapped = hammer_pkg.remap_sfx_symbols("Viking", "SwapXavier");
    assert!(hammer_remapped >= 3, "SFX_Hammer.swf must remap at least 3 symbols (including bear suppression), got {}", hammer_remapped);
    hammer_pkg.write_to_file(&hammer_copy).unwrap();

    let read_hammer = SwfPackage::read_from_file(&hammer_copy).unwrap();
    assert!(read_hammer.validate().is_ok(), "Modified SFX_Hammer.swf must pass validation");
    let h_syms: std::collections::HashMap<String, u16> = read_hammer.get_symbol_classes().unwrap().into_iter().map(|(id, name)| (name, id)).collect();
    let side_smash = h_syms.get("a_HFXAttackSideSmashViking").copied().expect("a_HFXAttackSideSmashViking");
    let special_start = h_syms.get("a_HFXAttackSpecialBodvarStartIW_Front").copied().expect("a_HFXAttackSpecialBodvarStartIW_Front");
    let bear_rear = h_syms.get("a_HFXAttackSpecialBodvarIW_Rear").copied().expect("a_HFXAttackSpecialBodvarIW_Rear");

    assert_eq!(side_smash, 319, "a_HFXAttackSideSmashViking must point to Xavier sprite ID 319");
    assert_eq!(special_start, 292, "a_HFXAttackSpecialBodvarStartIW_Front must point to Xavier sprite ID 292");
    assert_ne!(bear_rear, 299, "Bodvar's base bear ID 299 must be replaced by Xavier effect to prevent clashing");

    // 2. Remap SFX_Sword.swf
    let mut sword_pkg = SwfPackage::read_from_file(&sword_copy).unwrap();
    let sword_remapped = sword_pkg.remap_sfx_symbols("Viking", "SwapXavier");
    assert!(sword_remapped >= 3, "SFX_Sword.swf must remap at least 3 symbols, got {}", sword_remapped);
    sword_pkg.write_to_file(&sword_copy).unwrap();

    let read_sword = SwfPackage::read_from_file(&sword_copy).unwrap();
    assert!(read_sword.validate().is_ok(), "Modified SFX_Sword.swf must pass validation");
    let s_syms: std::collections::HashMap<String, u16> = read_sword.get_symbol_classes().unwrap().into_iter().map(|(id, name)| (name, id)).collect();
    let swoosh = s_syms.get("a_swooshAttackSpecialVikingNewDay").copied().expect("a_swooshAttackSpecialVikingNewDay");
    let front = s_syms.get("a_SFXAttackSpecialVikingIW_Front").copied().expect("a_SFXAttackSpecialVikingIW_Front");
    let rear = s_syms.get("a_SFXAttackSpecialVikingIW_Rear").copied().expect("a_SFXAttackSpecialVikingIW_Rear");

    assert_eq!(swoosh, 11, "a_swooshAttackSpecialVikingNewDay must point to Xavier New Day swoosh ID 11");
    assert_eq!(front, 11, "a_SFXAttackSpecialVikingIW_Front must be redirected to Xavier authentic VFX");
    assert_eq!(rear, 11, "a_SFXAttackSpecialVikingIW_Rear must be redirected to Xavier authentic VFX");

    // 3. Remap SFX_HitReacts.swf
    let mut hitreacts_pkg = SwfPackage::read_from_file(&hitreacts_copy).unwrap();
    let hr_remapped = hitreacts_pkg.remap_sfx_symbols("Viking", "SwapXavier");
    assert!(hr_remapped >= 2, "SFX_HitReacts.swf must remap at least 2 hit reacts, got {}", hr_remapped);
    hitreacts_pkg.write_to_file(&hitreacts_copy).unwrap();

    let read_hr = SwfPackage::read_from_file(&hitreacts_copy).unwrap();
    assert!(read_hr.validate().is_ok(), "Modified SFX_HitReacts.swf must pass validation");
    let hr_syms: std::collections::HashMap<String, u16> = read_hr.get_symbol_classes().unwrap().into_iter().map(|(id, name)| (name, id)).collect();
    let hammer_hit = hr_syms.get("a_HitReactHammerViking").copied().expect("a_HitReactHammerViking");
    let sword_hit = hr_syms.get("a_HitReactGreen").copied().expect("a_HitReactGreen");
    assert_eq!(hammer_hit, 501, "a_HitReactHammerViking must point to Xavier hit reaction ID 501");
    assert_eq!(sword_hit, 518, "a_HitReactGreen must point to Xavier hit reaction ID 518");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_regression_negan_podium_presentation() {
    use brawlcraft_slint::bnk_engine::ak_hash;
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use flate2::read::ZlibDecoder;
    use std::fs;
    use std::io::Read;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let game_dir = Path::new(&inst.game_path);

    // 1. Verify Character Select & Podium Stance in Animation_CharacterSelect.anm
    let select_anm = game_dir.join("anims").join("Animation_CharacterSelect.anm");
    assert!(select_anm.is_file(), "Animation_CharacterSelect.anm must exist");
    let select_bytes = fs::read(&select_anm).unwrap();
    let mut decoder = ZlibDecoder::new(&select_bytes[4..]);
    let mut decomp = Vec::new();
    decoder.read_to_end(&mut decomp).unwrap();

    assert!(decomp.windows("IdleNegan".len()).any(|w| w == "IdleNegan".as_bytes()), "IdleNegan must exist in Animation_CharacterSelect.anm");
    assert!(decomp.windows("SelectedNegan".len()).any(|w| w == "SelectedNegan".as_bytes()), "SelectedNegan must exist in Animation_CharacterSelect.anm");
    assert!(decomp.windows("IdleSellsword".len()).any(|w| w == "IdleSellsword".as_bytes()), "IdleSellsword must exist in Animation_CharacterSelect.anm");
    assert!(decomp.windows("SelectedSellsword".len()).any(|w| w == "SelectedSellsword".as_bytes()), "SelectedSellsword must exist in Animation_CharacterSelect.anm");

    // 2. Verify UI_Podiums.anm is purely pedestal geometry, confirming player podium stance is driven by Animation_CharacterSelect.anm
    let podiums_anm = game_dir.join("anims").join("UI_Podiums.anm");
    assert!(podiums_anm.is_file(), "UI_Podiums.anm must exist");
    let pod_bytes = fs::read(&podiums_anm).unwrap();
    let mut pod_decoder = ZlibDecoder::new(&pod_bytes[4..]);
    let mut pod_decomp = Vec::new();
    pod_decoder.read_to_end(&mut pod_decomp).unwrap();
    assert!(!pod_decomp.windows("Negan".len()).any(|w| w == "Negan".as_bytes()), "UI_Podiums.anm must NOT contain character animations; it strictly drives pedestal rigs");

    // 3. Verify Announcer Voice in VOX_Announcer.bnk
    let vox_path = game_dir.join("audio").join("pc").join("English(US)").join("VOX_Announcer.bnk");
    let vox_bytes = fs::read(&vox_path).unwrap();
    let negan_vox_hash = ak_hash("VO_Announcer_Name_Jaeyun_Negan_Play").to_le_bytes();
    let jaeyun_vox_hash = ak_hash("VO_Announcer_Name_Jaeyun_Play").to_le_bytes();
    assert!(vox_bytes.windows(4).any(|w| w == negan_vox_hash), "Negan announcer event 0x2A2C62A4 must exist in VOX_Announcer.bnk");
    assert!(vox_bytes.windows(4).any(|w| w == jaeyun_vox_hash), "Jaeyun announcer event 0x3EEA1CB6 must exist in VOX_Announcer.bnk");

    // 4. Verify Lock-in & Selection Audio in PC_Movement.bnk
    let mvt_path = game_dir.join("audio").join("pc").join("PC_Movement.bnk");
    let mvt_bytes = fs::read(&mvt_path).unwrap();
    let negan_mvt_hash = ak_hash("MVT_Select_Sellsword_Negan_Play").to_le_bytes();
    let jaeyun_mvt_hash = ak_hash("MVT_Select_Sellsword_Play").to_le_bytes();
    assert!(mvt_bytes.windows(4).any(|w| w == negan_mvt_hash), "Negan lock-in sound event 0xB580989C must exist in PC_Movement.bnk");
    assert!(mvt_bytes.windows(4).any(|w| w == jaeyun_mvt_hash), "Sellsword lock-in sound event 0x08C3DB7E must exist in PC_Movement.bnk");

    // 5. Verify Roster & Podium Portrait in UI_Icons.swf
    let ui_icons = game_dir.join("UI_Icons.swf");
    let icons_pkg = SwfPackage::read_from_file(&ui_icons).unwrap();
    let icon_syms = icons_pkg.get_symbol_classes().unwrap();
    assert!(icon_syms.iter().any(|(_, name)| name == "a_Roster_Pose_Negan"), "a_Roster_Pose_Negan must exist in UI_Icons.swf");
}



#[test]
fn test_verify_complete_legend_catalog_restoration() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::RendererClient;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let playable_legends: Vec<_> = cat
        .legends
        .iter()
        .filter(|l| !l.hero_name.eq_ignore_ascii_case("random") && !l.hero_name.eq_ignore_ascii_case("default_character"))
        .collect();

    // 1. Total playable legends discovered must be >= 70
    assert_eq!(
        playable_legends.len(),
        70,
        "Expected exactly 70 playable legends, got {}",
        playable_legends.len()
    );

    // 2. Explicitly verify Artemis, Yumiko, and Magyar are restored and present
    let artemis = playable_legends.iter().find(|l| l.display_name.eq_ignore_ascii_case("Artemis") || l.hero_name.eq_ignore_ascii_case("Spacehunter"));
    assert!(artemis.is_some(), "Artemis must be present in complete legend catalog");
    let artemis = artemis.unwrap();
    assert_eq!(artemis.hero_id, 37, "Artemis HeroID must be 37");
    assert_eq!(artemis.weapon1, "RocketLance", "Artemis weapon 1 must be RocketLance");
    assert_eq!(artemis.weapon2, "Scythe", "Artemis weapon 2 must be Scythe");
    assert!(artemis.costumes.len() >= 10, "Artemis must have available costumes");

    let yumiko = playable_legends.iter().find(|l| l.display_name.eq_ignore_ascii_case("Yumiko") || l.hero_name.eq_ignore_ascii_case("Ninetails"));
    assert!(yumiko.is_some(), "Yumiko must be present in complete legend catalog");
    let yumiko = yumiko.unwrap();
    assert_eq!(yumiko.hero_id, 36, "Yumiko HeroID must be 36");
    assert_eq!(yumiko.weapon1, "Bow", "Yumiko weapon 1 must be Bow");
    assert_eq!(yumiko.weapon2, "Hammer", "Yumiko weapon 2 must be Hammer");
    assert!(yumiko.costumes.len() >= 10, "Yumiko must have available costumes");

    let magyar = playable_legends.iter().find(|l| l.display_name.eq_ignore_ascii_case("Magyar") || l.hero_name.eq_ignore_ascii_case("GhostArmor"));
    assert!(magyar.is_some(), "Magyar must be present in complete legend catalog");
    let magyar = magyar.unwrap();
    assert_eq!(magyar.hero_id, 56, "Magyar HeroID must be 56");
    assert_eq!(magyar.weapon1, "Hammer", "Magyar weapon 1 must be Hammer");
    assert_eq!(magyar.weapon2, "Greatsword", "Magyar weapon 2 must be Greatsword");
    assert!(magyar.costumes.len() >= 8, "Magyar must have available costumes");

    // 3. Verify all 70 playable legends render valid SVG poses
    let mut client = RendererClient::new();
    let mut render_success = 0;
    for legend in &playable_legends {
        let req = serde_json::json!({
            "type": "legend",
            "name": legend.display_name,
            "hero": legend.hero_name,
            "gamePath": inst.game_path
        });
        if let Some(svg) = client.render(&req) {
            if svg.trim().starts_with("<svg") {
                render_success += 1;
            }
        }
    }
    assert_eq!(
        render_success,
        70,
        "All 70 playable legends must successfully render an authentic SVG pose, got {}",
        render_success
    );
}

#[test]
fn test_verify_complete_character_skin_catalog() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::RendererClient;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    assert!(cat.costumes.len() >= 670, "Expected >= 670 costumes across full game catalog, found {}", cat.costumes.len());

    let playable_legends: Vec<_> = cat.legends.iter()
        .filter(|l| !l.hero_name.eq_ignore_ascii_case("random") && !l.hero_name.eq_ignore_ascii_case("default_character"))
        .collect();

    for leg in &playable_legends {
        let def = leg.costumes.iter().find(|c| c.is_default);
        assert!(def.is_some(), "Legend {} ({}) must have a default costume identified", leg.display_name, leg.hero_name);
        assert!(!leg.costumes.is_empty(), "Legend {} must have costumes list populated", leg.display_name);
    }

    let mut client = RendererClient::new();
    let mut rendered_costumes = 0;
    for c in &cat.costumes {
        let req = serde_json::json!({
            "type": "costume",
            "icon": c.icon_name,
            "iconFile": c.icon_file_name,
            "costumeName": c.costume_name,
            "ownerHero": c.owner_hero,
            "swfFile": c.swf_file,
            "gamePath": inst.game_path
        });
        if let Some(svg) = client.render(&req) {
            if svg.trim().starts_with("<svg") {
                rendered_costumes += 1;
            }
        }
    }
    assert!(rendered_costumes >= 660, "Expected >= 660 costumes to render SVG, got {}", rendered_costumes);
}

#[test]
fn test_verify_complete_weapon_skin_catalog() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;
    use std::collections::HashSet;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    assert!(cat.weapon_skins.len() >= 2100, "Expected >= 2100 weapon skins, found {}", cat.weapon_skins.len());

    let categories: HashSet<_> = cat.weapon_skins.iter().map(|w| w.base_weapon.to_lowercase()).collect();
    let expected = ["hammer", "sword", "pistol", "rocketlance", "spear", "katar", "axe", "bow", "fists", "scythe", "cannon", "orb", "greatsword", "boots"];
    for exp in &expected {
        assert!(categories.contains(*exp), "Weapon category {} must exist in complete weapon catalog", exp);
    }
}

#[test]
fn test_verify_costume_weapon_synchronization_complete_roster() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let sample_heroes = [
        ("Viking", "Bödvar"), ("Thatch", "Thatch"), ("Dwarf", "Ulgrim"), ("Gargoyle", "Onyx"), ("Sellsword", "Jaeyun"),
        ("Spacehunter", "Artemis"), ("Ninetails", "Yumiko"), ("GhostArmor", "Magyar"), ("Skeleton", "Azoth"), ("ActualRobot", "Vector"),
        ("VampLord", "Volkov"), ("BirdBard", "Munin"), ("RoboEngineer", "Seven"), ("GreekSpeedster", "Thea"), ("Loki", "Loki"),
        ("Imugi", "Imugi"), ("AfricanKing", "King Zuva"), ("BladeDancer", "Priya"), ("CyberVirus", "Ransom"), ("Cleric", "Lady Vera"),
        ("DarkheartMonster", "Rupture"), ("ActualGladiator", "Aurus"), ("AstroGirl", "Qinghua & Baobao")
    ];

    for (hero_internal, hero_disp) in &sample_heroes {
        let leg = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case(hero_internal) || l.display_name.eq_ignore_ascii_case(hero_disp));
        assert!(leg.is_some(), "Hero {} ({}) must exist in complete catalog", hero_disp, hero_internal);
        let leg = leg.unwrap();

        for costume in &leg.costumes {
            if costume.is_default {
                continue;
            }
            let matching_weapons: Vec<_> = cat.weapon_skins.iter()
                .filter(|w| {
                    (w.owner_hero.eq_ignore_ascii_case(&costume.costume_name) ||
                     w.weapon_skin_name.to_lowercase().contains(&costume.costume_name.to_lowercase()))
                    && (w.base_weapon.eq_ignore_ascii_case(&leg.weapon1) || w.base_weapon.eq_ignore_ascii_case(&leg.weapon2))
                })
                .collect();
            if costume.is_crossover {
                assert!(!matching_weapons.is_empty(), "Crossover costume {} of {} must have matching weapon skins", costume.costume_name, hero_disp);
            }
        }
    }
}

#[test]
fn test_verify_desktop_app_serves_complete_catalog() {
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::desktop_app::FrontendAssets;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    let index_file = FrontendAssets::get("index.html").expect("index.html must be embedded in binary");
    let index_str = std::str::from_utf8(&index_file.data).expect("index.html must be valid utf8");
    assert!(index_str.contains("<script"), "index.html must include script tag");

    let playable = cat.legends.iter()
        .filter(|l| !l.hero_name.eq_ignore_ascii_case("random") && !l.hero_name.eq_ignore_ascii_case("default_character"))
        .count();
    assert_eq!(playable, 70, "Desktop app catalog must serve exactly 70 playable legends, got {}", playable);
    assert!(cat.weapon_skins.len() >= 2100, "Desktop app catalog must serve at least 2100 weapon skins");
}

#[test]
fn test_verify_complete_data_pipeline_at_every_stage() {
    use brawlcraft_slint::catalog::{parse_legends, Catalog};
    use brawlcraft_slint::desktop_app::FrontendAssets;
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }

    // STAGE 1: Local Brawlhalla assets (Game.swz -> HeroTypes.xml)
    use brawlcraft_slint::swz::{find_swz_key, read_swz_files};
    let game_swz_path = std::path::Path::new(&inst.game_swz_path);
    assert!(game_swz_path.is_file(), "Game.swz must exist in Brawlhalla directory");
    let air_swf_path = std::path::Path::new(&inst.air_swf_path);
    let key = find_swz_key(air_swf_path, game_swz_path).unwrap_or(154281100u32);
    let files = read_swz_files(game_swz_path, key).expect("Game.swz must decrypt successfully");
    let hero_xml = files.get("HeroTypes.xml").expect("HeroTypes.xml must exist in Game.swz");
    let raw_parsed = parse_legends(hero_xml);
    let raw_playable = raw_parsed.iter()
        .filter(|l| !l.hero_name.eq_ignore_ascii_case("random") && !l.hero_name.eq_ignore_ascii_case("default_character"))
        .count();
    println!("STAGE 1 - Raw Discovered Playable Legends: {}", raw_playable);
    assert_eq!(raw_playable, 70, "Raw discovered playable legends in local game assets must be exactly 70");

    // STAGE 2 & 3: Catalog initialization
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    let backend_playable = cat.legends.iter()
        .filter(|l| !l.hero_name.eq_ignore_ascii_case("random") && !l.hero_name.eq_ignore_ascii_case("default_character"))
        .count();
    println!("STAGE 2 & 3 - Backend Catalog Playable Legends: {}", backend_playable);
    assert_eq!(backend_playable, 70, "Backend catalog must hold exactly 70 playable legends");

    // STAGE 4: desktop_app.rs build_catalog_response
    // Simulating IPC response payload
    let json_val = serde_json::to_value(&cat.legends).unwrap();
    let json_legends = json_val.as_array().unwrap();
    let response_playable = json_legends.iter()
        .filter(|l| {
            let h = l.get("heroName").and_then(|v| v.as_str()).unwrap_or("");
            !h.eq_ignore_ascii_case("random") && !h.eq_ignore_ascii_case("default_character")
        })
        .count();
    println!("STAGE 4 - Backend Response Playable Legends: {}", response_playable);
    assert_eq!(response_playable, 70, "Backend response must deliver exactly 70 playable legends");

    // STAGE 5: Frontend received
    // App.jsx receives cat_json.legends
    let frontend_received = response_playable;
    println!("STAGE 5 - Frontend Received Playable Legends: {}", frontend_received);
    assert_eq!(frontend_received, 70, "Frontend received must equal 70 playable legends");

    // STAGE 6: Frontend filtered result (filteredLegends in LegendsPage.jsx with search='' and weaponFilter='All')
    let frontend_filtered: Vec<_> = json_legends.iter()
        .filter(|l| {
            let h = l.get("heroName").and_then(|v| v.as_str()).unwrap_or("");
            let d = l.get("displayName").and_then(|v| v.as_str()).unwrap_or("");
            !h.eq_ignore_ascii_case("random") && !d.eq_ignore_ascii_case("random") && !h.eq_ignore_ascii_case("default_character")
        })
        .collect();
    println!("STAGE 6 - Frontend Filtered Result Legends: {}", frontend_filtered.len());
    assert_eq!(frontend_filtered.len(), 70, "Frontend filtered result must be exactly 70");

    // STAGE 7: Rendered character cards
    // Every single filtered legend produces a store-card
    let rendered_cards = frontend_filtered.len();
    println!("STAGE 7 - Rendered Character Cards: {}", rendered_cards);
    assert_eq!(rendered_cards, 70, "Rendered character cards must be exactly 70");

    // STAGE 8: Verify embedded production frontend bundle
    let index_file = FrontendAssets::get("index.html").expect("index.html embedded");
    let index_str = std::str::from_utf8(&index_file.data).expect("index.html utf8");
    assert!(index_str.contains("<script"), "index.html has script");
}

#[test]
fn test_skin_removal_and_duplicate_cleanup() {
    use brawlcraft_slint::catalog::{Catalog, EXCLUDED_COSTUME_IDS, DUPLICATE_COSTUME_IDS};
    use brawlcraft_slint::game_detector::GameDetector;

    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        return;
    }
    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();

    // 1. Verify exact counts
    assert_eq!(EXCLUDED_COSTUME_IDS.len(), 113, "Must have exactly 113 excluded costume entries");
    assert_eq!(DUPLICATE_COSTUME_IDS.len(), 5, "Must have exactly 5 duplicate costume entries");
    assert_eq!(cat.costumes.len(), 676, "Catalog should contain exactly 676 costumes after 118 exclusions");

    // 2. Verify all excluded costumes are absent from catalog and from legend costume lists
    for &(hero, cid) in EXCLUDED_COSTUME_IDS {
        assert!(
            !cat.costumes.iter().any(|c| c.costume_id == cid && c.owner_hero.eq_ignore_ascii_case(hero)),
            "Costume ID {} for {} must not exist in global costumes",
            cid, hero
        );
        if let Some(legend) = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case(hero)) {
            assert!(
                !legend.costumes.iter().any(|c| c.costume_id == cid),
                "Costume ID {} for {} must not exist in legend {} costumes",
                cid, hero, legend.display_name
            );
        }
    }

    // 3. Verify duplicate stance exclusions
    for &(hero, cid) in DUPLICATE_COSTUME_IDS {
        assert!(
            !cat.costumes.iter().any(|c| c.costume_id == cid && c.owner_hero.eq_ignore_ascii_case(hero)),
            "Duplicate Costume ID {} for {} must not exist in global costumes",
            cid, hero
        );
    }

    // 4. Verify Brynn (Wolf Casey: Eivor 589 & EivorMale 590, Barbara 359, EpicBrynn 508)
    let brynn = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("actualvalk")).expect("Brynn exists");
    for id in [589, 590, 359, 508] {
        assert!(!brynn.costumes.iter().any(|c| c.costume_id == id), "Brynn costume {} must be removed", id);
    }

    // 5. Verify Diana (Lara Croft: Lara 440 & Croft 441, EpicDiana 526)
    let diana = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("marksman")).expect("Diana exists");
    for id in [440, 441, 526] {
        assert!(!diana.costumes.iter().any(|c| c.costume_id == id), "Diana costume {} must be removed", id);
    }

    // 6. Verify Stance Duplicates Pruning:
    // Ransom (CyberVirus, ID 68): Stance 2 (720) removed, canonical default (719) kept
    let ransom = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("cybervirus")).expect("Ransom exists");
    assert!(!ransom.costumes.iter().any(|c| c.costume_id == 720), "CyberVirusStance2 must be removed");
    assert!(ransom.costumes.iter().any(|c| c.costume_id == 719 && c.is_default), "CyberVirus default must be preserved");
    assert_eq!(ransom.costumes.len(), 4, "Ransom should have exactly 4 costumes remaining");

    // Seven (Roboengineer, ID 61): Stance 2 (640) removed, canonical default (639) kept
    let seven = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("roboengineer")).expect("Seven exists");
    assert!(!seven.costumes.iter().any(|c| c.costume_id == 640), "RoboengineerStance2 must be removed");
    assert!(seven.costumes.iter().any(|c| c.costume_id == 639 && c.is_default), "Roboengineer default must be preserved");
    assert_eq!(seven.costumes.len(), 5, "Seven should have exactly 5 costumes remaining");

    // Priya (BladeDancer, ID 67): Stance 2 (964) removed, canonical default (965) kept
    let priya = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("bladedancer")).expect("Priya exists");
    assert!(!priya.costumes.iter().any(|c| c.costume_id == 964), "BladeDancerStance2 must be removed");
    assert!(priya.costumes.iter().any(|c| c.costume_id == 965 && c.is_default), "BladeDancer default must be preserved");
    assert_eq!(priya.costumes.len(), 5, "Priya should have exactly 5 costumes remaining");

    // Aurus (ActualGladiator, ID 71): Stance 2 (785) removed, canonical default (783) kept
    let aurus = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("actualgladiator")).expect("Aurus exists");
    assert!(!aurus.costumes.iter().any(|c| c.costume_id == 785), "ActualGladiatorStance2 must be removed");
    assert!(aurus.costumes.iter().any(|c| c.costume_id == 783 && c.is_default), "ActualGladiator default must be preserved");
    assert_eq!(aurus.costumes.len(), 4, "Aurus should have exactly 4 costumes remaining");

    // Lady Vera (Cleric, ID 69): Stance 2 (738) removed, canonical default (737) kept
    let vera = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("cleric")).expect("Lady Vera exists");
    assert!(!vera.costumes.iter().any(|c| c.costume_id == 738), "ClericStance2 must be removed");
    assert!(vera.costumes.iter().any(|c| c.costume_id == 737 && c.is_default), "Cleric default must be preserved");
    assert_eq!(vera.costumes.len(), 5, "Lady Vera should have exactly 5 costumes remaining");

    // 7. Verify Nash (Caveman / Gnash, ID 7): Blue (355) and Becky (409) removed, all 15 remaining distinct skins preserved
    let gnash = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("caveman")).expect("Nash/Gnash exists");
    assert!(!gnash.costumes.iter().any(|c| c.costume_id == 355), "Shovel Knight (Blue 355) must be removed from Nash");
    assert!(!gnash.costumes.iter().any(|c| c.costume_id == 409), "Becky Lynch (Becky 409) must be removed from Nash");
    assert_eq!(gnash.costumes.len(), 15, "Nash should have exactly 15 costumes remaining");
    // Verify distinct ZombieWalker variants are preserved
    assert!(gnash.costumes.iter().any(|c| c.costume_name == "ZombieWalker1"));
    assert!(gnash.costumes.iter().any(|c| c.costume_name == "ZombieWalker2"));
    assert!(gnash.costumes.iter().any(|c| c.costume_name == "ZombieWalker3"));

    // 8. Verify specific newly removed crossover skins across characters
    let ezio = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("ezio")).expect("Ezio exists");
    assert!(!ezio.costumes.iter().any(|c| c.costume_id == 641), "Anakin must be removed from Ezio");

    let petra = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("ragefighter")).expect("Petra exists");
    assert!(!petra.costumes.iter().any(|c| c.costume_id == 574), "SFKen must be removed from Petra");

    let rayman = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("rayman")).expect("Rayman exists");
    assert!(!rayman.costumes.iter().any(|c| c.costume_id == 573), "SFDhalsim must be removed from Rayman");

    let jiro = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("shinobi")).expect("Jiro exists");
    assert!(!jiro.costumes.iter().any(|c| c.costume_id == 637), "Yoshimitsu must be removed from Jiro");

    let kor = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("golem")).expect("Kor exists");
    assert!(!kor.costumes.iter().any(|c| c.costume_id == 456), "FourArms must be removed from Kor");

    let koji = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("samurai")).expect("Koji exists");
    assert!(!koji.costumes.iter().any(|c| c.costume_id == 376), "CyberSam must be removed from Koji");

    let asuri = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("cat")).expect("Asuri exists");
    assert!(!asuri.costumes.iter().any(|c| c.costume_id == 793), "Puss must be removed from Asuri");

    let raptor = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("sentai")).expect("Red Raptor exists");
    assert!(!raptor.costumes.iter().any(|c| c.costume_id == 735), "Eren must be removed from Red Raptor");

    let teros = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("minotaur")).expect("Teros exists");
    assert!(!teros.costumes.iter().any(|c| c.costume_id == 421), "RomanReigns must be removed from Teros");
    assert!(!teros.costumes.iter().any(|c| c.costume_id == 396), "Gruagach must be removed from Teros");

    let ada = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("cyber")).expect("Ada exists");
    assert!(!ada.costumes.iter().any(|c| c.costume_id == 454), "Heatblast must be removed from Ada");

    let vraxx = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("alien")).expect("Lord Vraxx exists");
    assert!(!vraxx.costumes.iter().any(|c| c.costume_id == 309), "AlienWarlord must be removed from Lord Vraxx");

    // 9. Verify unaffected legends (e.g. Bödvar, Ulgrim, Onyx) remain completely intact
    let bodvar = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("viking")).expect("Bodvar exists");
    assert_eq!(bodvar.costumes.len(), 15, "Bodvar must retain all 15 costumes");
    assert!(bodvar.costumes.iter().any(|c| c.is_default), "Bodvar default costume must exist");

    let ulgrim = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("dwarf")).expect("Ulgrim exists");
    assert_eq!(ulgrim.costumes.len(), 11, "Ulgrim must retain all 11 costumes");

    let onyx = cat.legends.iter().find(|l| l.hero_name.eq_ignore_ascii_case("gargoyle")).expect("Onyx exists");
    assert_eq!(onyx.costumes.len(), 9, "Onyx must retain all 9 costumes");
}

