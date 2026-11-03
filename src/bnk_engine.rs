use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Computes the 32-bit FNV-1 hash used by Audiokinetic Wwise for string event identifiers.
/// The input string is normalized to lowercase ASCII before hashing.
pub fn ak_hash(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in s.to_ascii_lowercase().bytes() {
        h = (h.wrapping_mul(16777619)) ^ (b as u32);
    }
    h
}

/// Swaps two event IDs in a Wwise SoundBank (.bnk) file.
/// Operates in-place on 4-byte little-endian event hashes, preserving bit-for-bit bank length
/// and header integrity.
pub fn swap_bnk_events(bnk_path: &Path, event_a: &str, event_b: &str) -> Result<usize, String> {
    if !bnk_path.is_file() {
        return Err(format!("BNK file not found: {}", bnk_path.display()));
    }

    let mut data = fs::read(bnk_path)
        .map_err(|e| format!("Failed to read {}: {}", bnk_path.display(), e))?;

    let hash_a = ak_hash(event_a).to_le_bytes();
    let hash_b = ak_hash(event_b).to_le_bytes();

    if hash_a == hash_b {
        return Ok(0);
    }

    let pos_a = data.windows(4).position(|w| w == hash_a);
    let pos_b = data.windows(4).position(|w| w == hash_b);

    match (pos_a, pos_b) {
        (Some(pa), Some(pb)) => {
            // Swap the two 4-byte hashes
            data[pa..pa + 4].copy_from_slice(&hash_b);
            data[pb..pb + 4].copy_from_slice(&hash_a);

            // Write atomically via temporary file
            let tmp_path = format!("{}.tmp", bnk_path.to_string_lossy());
            {
                let mut out = File::create(&tmp_path)
                    .map_err(|e| format!("Failed creating temp BNK {}: {}", tmp_path, e))?;
                out.write_all(&data)
                    .map_err(|e| format!("Failed writing BNK data: {}", e))?;
                out.flush()
                    .map_err(|e| format!("Failed flushing BNK data: {}", e))?;
            }

            if fs::rename(&tmp_path, bnk_path).is_err() {
                if let Err(e) = fs::copy(&tmp_path, bnk_path) {
                    let _ = fs::remove_file(&tmp_path);
                    return Err(format!("Failed to overwrite BNK {}: {}", bnk_path.display(), e));
                }
                let _ = fs::remove_file(&tmp_path);
            }

            crate::telemetry::log(
                "INFO",
                "BNK-SWAP",
                &format!(
                    "Swapped events '{}' <-> '{}' in {}",
                    event_a,
                    event_b,
                    bnk_path.file_name().unwrap_or_default().to_string_lossy()
                ),
                None,
            );

            Ok(1)
        }
        (None, _) => Err(format!(
            "Event '{}' (hash 0x{:08X}) not found in {}",
            event_a,
            ak_hash(event_a),
            bnk_path.display()
        )),
        (_, None) => Err(format!(
            "Event '{}' (hash 0x{:08X}) not found in {}",
            event_b,
            ak_hash(event_b),
            bnk_path.display()
        )),
    }
}

/// Dedicated blaster audio event mappings for Star Wars Bowcaster & E-11 Blaster.
/// Maps the dedicated WEP_Pistol_SW03 sound event IDs to the default pistol sound event IDs.
pub const CHEWBACCA_BLASTER_AUDIO_EVENTS: &[(&str, &str)] = &[
    ("IMP_Pistol_SW02_Play", "IMP_Pistol_Play"),
    ("WEP_Pistol_SW03_Air_Down_A_Combo_Play", "WEP_Pistol_Air_Down_A_Combo_Play"),
    ("WEP_Pistol_SW03_Air_Down_A_Play", "WEP_Pistol_Air_Down_A_Play"),
    ("WEP_Pistol_SW03_Air_Down_B_Buildup_Play", "WEP_Pistol_Air_Down_B_Buildup_Play"),
    ("WEP_Pistol_SW03_Air_Down_B_Fire_Play", "WEP_Pistol_Air_Down_B_Fire_Play"),
    ("WEP_Pistol_SW03_Air_Side_A_Play", "WEP_Pistol_Air_Side_A_Play"),
    ("WEP_Pistol_SW03_Air_Side_B_Play", "WEP_Pistol_Air_Side_B_Play"),
    ("WEP_Pistol_SW03_Down_A_a_Play", "WEP_Pistol_Down_A_a_Play"),
    ("WEP_Pistol_SW03_Down_A_b_Play", "WEP_Pistol_Down_A_b_Play"),
    ("WEP_Pistol_SW03_Down_A_c_Play", "WEP_Pistol_Down_A_c_Play"),
    ("WEP_Pistol_SW03_Side_A_Play", "WEP_Pistol_Side_A_Play"),
    ("WEP_Pistol_SW03_Stand_A_Play", "WEP_Pistol_Stand_A_Play"),
    ("WEP_Pistol_SW03_Up_A_Play", "WEP_Pistol_Up_A_Play"),
    ("WEP_Pistol_SW03_Down_A_a2_Play", "WEP_Pistol_Down_A_a2_Play"),
    ("WEP_Pistol_SW03_Side_A2_Play", "WEP_Pistol_Side_A2_Play"),
];

