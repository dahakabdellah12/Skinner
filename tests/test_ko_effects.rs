use brawlcraft_slint::game_detector::GameDetector;
use std::path::Path;

#[test]
fn test_ko_effects_pipeline() {
    let inst = GameDetector::detect(None);
    if !inst.is_valid {
        println!("Game not valid");
        return;
    }
    let game_dir = Path::new(&inst.game_path);
    
    use brawlcraft_slint::swf_engine::SwfPackage;
    let mut valid_symbols = std::collections::HashSet::new();
    for swf_name in ["SFX_KO.swf", "SFX_KO2.swf"] {
        let p = game_dir.join(swf_name);
        if let Ok(swf) = SwfPackage::read_from_file(&p) {
            if let Ok(syms) = swf.get_symbol_classes() {
                for (_, name) in syms {
                    valid_symbols.insert((swf_name, name));
                }
            }
        }
    }
    println!("Total symbols in SFX_KO + SFX_KO2: {}", valid_symbols.len());
    
    // TEST APPLYING KO EFFECT MODIFICATION
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use brawlcraft_slint::models::ModificationRequest;
    
    let mut catalog = Catalog::default();
    catalog.load_from_installation(&inst).unwrap();
    let mut backup_mgr = BackupManager::new(&inst.game_path);
    
    println!("\n=== TESTING INTRA-SWF KO EFFECT MODIFICATION ===");
    let mut engine = ModificationEngine::new(Path::new(&inst.game_path).to_path_buf(), &mut backup_mgr, &catalog);
    let mut req = ModificationRequest::default();
    req.target_ko_effect_name = Some("SmokeTrailEffect".into());
    req.replacement_ko_effect_name = Some("DemonHand".into());
    let res = engine.apply_ko_effect_modification(&req);
    println!("Intra-SWF result: success={} msg='{}'", res.success, res.message);
    assert!(res.success, "Intra-SWF KO modification must succeed");
    
    // Restore
    engine.backup_manager.restore_file("SFX_KO.swf").unwrap();
    let _ = engine.backup_manager.restore_file("audio/pc/UI.bnk");
    println!("Restored SFX_KO.swf and UI.bnk to pristine");
    
    println!("\n=== TESTING CROSS-SWF KO EFFECT MODIFICATION ===");
    let mut req_cross = ModificationRequest::default();
    req_cross.target_ko_effect_name = Some("SmokeTrailEffect".into());
    req_cross.replacement_ko_effect_name = Some("StreetFighter".into());
    let res_cross = engine.apply_ko_effect_modification(&req_cross);
    println!("Cross-SWF result: success={} msg='{}'", res_cross.success, res_cross.message);
    assert!(res_cross.success, "Cross-SWF KO modification must succeed");
    
    // Restore
    engine.backup_manager.restore_file("SFX_KO.swf").unwrap();
    let _ = engine.backup_manager.restore_file("audio/pc/UI.bnk");
    println!("Restored SFX_KO.swf and UI.bnk to pristine again");
}

#[test]
fn test_print_trail_effect_xml() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::catalog::Catalog;

    let inst = GameDetector::detect(None);
    if !inst.is_valid { return; }

    let mut cat = Catalog::default();
    cat.load_from_installation(&inst).unwrap();
    println!("Loaded catalog costumes count: {}", cat.costumes.len());
    println!("Loaded catalog ko_effects count: {}", cat.ko_effects.len());
    for ko in &cat.ko_effects {
        assert!(!ko.anim_class.contains("BP13"), "BP13KO must be filtered out because it is missing SWF on disk");
        assert!(!ko.anim_class.contains("TitanChomp"), "TitanChompKO must be filtered out because it is missing SWF on disk");
        if ko.ko_effect_name == "BP14" {
            assert_eq!(ko.display_name, "Divine Encounter");
        }
        if ko.ko_effect_name == "AtomicSkull" {
            assert_eq!(ko.display_name, "Fallout");
        }
    }
    for c in &cat.costumes {
        if c.owner_hero.eq_ignore_ascii_case("viking") && c.costume_name.eq_ignore_ascii_case("Bear") {
            assert_eq!(c.display_name, "Bear'dvar");
        }
        if c.owner_hero.eq_ignore_ascii_case("marksman") && c.costume_name.eq_ignore_ascii_case("Marksman") {
            assert_eq!(c.display_name, "Default Diana");
        }
    }
}


