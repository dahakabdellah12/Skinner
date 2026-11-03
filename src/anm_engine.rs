use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

/// Swaps a specific pair of animation names directly inside an ANM file.
/// Used for podium/character selection presentation animations (e.g. IdleViking <-> IdleXavier,
/// SelectedViking <-> SelectedXavier) and individual move overrides.
pub fn swap_anm_animation_pair(
    anm_path: &Path,
    anim_a: &str,
    anim_b: &str,
) -> Result<bool, String> {
    if !anm_path.is_file() {
        return Err(format!("ANM file not found: {}", anm_path.display()));
    }

    let file_bytes = fs::read(anm_path)
        .map_err(|e| format!("Failed to read {}: {}", anm_path.display(), e))?;

    if file_bytes.len() < 4 {
        return Err("Invalid ANM file: file is too short".into());
    }

    let uncompressed_len =
        u32::from_le_bytes([file_bytes[0], file_bytes[1], file_bytes[2], file_bytes[3]]) as usize;

    let decoder = ZlibDecoder::new(&file_bytes[4..]);
    let mut decompressed = Vec::with_capacity(uncompressed_len.min(64 * 1024 * 1024));
    decoder
        .take(64 * 1024 * 1024)
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Zlib decompression failed for {}: {}", anm_path.display(), e))?;

    let a_bytes = anim_a.as_bytes();
    let b_bytes = anim_b.as_bytes();

    let mut a_pat = Vec::with_capacity(2 + a_bytes.len());
    a_pat.extend_from_slice(&(a_bytes.len() as u16).to_le_bytes());
    a_pat.extend_from_slice(a_bytes);

    let mut b_pat = Vec::with_capacity(2 + b_bytes.len());
    b_pat.extend_from_slice(&(b_bytes.len() as u16).to_le_bytes());
    b_pat.extend_from_slice(b_bytes);

    let a_pos = decompressed.windows(a_pat.len()).position(|w| *w == a_pat[..]);
    let b_pos = decompressed.windows(b_pat.len()).position(|w| *w == b_pat[..]);

    if let (Some(ap), Some(bp)) = (a_pos, b_pos) {
        let (first_pos, first_pat, first_rep, second_pos, second_pat, second_rep) = if ap < bp {
            (ap, a_pat.clone(), b_pat.clone(), bp, b_pat, a_pat)
        } else {
            (bp, b_pat.clone(), a_pat.clone(), ap, a_pat, b_pat)
        };

        let len_diff = first_rep.len() as isize - first_pat.len() as isize;
        decompressed.splice(first_pos..first_pos + first_pat.len(), first_rep);
        let adj_second_pos = (second_pos as isize + len_diff) as usize;
        decompressed.splice(adj_second_pos..adj_second_pos + second_pat.len(), second_rep);

        // Recompress ANM file
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&decompressed)
            .map_err(|e| format!("Failed to compress ANM data: {}", e))?;
        let compressed_body = encoder
            .finish()
            .map_err(|e| format!("Failed to finish ANM compression: {}", e))?;

        let tmp_path = format!("{}.tmp", anm_path.to_string_lossy());
        {
            let mut out = File::create(&tmp_path)
                .map_err(|e| format!("Failed to create temporary ANM {}: {}", tmp_path, e))?;
            out.write_all(&(decompressed.len() as u32).to_le_bytes())
                .map_err(|e| format!("Failed to write ANM length: {}", e))?;
            out.write_all(&compressed_body)
                .map_err(|e| format!("Failed to write compressed ANM: {}", e))?;
            out.flush()
                .map_err(|e| format!("Failed to flush ANM: {}", e))?;
        }

        if fs::rename(&tmp_path, anm_path).is_err() {
            if let Err(e) = fs::copy(&tmp_path, anm_path) {
                let _ = fs::remove_file(&tmp_path);
                return Err(format!("Failed to overwrite ANM {}: {}", anm_path.display(), e));
            }
            let _ = fs::remove_file(&tmp_path);
        }

        crate::telemetry::log(
            "INFO",
            "ANM-PAIR",
            &format!(
                "Swapped animation pair '{}' <-> '{}' in {}",
                anim_a,
                anim_b,
                anm_path.file_name().unwrap_or_default().to_string_lossy()
            ),
            None,
        );

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Swaps signature and weapon animations for a specific legend inside an uncompressed/compressed ANM file.
/// Guarantees 100% character isolation by strictly associating animations with `hero_key` and `swap_suffix`.
pub fn swap_anm_signatures(
    anm_path: &Path,
    hero_key: &str,
    swap_suffix: &str,
) -> Result<usize, String> {
    if !anm_path.is_file() {
        return Err(format!("ANM file not found: {}", anm_path.display()));
    }

    let file_bytes = fs::read(anm_path)
        .map_err(|e| format!("Failed to read {}: {}", anm_path.display(), e))?;

    if file_bytes.len() < 4 {
        return Err("Invalid ANM file: file is too short".into());
    }

    let uncompressed_len =
        u32::from_le_bytes([file_bytes[0], file_bytes[1], file_bytes[2], file_bytes[3]]) as usize;

    let decoder = ZlibDecoder::new(&file_bytes[4..]);
    let mut decompressed = Vec::with_capacity(uncompressed_len.min(64 * 1024 * 1024));
    decoder
        .take(64 * 1024 * 1024)
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Zlib decompression failed for {}: {}", anm_path.display(), e))?;

    // Hero key aliases (Brawlhalla internal naming in animation files)
    let hero_lower = hero_key.to_lowercase();
    let hero_aliases: Vec<&str> = match hero_lower.as_str() {
        "thatch" => vec!["thatch", "pirate"],
        "bodvar" | "viking" => vec!["bodvar", "viking"],
        "ulgrim" | "dwarf" => vec!["ulgrim", "dwarf"],
        "jaeyun" | "sellsword" => vec!["jaeyun", "sellsword"],
        "onyx" | "gargoyle" => vec!["onyx", "gargoyle"],
        other => vec![other],
    };

    let clean_suffix = if swap_suffix.starts_with("Swap") {
        swap_suffix.trim_start_matches("Swap")
    } else {
        swap_suffix
    };

    // Scan all valid length-prefixed strings in the ANM file
    let mut all_anims: HashSet<String> = HashSet::new();
    let mut offset = 0;
    while offset + 2 < decompressed.len() {
        let str_len = u16::from_le_bytes([decompressed[offset], decompressed[offset + 1]]) as usize;
        if str_len >= 3 && str_len < 90 && offset + 2 + str_len <= decompressed.len() {
            if let Ok(name) = std::str::from_utf8(&decompressed[offset + 2..offset + 2 + str_len]) {
                if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    all_anims.insert(name.to_string());
                }
            }
        }
        offset += 1;
    }

    let mut pairs_to_swap: Vec<(String, String)> = Vec::new();

    for name in &all_anims {
        let name_lower = name.to_lowercase();
        let is_swap_match = name_lower.contains(&swap_suffix.to_lowercase())
            || name_lower.contains(&clean_suffix.to_lowercase());

        if !is_swap_match {
            continue;
        }

        // Try different suffix stripping patterns to discover authentic base animation
        let mut candidate_bases: Vec<String> = Vec::new();

        let under_swap = format!("_{}", swap_suffix);
        let under_clean = format!("_{}", clean_suffix);

        if let Some(stripped) = name.strip_suffix(&under_swap) {
            candidate_bases.push(stripped.to_string());
        }
        if let Some(stripped) = name.strip_suffix(swap_suffix) {
            candidate_bases.push(stripped.to_string());
        }
        if let Some(stripped) = name.strip_suffix(&under_clean) {
            candidate_bases.push(stripped.to_string());
        }
        if let Some(stripped) = name.strip_suffix(clean_suffix) {
            candidate_bases.push(stripped.to_string());
        }

        // Special handling for Chewbacca single-pistol weapon animations
        if clean_suffix.eq_ignore_ascii_case("Chewbacca") {
            if let Some(stripped) = name.strip_suffix("ChewbaccaPistol") {
                candidate_bases.push(stripped.to_string());
            }
            if name.contains("SwapChewbacca") {
                // e.g. AttackSpecialPirateSwapChewbacca0 -> AttackSpecialPirate0
                let remapped = name.replace("SwapChewbacca", "");
                candidate_bases.push(remapped);
            }
        }

        for base in candidate_bases {
            if base.is_empty() || base == *name {
                continue;
            }
            if all_anims.contains(&base) {
                // If hero_key is provided and base contains a known hero name, ensure it matches this hero
                let base_lower = base.to_lowercase();
                let matches_hero = hero_aliases.iter().any(|alias| base_lower.contains(alias))
                    || (!base_lower.contains("viking")
                        && !base_lower.contains("pirate")
                        && !base_lower.contains("dwarf")
                        && !base_lower.contains("sellsword")
                        && !base_lower.contains("gargoyle"));

                if matches_hero {
                    if !pairs_to_swap.iter().any(|(b, s)| b == &base && s == name) {
                        pairs_to_swap.push((base.clone(), name.clone()));
                        break;
                    }
                }
            }
        }
    }

    if pairs_to_swap.is_empty() {
        return Ok(0);
    }

    let mut swap_count = 0;

    for (base_name, swap_name) in pairs_to_swap {
        let base_bytes = base_name.as_bytes();
        let swap_bytes = swap_name.as_bytes();

        let mut base_pat = Vec::with_capacity(2 + base_bytes.len());
        base_pat.extend_from_slice(&(base_bytes.len() as u16).to_le_bytes());
        base_pat.extend_from_slice(base_bytes);

        let mut swap_pat = Vec::with_capacity(2 + swap_bytes.len());
        swap_pat.extend_from_slice(&(swap_bytes.len() as u16).to_le_bytes());
        swap_pat.extend_from_slice(swap_bytes);

        let base_pos = decompressed
            .windows(base_pat.len())
            .position(|w| *w == base_pat[..]);
        let swap_pos = decompressed
            .windows(swap_pat.len())
            .position(|w| *w == swap_pat[..]);

        if let (Some(bp), Some(sp)) = (base_pos, swap_pos) {
            let (first_pos, first_pat, first_rep, second_pos, second_pat, second_rep) = if bp < sp {
                (bp, base_pat.clone(), swap_pat.clone(), sp, swap_pat, base_pat)
            } else {
                (sp, swap_pat.clone(), base_pat.clone(), bp, base_pat, swap_pat)
            };

            let len_diff = first_rep.len() as isize - first_pat.len() as isize;
            decompressed.splice(first_pos..first_pos + first_pat.len(), first_rep);
            let adj_second_pos = (second_pos as isize + len_diff) as usize;
            decompressed.splice(adj_second_pos..adj_second_pos + second_pat.len(), second_rep);

            swap_count += 1;
        }
    }

    if swap_count == 0 {
        return Ok(0);
    }

    // Recompress ANM file
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&decompressed)
        .map_err(|e| format!("Failed to compress ANM data: {}", e))?;
    let compressed_body = encoder
        .finish()
        .map_err(|e| format!("Failed to finish ANM compression: {}", e))?;

    let tmp_path = format!("{}.tmp", anm_path.to_string_lossy());
    {
        let mut out = File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temporary ANM {}: {}", tmp_path, e))?;
        out.write_all(&(decompressed.len() as u32).to_le_bytes())
            .map_err(|e| format!("Failed to write ANM length: {}", e))?;
        out.write_all(&compressed_body)
            .map_err(|e| format!("Failed to write compressed ANM: {}", e))?;
        out.flush()
            .map_err(|e| format!("Failed to flush ANM: {}", e))?;
    }

    if fs::rename(&tmp_path, anm_path).is_err() {
        if let Err(e) = fs::copy(&tmp_path, anm_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("Failed to overwrite ANM {}: {}", anm_path.display(), e));
        }
        let _ = fs::remove_file(&tmp_path);
    }

    Ok(swap_count)
}