/// Installs dedicated Star Wars blaster sound effects into WEP_Pistol.bnk.
/// Reads audio/pc/WEP_Pistol_03.bnk, remaps the 15 Star Wars blaster event IDs to the base pistol
/// event IDs, and writes the output directly to audio/pc/WEP_Pistol.bnk.
/// Ensures custom sound effects play for light attacks, recovery sounds, and single-blaster attack behavior.
pub fn install_chewbacca_blaster_audio(game_path: &Path) -> Result<usize, String> {
    let src_path = game_path.join("audio").join("pc").join("WEP_Pistol_03.bnk");
    let dst_path = game_path.join("audio").join("pc").join("WEP_Pistol.bnk");

    if !src_path.is_file() {
        return Err(format!("Source Star Wars audio bank not found: {}", src_path.display()));
    }

    let mut data = fs::read(&src_path)
        .map_err(|e| format!("Failed to read {}: {}", src_path.display(), e))?;

    let mut mapped_count = 0;
    for (sw_event, base_event) in CHEWBACCA_BLASTER_AUDIO_EVENTS {
        let sw_hash = ak_hash(sw_event).to_le_bytes();
        let base_hash = ak_hash(base_event).to_le_bytes();

        if let Some(pos) = data.windows(4).position(|w| w == sw_hash) {
            data[pos..pos + 4].copy_from_slice(&base_hash);
            mapped_count += 1;
        }
    }

    if mapped_count == 0 {
        return Err("No Star Wars blaster sound events were found in WEP_Pistol_03.bnk".into());
    }

    let tmp_path = format!("{}.tmp", dst_path.to_string_lossy());
    {
        let mut out = File::create(&tmp_path)
            .map_err(|e| format!("Failed creating temp file {}: {}", tmp_path, e))?;
        out.write_all(&data)
            .map_err(|e| format!("Failed writing WEP_Pistol data: {}", e))?;
        out.flush()
            .map_err(|e| format!("Failed flushing WEP_Pistol data: {}", e))?;
    }

    if fs::rename(&tmp_path, &dst_path).is_err() {
        if let Err(e) = fs::copy(&tmp_path, &dst_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("Failed to overwrite WEP_Pistol.bnk {}: {}", dst_path.display(), e));
        }
        let _ = fs::remove_file(&tmp_path);
    }

    crate::telemetry::log(
        "INFO",
        "BNK-BLASTER",
        &format!("Installed Chewbacca dedicated blaster audio ({} events mapped) into WEP_Pistol.bnk", mapped_count),
        None,
    );

    Ok(mapped_count)
}

/// Verified Signature SFX event mappings for Bödvar / Xavier Woods (WWE).
pub const XAVIER_SIGNATURE_SFX_EVENTS: &[(&str, &str)] = &[
    ("SPC_Bodvar_Hammer_Swing_Play", "SPC_Bodvar_Hammer_Neutral_B_Xavier_Fire_Play"),
    ("SPC_Bodvar_Hammer_Side_B_Swing_Play", "SPC_Bodvar_Hammer_Side_B_Xavier_Swing_Play"),
    ("SPC_Bodvar_Hammer_Swing_B_Play", "SPC_Bodvar_Hammer_Down_B_Xavier_Fire_Play"),
    ("SPC_Bodvar_Sword_Swing_A_Play", "SPC_Bodvar_Sword_Side_B_Xavier_Fire_Play"),
    ("SPC_Bodvar_Sword_Down_B_Launch_Play", "SPC_Bodvar_Sword_Down_B_Xavier_Swing_Play"),
];