#[test]
fn test_verify_all_ko_effects_swapping() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::catalog::Catalog;
    use brawlcraft_slint::backup_manager::BackupManager;
    use brawlcraft_slint::modification_engine::ModificationEngine;
    use brawlcraft_slint::models::ModificationRequest;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid { return; }
    let mut catalog = Catalog::default();
    catalog.load_from_installation(&inst).unwrap();
    let mut backup_mgr = BackupManager::new(&inst.game_path);
    let mut engine = ModificationEngine::new(Path::new(&inst.game_path).to_path_buf(), &mut backup_mgr, &catalog);

    let test_effects = ["DemonHand", "GingerBreadMan1", "BlackHole", "Animus", "AtomicSkull", "StreetFighter"];
    for name in &test_effects {
        let mut req = ModificationRequest::default();
        req.target_ko_effect_name = Some("SmokeTrailEffect".into());
        req.replacement_ko_effect_name = Some(name.to_string());
        let res = engine.apply_ko_effect_modification(&req);
        println!("Test swap to '{}': success={}, msg='{}'", name, res.success, res.message);
        assert!(res.success, "Swap to '{}' failed: {}", name, res.message);

        // Verify that SFX_KO.swf can be re-read and parsed properly
        let sfx_ko_path = Path::new(&inst.game_path).join("SFX_KO.swf");
        let re_read = SwfPackage::read_from_file(&sfx_ko_path);
        assert!(re_read.is_ok(), "Corrupted SWF when swapping to '{}'!", name);

        // Check symbols
        let syms = re_read.unwrap().get_symbol_classes().unwrap();
        let target_sym = syms.iter().find(|(_, s)| s.eq_ignore_ascii_case("a_SmokeTrailEndBlue"));
        assert!(target_sym.is_some(), "a_SmokeTrailEndBlue missing from SymbolClass after swapping to '{}'!", name);
        println!("  -> a_SmokeTrailEndBlue mapped to ID {}", target_sym.unwrap().0);

        engine.backup_manager.restore_file("SFX_KO.swf").unwrap();
        let _ = engine.backup_manager.restore_file("audio/pc/UI.bnk");
    }
}

#[test]
#[ignore]
fn test_find_ko_sound_events_in_bnk() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::bnk_engine::ak_hash;
    use std::fs;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid { return; }
    let game_dir = Path::new(&inst.game_path);
    let audio_dir = game_dir.join("audio").join("pc");

    let events = [
        "UI_InGame_KO_Default_Play",
        "UI_InGame_KO_Demon_Hand_Play",
        "UI_InGame_KO_Shrek_Mongo_Play",
        "UI_InGame_KO_BP5_BlackHole_Play",
        "UI_InGame_KO_BP14_Angel_Play",
    ];

    println!("\n=== CHECKING KO SOUND EVENT HASHES IN BNK FILES ===");
    for e in &events {
        let h = ak_hash(e);
        let h_bytes = h.to_le_bytes();
        println!("Event '{}' -> 0x{:08X} ({:?})", e, h, h_bytes);
    }

    if let Ok(entries) = fs::read_dir(&audio_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("bnk") {
                if let Ok(data) = fs::read(&path) {
                    let mut found = Vec::new();
                    for e in &events {
                        let h_bytes = ak_hash(e).to_le_bytes();
                        if data.windows(4).any(|w| w == h_bytes) {
                            found.push(*e);
                        }
                    }
                    if !found.is_empty() {
                        println!("Found in {}: {:?}", path.file_name().unwrap().to_string_lossy(), found);
                    }
                }
            }
        }
    }
}