/// Verified Signature SFX event mappings for Thatch / Snake Eyes (G.I. Joe).
pub const SNAKEEYES_SIGNATURE_SFX_EVENTS: &[(&str, &str)] = &[
    ("SPC_Thatch_Sword_Shark_Play", "SPC_Thatch_Sword_Neutral_B_SnakeEyes_Launch_Play"),
    ("SPC_Thatch_Sword_Swing_C_Play", "SPC_Thatch_Sword_Side_B_SnakeEyes_Launch_Play"),
    ("SPC_Thatch_Sword_Swing_D_Play", "SPC_Thatch_Sword_Down_B_SnakeEyes_Launch_Play"),
    ("Lucien_Pistol_2_Play", "SPC_Thatch_Pistol_Neutral_B_SnakeEyes_Launch_Play"),
    ("Lucien_Pistol_4_Play", "SPC_Thatch_Pistol_Side_B_SnakeEyes_Launch_Play"),
    ("SPC_Thatch_Pistol_Fire_Play", "SPC_Thatch_Pistol_Down_B_SnakeEyes_Launch_Play"),
    ("SPC_Thatch_Pistol_D_Explode_Play", "SPC_Thatch_Pistol_Down_C_SnakeEyes_Hit_Play"),
];

/// Verified Signature SFX event mappings for Thatch / Chewbacca (Star Wars).
pub const CHEWBACCA_SIGNATURE_SFX_EVENTS: &[(&str, &str)] = &[
    ("SPC_Thatch_Sword_Shark_Play", "SPC_Thatch_Chewbacca_Sword_Neutral_B_Launch_Play"),
    ("SPC_Thatch_Sword_Swing_C_Play", "SPC_Thatch_Chewbacca_Sword_Side_B_Launch_Play"),
    ("SPC_Thatch_Sword_Swing_D_Play", "SPC_Thatch_Chewbacca_Sword_Down_B_Launch_Play"),
    ("Lucien_Pistol_2_Play", "SPC_Thatch_Chewbacca_Blasters_Neutral_B_Launch_Play"),
    ("Lucien_Pistol_4_Play", "SPC_Thatch_Chewbacca_Blasters_Side_B_Launch_Play"),
    ("SPC_Thatch_Pistol_Fire_Play", "SPC_Thatch_Chewbacca_Blasters_Down_B_Launch_Play"),
    ("SPC_Thatch_Pistol_D_Explode_Play", "SPC_Thatch_Chewbacca_Blasters_Down_C_Hit_1_Play"),
];

/// Verified Signature SFX event mappings for Ulgrim / Macho Man (WWE).
pub const MACHOMAN_SIGNATURE_SFX_EVENTS: &[(&str, &str)] = &[
    ("SPC_Dwarf_Axe_Swing_B_Play", "SPC_Ulgrim_Axe_Down_B_MachoMan_Fire_Play"),
    ("SPC_Dwarf_Axe_Swing_A_Play", "SPC_Ulgrim_Axe_Side_B_MachoMan_Fire_Play"),
    ("SPC_Dwarf_Lance_Swing_Play", "SPC_Ulgrim_Lance_Neutral_B_MachoMan_Hit_Play"),
    ("SPC_Dwarf_Lance_Blast_Play", "SPC_Ulgrim_Lance_Down_B_MachoMan_Hit_Play"),
    ("SPC_Dwarf_Lance_Side_B_Play", "SPC_Ulgrim_Lance_Side_B_MachoMan_Fire_Play"),
];