#[test]
fn test_inspect_black_hole_tags() {
    use brawlcraft_slint::game_detector::GameDetector;
    use brawlcraft_slint::swf_engine::SwfPackage;
    use std::path::Path;

    let inst = GameDetector::detect(None);
    if !inst.is_valid { return; }
    let game_dir = Path::new(&inst.game_path);
    let tt_path = game_dir.join("UI_TooltipAnimations.swf");
    let _swf = SwfPackage::read_from_file(&tt_path).unwrap();
    let _syms = _swf.get_symbol_classes().unwrap();
    struct BitReader<'a> {
        data: &'a [u8],
        bit_pos: usize,
    }
    impl<'a> BitReader<'a> {
        fn read_bits(&mut self, n: usize) -> u32 {
            let mut val = 0u32;
            for _ in 0..n {
                let byte_idx = self.bit_pos / 8;
                let bit_idx = 7 - (self.bit_pos % 8);
                let bit = (self.data[byte_idx] >> bit_idx) & 1;
                val = (val << 1) | (bit as u32);
                self.bit_pos += 1;
            }
            val
        }
        fn read_sbits(&mut self, n: usize) -> i32 {
            if n == 0 { return 0; }
            let v = self.read_bits(n);
            if (v & (1 << (n - 1))) != 0 {
                (v as i32) - (1 << n)
            } else {
                v as i32
            }
        }
    }

    fn decode_matrix(bytes: &[u8]) -> (f64, f64, f64, f64, f64, f64) {
        let mut r = BitReader { data: bytes, bit_pos: 0 };
        let has_scale = r.read_bits(1) != 0;
        let mut sx = 1.0;
        let mut sy = 1.0;
        if has_scale {
            let n = r.read_bits(5) as usize;
            sx = (r.read_sbits(n) as f64) / 65536.0;
            sy = (r.read_sbits(n) as f64) / 65536.0;
        }
        let has_rot = r.read_bits(1) != 0;
        let mut r0 = 0.0;
        let mut r1 = 0.0;
        if has_rot {
            let n = r.read_bits(5) as usize;
            r0 = (r.read_sbits(n) as f64) / 65536.0;
            r1 = (r.read_sbits(n) as f64) / 65536.0;
        }
        let n_trans = r.read_bits(5) as usize;
        let tx = (r.read_sbits(n_trans) as f64) / 20.0; // twips to px
        let ty = (r.read_sbits(n_trans) as f64) / 20.0;
        (sx, r0, r1, sy, tx, ty)
    }

    let wp_path = game_dir.join("Gfx_Weapons_Sets.swf");
    if let Ok(_swf) = SwfPackage::read_from_file(&wp_path) {
        let _syms = _swf.get_symbol_classes().unwrap();
    let b1 = [0xC2, 0x8B, 0x69, 0x94, 0xBB, 0x15, 0xD2, 0x72, 0xEF, 0x55, 0x6A, 0xF7, 0x8C, 0x00];
    let b2 = [0xC7, 0x08, 0xDE, 0x81, 0xB5, 0xC5, 0x58, 0xAE, 0xAA, 0x49, 0x5A, 0xB2, 0xBB, 0x20];
    let b3 = [0xC6, 0xF8, 0xA2, 0x96, 0x5D, 0xC5, 0x32, 0x7E, 0xA4, 0x90, 0x5D, 0x66, 0x10, 0xE0];
    let b4 = [0xC6, 0x87, 0x6B, 0x40, 0x2E, 0xC1, 0xDD, 0x62, 0x39, 0xA1, 0x76, 0x67, 0xBE, 0x80];
    println!("Fists Depth 1 (Forearm L): {:?}", decode_matrix(&b1));
    println!("Fists Depth 5 (Fist L):    {:?}", decode_matrix(&b2));
    println!("Fists Depth 8 (Forearm R): {:?}", decode_matrix(&b3));
    println!("Fists Depth 12 (Fist R):   {:?}", decode_matrix(&b4));
    }
}