/// Verified Signature SFX event mappings for Onyx / Destiny Titan (Destiny 2).
pub const DESTINYTITAN_SIGNATURE_SFX_EVENTS: &[(&str, &str)] = &[
    ("SPC_Onyx_Cannon_Neutral_B_Launch_Play", "SPC_Onyx_Titan_Cannon_Neutral_B_Launch_Play"),
    ("SPC_Onyx_Cannon_Neutral_C_Hit_Play", "SPC_Onyx_Titan_Cannon_Neutral_C_Hit_Play"),
    ("SPC_Onyx_Cannon_Side_B_Launch_Play", "SPC_Onyx_Titan_Cannon_Side_B_Launch_Play"),
    ("SPC_Onyx_Cannon_Side_C_Hit_Play", "SPC_Onyx_Titan_Cannon_Side_C_Hit_2_Play"),
    ("SPC_Onyx_Cannon_Down_B_Launch_Play", "SPC_Onyx_Titan_Cannon_Down_B_Launch_Play"),
    ("SPC_Onyx_Cannon_Down_C_Hit_Play", "SPC_Onyx_Titan_Cannon_Down_C_Hit_Play"),
    ("SPC_Onyx_Gauntlet_Neutral_B_Launch_Play", "SPC_Onyx_Titan_Gauntlets_Neutral_B_Launch_Play"),
    ("SPC_Onyx_Gauntlet_Neutral_C_Hit_Play", "SPC_Onyx_Titan_Gauntlets_Neutral_C_Hit_Play"),
    ("SPC_Onyx_Gauntlet_Side_B_Launch_Play", "SPC_Onyx_Titan_Gauntlets_Side_B_Launch_Play"),
    ("SPC_Onyx_Gauntlet_Side_C_Hit_Play", "SPC_Onyx_Titan_Gauntlets_Side_C_Hit_Play"),
    ("SPC_Onyx_Gauntlet_Down_B_Launch_Play", "SPC_Onyx_Titan_Gauntlets_Down_B_Launch_Play"),
    ("SPC_Onyx_Gauntlet_Down_C_Hit_Play", "SPC_Onyx_Titan_Gauntlets_Down_C_Hit_1_Play"),
];

/// Verified Signature SFX event mappings for Jaeyun / Negan (The Walking Dead).
pub const NEGAN_SIGNATURE_SFX_EVENTS: &[(&str, &str)] = &[
    ("SPC_Sellsword_Sword_Neutral_B_Launch_Play", "SPC_Sellsword_Sword_Neutral_B_Negan_Launch_Play"),
    ("SPC_Sellsword_Sword_Neutral_C_Hit_Play", "SPC_Sellsword_Sword_Neutral_C_Negan_Hit_Play"),
    ("SPC_Sellsword_Sword_Side_B_Launch_Play", "SPC_Sellsword_Sword_Side_B_Negan_Launch_Play"),
    ("SPC_Sellsword_Sword_Side_C_Hit_Play", "SPC_Sellsword_Sword_Side_C_Negan_Hit_Play"),
    ("SPC_Sellsword_Sword_Down_B_Launch_Play", "SPC_Sellsword_Sword_Down_B_Negan_Launch_Play"),
    ("SPC_Sellsword_Sword_Down_C_Hit_Play", "SPC_Sellsword_Sword_Down_C_Negan_Hit_Play"),
    ("SPC_Sellsword_Greatsword_Neutral_B_Launch_Play", "SPC_Sellsword_Greatsword_Neutral_B_Negan_Launch_Play"),
    ("SPC_Sellsword_Greatsword_Neutral_C_Hit_Play", "SPC_Sellsword_Greatsword_Neutral_C_Negan_Hit_Play"),
    ("SPC_Sellsword_Greatsword_Side_D_LaunchSolo_Play", "SPC_Sellsword_Greatsword_Side_D_Negan_LaunchSolo_Play"),
    ("SPC_Sellsword_Greatsword_Side_F_GroundRocksSolo_Play", "SPC_Sellsword_Greatsword_Side_F_Negan_GroundRocksSolo_Play"),
    ("SPC_Sellsword_Greatsword_Side_C_Hit_Play", "SPC_Sellsword_Greatsword_Side_C_Negan_Hit_Play"),
    ("SPC_Sellsword_Greatsword_Side_E_AirLeavesSolo_Play", "SPC_Sellsword_Greatsword_Side_D_Negan_AirLeavesSolo_Play"),
    ("SPC_Sellsword_Greatsword_Side_Air_C_Hit_Play", "SPC_Sellsword_Greatsword_Side_Air_C_Negan_Hit_Play"),
    ("SPC_Sellsword_Greatsword_Down_D_LaunchSolo_Play", "SPC_Sellsword_Greatsword_Down_D_Negan_LaunchSolo_Play"),
    ("SPC_Sellsword_Greatsword_Down_F_GroundRocksSolo_Play", "SPC_Sellsword_Greatsword_Down_F_Negan_GroundRocksSolo_Play"),
    ("SPC_Sellsword_Greatsword_Down_E_AirLeavesSolo_Play", "SPC_Sellsword_Greatsword_Down_E_Negan_AirLeavesSolo_Play"),
    ("SPC_Sellsword_Greatsword_Down_Air_C_Hit_Play", "SPC_Sellsword_Greatsword_Down_Air_C_Negan_Hit_Play"),
];

/// Returns the soundbank filename and verified event pair slice for a given crossover legend.
pub fn get_signature_sfx_info(crossover_suffix: &str) -> Option<(&'static str, &'static [(&'static str, &'static str)])> {
    match crossover_suffix.to_lowercase().as_str() {
        "xavier" => Some(("SPC_Bodvar.bnk", XAVIER_SIGNATURE_SFX_EVENTS)),
        "snakeeyes" => Some(("SPC_Thatch.bnk", SNAKEEYES_SIGNATURE_SFX_EVENTS)),
        "chewbacca" => Some(("SPC_Thatch.bnk", CHEWBACCA_SIGNATURE_SFX_EVENTS)),
        "machoman" => Some(("SPC_Dwarf.bnk", MACHOMAN_SIGNATURE_SFX_EVENTS)),
        "destinytitan" => Some(("SPC_Onyx.bnk", DESTINYTITAN_SIGNATURE_SFX_EVENTS)),
        "negan" => Some(("SPC_Sellsword.bnk", NEGAN_SIGNATURE_SFX_EVENTS)),
        _ => None,
    }
}

/// Swaps all authentic Signature SFX events between base legend and crossover in the legend's SPC SoundBank.
pub fn swap_signature_sfx(game_path: &Path, crossover_suffix: &str) -> Result<usize, String> {
    let (bank_name, pairs) = match get_signature_sfx_info(crossover_suffix) {
        Some(info) => info,
        None => return Ok(0),
    };

    let bnk_path = game_path.join("audio").join("pc").join(bank_name);
    if !bnk_path.is_file() {
        return Err(format!("BNK file not found: {}", bnk_path.display()));
    }

    let mut data = fs::read(&bnk_path)
        .map_err(|e| format!("Failed to read {}: {}", bnk_path.display(), e))?;

    let mut swapped_count = 0;
    for (event_a, event_b) in pairs {
        let hash_a = ak_hash(event_a).to_le_bytes();
        let hash_b = ak_hash(event_b).to_le_bytes();

        if hash_a == hash_b {
            continue;
        }

        let pos_a = data.windows(4).position(|w| w == hash_a);
        let pos_b = data.windows(4).position(|w| w == hash_b);

        if let (Some(pa), Some(pb)) = (pos_a, pos_b) {
            data[pa..pa + 4].copy_from_slice(&hash_b);
            data[pb..pb + 4].copy_from_slice(&hash_a);
            swapped_count += 1;
        }
    }

    if swapped_count == 0 {
        return Ok(0);
    }

    let tmp_path = format!("{}.tmp", bnk_path.to_string_lossy());
    {
        let mut out = File::create(&tmp_path)
            .map_err(|e| format!("Failed creating temp BNK {}: {}", tmp_path, e))?;
        out.write_all(&data)
            .map_err(|e| format!("Failed writing BNK data: {}", e))?;
        out.flush()
            .map_err(|e| format!("Failed flushing BNK data: {}", e))?;
    }

    if fs::rename(&tmp_path, &bnk_path).is_err() {
        if let Err(e) = fs::copy(&tmp_path, &bnk_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("Failed to overwrite BNK {}: {}", bnk_path.display(), e));
        }
        let _ = fs::remove_file(&tmp_path);
    }

    crate::telemetry::log(
        "INFO",
        "BNK-SIG-SFX",
        &format!(
            "Swapped {} signature sound event(s) in {}",
            swapped_count,
            bank_name
        ),
        None,
    );

    Ok(swapped_count)
}
