use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct SwfTag {
    pub tag_code: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct SwfPackage {
    pub version: u8,
    pub header_rect_and_frames: Vec<u8>,
    pub tags: Vec<SwfTag>,
}

fn is_definition_tag(tag_code: u16) -> bool {
    matches!(tag_code, 2 | 6 | 10 | 11 | 14 | 20 | 21 | 22 | 32 | 33 | 35 | 36 | 37 | 39 | 46 | 48 | 75 | 83 | 84)
}

fn offset_sprite_inner_ids(data: &mut [u8], offset: u16) {
    if data.len() < 4 { return; }
    let mut inner_offset = 4;
    while inner_offset + 2 <= data.len() {
        let h = u16::from_le_bytes([data[inner_offset], data[inner_offset + 1]]);
        let code = h >> 6;
        let mut len = (h & 0x3F) as usize;
        let header_size = if len == 0x3F {
            if inner_offset + 6 > data.len() { break; }
            len = u32::from_le_bytes([
                data[inner_offset + 2],
                data[inner_offset + 3],
                data[inner_offset + 4],
                data[inner_offset + 5],
            ]) as usize;
            6
        } else {
            2
        };

        let body_start = inner_offset + header_size;
        let next_tag = body_start + len;
        if next_tag > data.len() { break; }

        if code == 26 {
            let flags = data[body_start];
            let has_char = (flags & 0x02) != 0;
            if has_char && body_start + 4 <= next_tag {
                let c = u16::from_le_bytes([data[body_start + 3], data[body_start + 4]]);
                let nc = c.saturating_add(offset);
                data[body_start + 3] = (nc & 0xFF) as u8;
                data[body_start + 4] = ((nc >> 8) & 0xFF) as u8;
            }
        } else if code == 70 {
            if body_start + 4 <= next_tag {
                let flags1 = data[body_start];
                let flags2 = data[body_start + 1];
                let has_char = (flags1 & 0x02) != 0;
                let has_class = (flags2 & 0x08) != 0;
                if has_char && !has_class && body_start + 4 <= next_tag {
                    let c = u16::from_le_bytes([data[body_start + 4], data[body_start + 5]]);
                    let nc = c.saturating_add(offset);
                    data[body_start + 4] = (nc & 0xFF) as u8;
                    data[body_start + 5] = ((nc >> 8) & 0xFF) as u8;
                }
            }
        }
        if code == 0 { break; }
        inner_offset = next_tag;
    }
}

pub fn is_weapon_symbol_for(prefix: &str, base_weapon: &str) -> bool {
    let p = prefix.to_lowercase();
    let bw = base_weapon.to_lowercase();

    match bw.as_str() {
        "hammer" => p.contains("hammer"),
        "sword" => (p.contains("sword") || p.starts_with("a_sword")) && !p.contains("great"),
        "greatsword" => p.contains("great") || p.contains("greatsword"),
        "spear" => p.contains("spear"),
        "katar" | "katars" => p.contains("katar"),
        "axe" => p.contains("axe"),
        "bow" => p.contains("bow"),
        "pistol" | "pistols" | "blasters" | "blaster" => p.contains("pistol") || p.contains("blaster"),
        "lance" | "rocketlance" => p.contains("lance"),
        "scythe" => p.contains("scythe"),
        "cannon" => p.contains("cannon"),
        "orb" => p.contains("orb"),
        "fists" | "gauntlets" => p.contains("fists") || p.contains("gauntlet"),
        "boots" | "battleboots" | "battle boots" => p.contains("boots"),
        "chakram" => p.contains("chakram"),
        _ => p.starts_with("a_weapon") || p.contains(&bw),
    }
}

pub fn is_any_weapon_symbol(prefix: &str) -> bool {
    let p = prefix.to_lowercase();
    p.starts_with("a_weapon")
        || p.starts_with("a_spear")
        || p.starts_with("a_lancedrill")
        || p.starts_with("a_lance")
        || p.starts_with("scythe")
        || p.contains("spear")
        || p.contains("katar")
        || p.contains("sword")
        || p.contains("axe")
        || p.contains("hammer")
        || p.contains("bow")
        || p.contains("pistol")
        || p.contains("blaster")
        || p.contains("cannon")
        || p.contains("orb")
        || p.contains("fists")
        || p.contains("gauntlet")
        || p.contains("great")
        || p.contains("greatsword")
        || p.contains("chakram")
        || p.contains("boots")
}

impl SwfPackage {
    pub fn read_from_file(path: &Path) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open SWF {}: {}", path.display(), e))?;
        let mut header = [0u8; 8];
        file.read_exact(&mut header)
            .map_err(|e| format!("Failed to read SWF header: {}", e))?;

        let sig = &header[0..3];
        let version = header[3];
        let _file_length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

        let decompressed_body = if sig == b"CWS" {
            let mut decoder = ZlibDecoder::new(file);
            let mut body = Vec::new();
            decoder
                .read_to_end(&mut body)
                .map_err(|e| format!("Failed to decompress CWS: {}", e))?;
            body
        } else if sig == b"FWS" {
            let mut body = Vec::new();
            file.read_to_end(&mut body)
                .map_err(|e| format!("Failed to read FWS body: {}", e))?;
            body
        } else {
            return Err(format!("Unsupported SWF signature: {:?}", sig));
        };

        if decompressed_body.len() < 5 {
            return Err("SWF body too short".into());
        }

        // Parse RECT
        let nbits = (decompressed_body[0] >> 3) as usize;
        let total_rect_bits = 5 + nbits * 4;
        let rect_byte_len = (total_rect_bits + 7) / 8;
        // rect + frame_rate (2 bytes) + frame_count (2 bytes)
        let header_len = rect_byte_len + 4;
        if decompressed_body.len() < header_len {
            return Err("SWF body shorter than header".into());
        }

        let header_rect_and_frames = decompressed_body[..header_len].to_vec();
        let mut cursor = Cursor::new(&decompressed_body[header_len..]);
        let mut tags = Vec::new();

        let mut buf2 = [0u8; 2];
        let mut buf4 = [0u8; 4];

        while cursor.read_exact(&mut buf2).is_ok() {
            let tag_code_and_len = u16::from_le_bytes(buf2);
            let tag_code = tag_code_and_len >> 6;
            let mut tag_len = (tag_code_and_len & 0x3F) as usize;

            if tag_len == 0x3F {
                if cursor.read_exact(&mut buf4).is_err() {
                    break;
                }
                tag_len = u32::from_le_bytes(buf4) as usize;
            }

            let remaining = cursor.get_ref().len().saturating_sub(cursor.position() as usize);
            if tag_len > remaining {
                break;
            }

            let mut data = vec![0u8; tag_len];
            if cursor.read_exact(&mut data).is_err() {
                break;
            }

            tags.push(SwfTag { tag_code, data });

            if tag_code == 0 {
                // EndTag
                break;
            }
        }

        crate::telemetry::log("DEBUG", "SWF-READ", &format!("Opened SWF: {} (tags: {}, decompressed: {} bytes)", path.display(), tags.len(), decompressed_body.len()), None);

        Ok(Self {
            version,
            header_rect_and_frames,
            tags,
        })
    }

    pub fn write_to_file(&self, path: &Path) -> Result<(), String> {
        self.validate()
            .map_err(|e| format!("SWF validation failed before writing {}: {}", path.display(), e))?;

        let mut body = Vec::new();
        body.extend_from_slice(&self.header_rect_and_frames);

        for tag in &self.tags {
            let tag_code = tag.tag_code;
            let tag_len = tag.data.len();

            if tag_len < 0x3F {
                let tag_code_and_len = (tag_code << 6) | (tag_len as u16);
                body.extend_from_slice(&tag_code_and_len.to_le_bytes());
            } else {
                let tag_code_and_len = (tag_code << 6) | 0x3F;
                body.extend_from_slice(&tag_code_and_len.to_le_bytes());
                body.extend_from_slice(&(tag_len as u32).to_le_bytes());
            }
            body.extend_from_slice(&tag.data);
        }

        let decompressed_length = (8 + body.len()) as u32;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&body)
            .map_err(|e| format!("Zlib compression error: {}", e))?;
        let compressed_body = encoder
            .finish()
            .map_err(|e| format!("Zlib finish error: {}", e))?;

        let tmp_path = format!("{}.tmp", path.to_string_lossy());
        {
            let mut out = File::create(&tmp_path)
                .map_err(|e| format!("Failed to create temp file {}: {}", tmp_path, e))?;
            out.write_all(b"CWS")
                .map_err(|e| format!("Failed to write sig: {}", e))?;
            out.write_all(&[self.version])
                .map_err(|e| format!("Failed to write version: {}", e))?;
            out.write_all(&decompressed_length.to_le_bytes())
                .map_err(|e| format!("Failed to write length: {}", e))?;
            out.write_all(&compressed_body)
                .map_err(|e| format!("Failed to write data: {}", e))?;
            out.flush()
                .map_err(|e| format!("Failed to flush temp file: {}", e))?;
        }

        // Atomic move with overwrite fallback
        if fs::rename(&tmp_path, path).is_err() {
            if let Err(e) = fs::copy(&tmp_path, path) {
                let _ = fs::remove_file(&tmp_path);
                return Err(format!("Failed to overwrite {}: {}", path.display(), e));
            }
            let _ = fs::remove_file(&tmp_path);
        }

        crate::telemetry::log("DEBUG", "SWF-WRITE", &format!("Saved SWF: {} (tags: {}, compressed: {} bytes)", path.display(), self.tags.len(), compressed_body.len()), None);

        Ok(())
    }

    pub fn remap_symbols(
        &mut self,
        target_suffix: &str,
        repl_suffix: &str,
    ) -> Result<usize, String> {
        let mut remapped_count = 0;

        // 1. Calculate max character id across definition tags and SymbolClass
        let mut max_id: u16 = 0;
        for tag in &self.tags {
            if is_definition_tag(tag.tag_code) && tag.data.len() >= 2 {
                let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if id > max_id {
                    max_id = id;
                }
            }
            if tag.tag_code == 76 {
                if let Ok(syms) = parse_symbol_class(&tag.data) {
                    for (id, _) in syms {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                }
            }
        }
        let empty_sprite_id = max_id.saturating_add(1);

        // 2. Insert empty sprite Tag 39 (FrameCount 1, ShowFrame, EndTag) before SymbolClass
        let empty_sprite_data = vec![
            (empty_sprite_id & 0xFF) as u8,
            ((empty_sprite_id >> 8) & 0xFF) as u8,
            0x01, 0x00, // frame count = 1
            0x40, 0x00, // ShowFrame (Tag 1)
            0x00, 0x00, // EndTag (Tag 0)
        ];
        let empty_tag = SwfTag {
            tag_code: 39,
            data: empty_sprite_data,
        };

        if let Some(pos) = self.tags.iter().position(|t| t.tag_code == 76) {
            self.tags.insert(pos, empty_tag);
        } else {
            self.tags.push(empty_tag);
        }

        // 3. Needles for target and replacement
        let repl_needle = format!("_{}", repl_suffix);
        let target_needle = format!("_{}", target_suffix);
        let base_target_needle = if target_suffix.ends_with("New") {
            Some(format!("_{}", target_suffix.trim_end_matches("New")))
        } else {
            None
        };
        let new_target_needle = format!("_{}New", target_suffix);

        for tag in &mut self.tags {
            if tag.tag_code == 76 {
                // SymbolClass Tag
                let mut symbols = parse_symbol_class(&tag.data)?;
                let mut repl_map: HashMap<String, (u16, String)> = HashMap::new();

                for (sym_id, sym_name) in &symbols {
                    if sym_name.contains("_All") {
                        continue;
                    }
                    if let Some(prefix) = sym_name.strip_suffix(&repl_needle) {
                        if is_any_weapon_symbol(prefix) {
                            continue; // Skip weapon symbols during costume remapping so weapons remain strictly independent!
                        }
                        repl_map.insert(prefix.to_lowercase(), (*sym_id, prefix.to_string()));
                    } else if repl_suffix.eq_ignore_ascii_case("Viking") && sym_name == "a_Arm1_Sleeveless" {
                        repl_map.insert("a_arm1".to_string(), (*sym_id, "a_Arm1".to_string()));
                    }
                }

                if repl_map.is_empty() {
                    continue;
                }

                let mut additions = Vec::new();

                for (sym_id, sym_name) in symbols.iter_mut() {
                    if sym_name.contains("_All") {
                        continue;
                    }

                    let prefix_opt = if let Some(p) = sym_name.strip_suffix(&target_needle) {
                        Some(p)
                    } else if let Some(ref base_needle) = base_target_needle {
                        sym_name.strip_suffix(base_needle)
                    } else if sym_name.ends_with(&new_target_needle) {
                        sym_name.strip_suffix(&new_target_needle)
                    } else if target_suffix.eq_ignore_ascii_case("Viking") && sym_name == "a_Arm1_Sleeveless" {
                        Some("a_Arm1")
                    } else {
                        None
                    };

                    if let Some(prefix) = prefix_opt {
                        if is_any_weapon_symbol(prefix) {
                            continue; // Never overwrite weapon symbols during costume remapping!
                        }
                        let prefix_lower = prefix.to_lowercase();
                        if let Some(&(new_id, _)) = repl_map.get(&prefix_lower) {
                            *sym_id = new_id;
                            remapped_count += 1;
                        } else {
                            let p_low = prefix_lower.as_str();
                            if p_low.contains("nose") || p_low.contains("beard") || p_low.contains("mouth") || p_low.contains("eyes") || p_low.contains("hair") || p_low.contains("hat") || p_low.contains("glasses") || p_low.contains("shades") {
                                *sym_id = empty_sprite_id;
                                remapped_count += 1;
                            }
                        }
                    }
                }

                // Add any symbols that target did not have (preserving original prefix casing)
                for (_prefix_lower, (repl_id, orig_prefix)) in &repl_map {
                    if is_any_weapon_symbol(orig_prefix) {
                        continue;
                    }
                    if target_suffix.eq_ignore_ascii_case("Viking") && orig_prefix.eq_ignore_ascii_case("a_Arm1") {
                        continue; // a_Arm1_Sleeveless already handles a_Arm1 on Viking!
                    }
                    let expected_name = format!("{}_{}", orig_prefix, target_suffix);
                    if !symbols.iter().any(|(_, name)| name.eq_ignore_ascii_case(&expected_name)) {
                        additions.push((*repl_id, expected_name));
                        remapped_count += 1;
                    }
                }

                symbols.extend(additions);
                tag.data = serialize_symbol_class(&symbols);
            }
        }

        crate::telemetry::log("INFO", "SWF-REMAP", &format!("Remapped {} symbols in SWF (target: '{}', replacement: '{}')", remapped_count, target_suffix, repl_suffix), None);

        Ok(remapped_count)
    }

    /// Directly remaps a single named symbol to point to the character ID of another symbol.
    /// Used for UI roster poses, character select portraits, and specific cosmetic mappings.
    pub fn remap_single_symbol(
        &mut self,
        target_symbol: &str,
        replacement_symbol: &str,
    ) -> Result<usize, String> {
        let mut remapped_count = 0;

        for tag in &mut self.tags {
            if tag.tag_code == 76 {
                let mut symbols = parse_symbol_class(&tag.data)?;
                let repl_id = symbols.iter().find(|(_, name)| name.eq_ignore_ascii_case(replacement_symbol)).map(|(id, _)| *id);

                if let Some(new_id) = repl_id {
                    let mut found_target = false;
                    for (id, name) in symbols.iter_mut() {
                        if name.eq_ignore_ascii_case(target_symbol) {
                            if *id != new_id {
                                *id = new_id;
                                remapped_count += 1;
                            }
                            found_target = true;
                            break;
                        }
                    }
                    if !found_target {
                        symbols.push((new_id, target_symbol.to_string()));
                        remapped_count += 1;
                    }
                    tag.data = serialize_symbol_class(&symbols);
                }
            }
        }

        Ok(remapped_count)
    }

    /// Replaces the internal animation and frames of a target sprite (e.g. a_SmokeTrailEndBlue, ID 46)
    /// with the animation and frames of another sprite within the same SWF.
    /// This preserves the target's unique character ID in SymbolClass, completely avoiding
    /// ActionScript 3 / AVM2 duplicate ID collisions that cause effects to disappear in-game.
    pub fn replace_sprite_animation(
        &mut self,
        target_symbol: &str,
        replacement_symbol: &str,
    ) -> Result<bool, String> {
        let (target_id, repl_id) = {
            let sym_tag = self.tags.iter().find(|t| t.tag_code == 76)
                .ok_or_else(|| "SymbolClass tag not found".to_string())?;
            let syms = parse_symbol_class(&sym_tag.data)?;
            let tid = syms.iter().find(|(_, name)| name.eq_ignore_ascii_case(target_symbol)).map(|(id, _)| *id)
                .ok_or_else(|| format!("Target symbol '{}' not found in SymbolClass", target_symbol))?;
            let rid = syms.iter().find(|(_, name)| name.eq_ignore_ascii_case(replacement_symbol)).map(|(id, _)| *id)
                .ok_or_else(|| format!("Replacement symbol '{}' not found in SymbolClass", replacement_symbol))?;
            (tid, rid)
        };

        if target_id == repl_id {
            return Ok(true);
        }

        let repl_data = self.tags.iter()
            .find(|t| t.tag_code == 39 && t.data.len() >= 4 && u16::from_le_bytes([t.data[0], t.data[1]]) == repl_id)
            .map(|t| t.data.clone())
            .ok_or_else(|| format!("DefineSprite for replacement ID {} not found", repl_id))?;

        let target_tag = self.tags.iter_mut()
            .find(|t| t.tag_code == 39 && t.data.len() >= 4 && u16::from_le_bytes([t.data[0], t.data[1]]) == target_id)
            .ok_or_else(|| format!("DefineSprite for target ID {} not found", target_id))?;

        let mut new_data = repl_data;
        new_data[0] = (target_id & 0xFF) as u8;
        new_data[1] = ((target_id >> 8) & 0xFF) as u8;
        target_tag.data = new_data;

        Ok(true)
    }

    /// Imports all subordinate definition tags (shapes, sub-sprites) from source_swf and replaces
    /// the target sprite's internal timeline and frames with the source sprite's animation.
    /// Crucially keeps target_symbol's character ID in SymbolClass intact without collisions!
    pub fn import_and_replace_sprite_from_swf(
        &mut self,
        source_swf: &SwfPackage,
        source_symbol: &str,
        target_symbol: &str,
    ) -> Result<usize, String> {
        let target_id = {
            let sym_tag = self.tags.iter().find(|t| t.tag_code == 76)
                .ok_or_else(|| "SymbolClass tag not found in target SWF".to_string())?;
            let syms = parse_symbol_class(&sym_tag.data)?;
            syms.iter().find(|(_, name)| name.eq_ignore_ascii_case(target_symbol)).map(|(id, _)| *id)
                .ok_or_else(|| format!("Target symbol '{}' not found in SymbolClass", target_symbol))?
        };

        let source_id = {
            let sym_tag = source_swf.tags.iter().find(|t| t.tag_code == 76)
                .ok_or_else(|| "SymbolClass tag not found in source SWF".to_string())?;
            let syms = parse_symbol_class(&sym_tag.data)?;
            syms.iter().find(|(_, name)| name.eq_ignore_ascii_case(source_symbol)).map(|(id, _)| *id)
                .ok_or_else(|| format!("Source symbol '{}' not found in SymbolClass", source_symbol))?
        };

        let mut max_id: u16 = 0;
        for tag in &self.tags {
            if is_definition_tag(tag.tag_code) && tag.data.len() >= 2 {
                let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if id > max_id {
                    max_id = id;
                }
            }
        }
        let offset = max_id.saturating_add(10);

        let mut needed_ids = HashSet::new();
        needed_ids.insert(source_id);
        let mut changed = true;
        while changed {
            changed = false;
            for tag in &source_swf.tags {
                if tag.tag_code == 39 && tag.data.len() >= 4 {
                    let sprite_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if needed_ids.contains(&sprite_id) {
                        let mut inner_offset = 4;
                        while inner_offset + 2 <= tag.data.len() {
                            let h = u16::from_le_bytes([tag.data[inner_offset], tag.data[inner_offset + 1]]);
                            let code = h >> 6;
                            let mut len = (h & 0x3F) as usize;
                            let header_size = if len == 0x3F {
                                if inner_offset + 6 > tag.data.len() { break; }
                                len = u32::from_le_bytes([
                                    tag.data[inner_offset + 2],
                                    tag.data[inner_offset + 3],
                                    tag.data[inner_offset + 4],
                                    tag.data[inner_offset + 5],
                                ]) as usize;
                                6
                            } else {
                                2
                            };

                            let body_start = inner_offset + header_size;
                            let next_tag = body_start + len;
                            if next_tag > tag.data.len() { break; }

                            if code == 26 && len >= 5 {
                                let flags = tag.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let inner_char_id = u16::from_le_bytes([tag.data[body_start + 3], tag.data[body_start + 4]]);
                                    if needed_ids.insert(inner_char_id) {
                                        changed = true;
                                    }
                                }
                            } else if code == 70 && len >= 6 {
                                let flags = tag.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let inner_char_id = u16::from_le_bytes([tag.data[body_start + 4], tag.data[body_start + 5]]);
                                    if needed_ids.insert(inner_char_id) {
                                        changed = true;
                                    }
                                }
                            }

                            if code == 0 { break; }
                            inner_offset = next_tag;
                        }
                    }
                }
            }
        }

        let mut new_imported_tags = Vec::new();
        let mut imported_source_sprite = None;

        for tag in &source_swf.tags {
            if is_definition_tag(tag.tag_code) && tag.data.len() >= 2 {
                let tid = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if !needed_ids.contains(&tid) {
                    continue;
                }
                let mut tag_copy = tag.clone();
                if tid == source_id && tag.tag_code == 39 {
                    offset_sprite_inner_ids(&mut tag_copy.data, offset);
                    imported_source_sprite = Some(tag_copy);
                    continue;
                }

                let new_id = tid.saturating_add(offset);
                tag_copy.data[0] = (new_id & 0xFF) as u8;
                tag_copy.data[1] = ((new_id >> 8) & 0xFF) as u8;

                if tag_copy.tag_code == 39 && tag_copy.data.len() >= 4 {
                    offset_sprite_inner_ids(&mut tag_copy.data, offset);
                }
                new_imported_tags.push(tag_copy);
            }
        }

        let root_sprite = imported_source_sprite
            .ok_or_else(|| format!("Root sprite for source symbol '{}' not found in source SWF", source_symbol))?;

        let insert_idx = self.tags.iter().position(|t| t.tag_code == 1 || t.tag_code == 76).unwrap_or(0);
        let imported_count = new_imported_tags.len();
        for (i, t) in new_imported_tags.into_iter().enumerate() {
            self.tags.insert(insert_idx + i, t);
        }

        let target_tag = self.tags.iter_mut()
            .find(|t| t.tag_code == 39 && t.data.len() >= 4 && u16::from_le_bytes([t.data[0], t.data[1]]) == target_id)
            .ok_or_else(|| format!("DefineSprite for target ID {} not found", target_id))?;

        let mut new_data = root_sprite.data;
        new_data[0] = (target_id & 0xFF) as u8;
        new_data[1] = ((target_id >> 8) & 0xFF) as u8;
        target_tag.data = new_data;

        Ok(imported_count + 1)
    }

    /// Remaps only weapon symbols for a specific weapon type (e.g. Spear, Hammer, Axe, Sword)
    /// Intra-SWF remapping strictly scoped to the weapon to protect body parts and faces.
    pub fn remap_weapon_symbols(
        &mut self,
        target_suffix: &str,
        repl_suffix: &str,
        base_weapon: &str,
    ) -> Result<usize, String> {
        let mut remapped_count = 0;
        let repl_needle = format!("_{}", repl_suffix);
        let target_needle = format!("_{}", target_suffix);

        for tag in &mut self.tags {
            if tag.tag_code == 76 {
                let mut symbols = parse_symbol_class(&tag.data)?;
                let mut repl_map: HashMap<String, (u16, String)> = HashMap::new();

                for (sym_id, sym_name) in &symbols {
                    if sym_name.contains("_All") {
                        continue;
                    }
                    if let Some(prefix) = sym_name.strip_suffix(&repl_needle) {
                        if is_weapon_symbol_for(prefix, base_weapon) {
                            repl_map.insert(prefix.to_lowercase(), (*sym_id, prefix.to_string()));
                        }
                    }
                }

                if repl_map.is_empty() {
                    continue;
                }

                let mut additions = Vec::new();

                for (sym_id, sym_name) in symbols.iter_mut() {
                    if sym_name.contains("_All") {
                        continue;
                    }
                    if let Some(prefix) = sym_name.strip_suffix(&target_needle) {
                        if is_weapon_symbol_for(prefix, base_weapon) {
                            let prefix_lower = prefix.to_lowercase();
                            if let Some(&(new_id, _)) = repl_map.get(&prefix_lower) {
                                *sym_id = new_id;
                                remapped_count += 1;
                            }
                        }
                    }
                }

                for (_prefix_lower, (repl_id, orig_prefix)) in &repl_map {
                    let expected_name = format!("{}_{}", orig_prefix, target_suffix);
                    if !symbols.iter().any(|(_, name)| name.eq_ignore_ascii_case(&expected_name)) {
                        additions.push((*repl_id, expected_name));
                        remapped_count += 1;
                    }
                }

                symbols.extend(additions);
                tag.data = serialize_symbol_class(&symbols);
            }
        }

        crate::telemetry::log("INFO", "SWF-WEAPON-REMAP", &format!("Remapped {} weapon symbols ({}) in SWF (target: '{}', replacement: '{}')", remapped_count, base_weapon, target_suffix, repl_suffix), None);

        Ok(remapped_count)
    }

    /// Imports costume tags and symbols from an external SWF package into this SWF package,
    /// remapping target symbols to the imported sprites and blanking unneeded parts.
    pub fn import_costume_from_swf(
        &mut self,
        source_swf: &SwfPackage,
        source_suffix: &str,
        target_suffix: &str,
        color_map: &HashMap<u32, u32>,
    ) -> Result<usize, String> {
        let mut remapped_count = 0;

        // 1. Calculate max character id across definition tags and SymbolClass in self
        let mut max_id: u16 = 0;
        for tag in &self.tags {
            if is_definition_tag(tag.tag_code) && tag.data.len() >= 2 {
                let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if id > max_id {
                    max_id = id;
                }
            }
            if tag.tag_code == 76 {
                if let Ok(syms) = parse_symbol_class(&tag.data) {
                    for (id, _) in syms {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                }
            }
        }

        let empty_sprite_id = max_id.saturating_add(1);
        let offset = max_id.saturating_add(10);

        // 2. Build empty sprite Tag 39 (FrameCount 1, ShowFrame, EndTag)
        let empty_sprite_data = vec![
            (empty_sprite_id & 0xFF) as u8,
            ((empty_sprite_id >> 8) & 0xFF) as u8,
            0x01, 0x00, // frame count = 1
            0x40, 0x00, // ShowFrame (Tag 1)
            0x00, 0x00, // EndTag (Tag 0)
        ];
        let empty_tag = SwfTag {
            tag_code: 39,
            data: empty_sprite_data,
        };

        // 3. Find source symbols matching source_suffix or auto-detect actual suffix in source_swf
        let mut actual_source_suffix = source_suffix.to_string();
        let mut source_symbols: Vec<(u16, String)> = Vec::new();

        for tag in &source_swf.tags {
            if tag.tag_code == 76 {
                if let Ok(syms) = parse_symbol_class(&tag.data) {
                    source_symbols.extend(syms);
                }
            }
        }

        let direct_needle = format!("_{}", source_suffix);
        let has_direct_match = source_symbols.iter().any(|(_, name)| name.ends_with(&direct_needle));

        if !has_direct_match {
            // Auto-detect actual suffix (e.g. "BP11Azoth01" when given "BP11Azoth")
            for (_, name) in &source_symbols {
                if let Some(pos) = name.rfind(source_suffix) {
                    let candidate = &name[pos..];
                    if !candidate.is_empty() && candidate != source_suffix {
                        actual_source_suffix = candidate.to_string();
                        break;
                    }
                }
            }
        }

        let actual_needle = format!("_{}", actual_source_suffix);
        let mut imported_map: HashMap<String, (u16, String)> = HashMap::new(); // lowercase -> (new_id, original_prefix)

        for (old_id, sym_name) in &source_symbols {
            if sym_name.contains("_All") {
                continue; // Skip meme/accessory symbols like _AllMikasa
            }
            if let Some(prefix) = sym_name.strip_suffix(&actual_needle) {
                if is_any_weapon_symbol(prefix) {
                    continue; // Skip weapon symbols during costume import so weapons are strictly isolated!
                }
                let new_id = old_id.saturating_add(offset);
                imported_map.insert(prefix.to_lowercase(), (new_id, prefix.to_string()));
            }
        }

        if imported_map.is_empty() {
            return Ok(0);
        }

        // Collect needed character IDs from imported sprites recursively to resolve all dependencies
        let mut needed_ids: HashSet<u16> = HashSet::new();
        for (_, (new_id, _)) in &imported_map {
            let old_id = new_id.saturating_sub(offset);
            needed_ids.insert(old_id);
        }
        let mut changed = true;
        while changed {
            changed = false;
            for tag in &source_swf.tags {
                if tag.tag_code == 39 && tag.data.len() >= 4 {
                    let sprite_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if needed_ids.contains(&sprite_id) {
                        let mut inner_offset = 4;
                        while inner_offset + 2 <= tag.data.len() {
                            let h = u16::from_le_bytes([tag.data[inner_offset], tag.data[inner_offset + 1]]);
                            let code = h >> 6;
                            let mut len = (h & 0x3F) as usize;
                            let header_size = if len == 0x3F {
                                if inner_offset + 6 > tag.data.len() { break; }
                                len = u32::from_le_bytes([
                                    tag.data[inner_offset + 2],
                                    tag.data[inner_offset + 3],
                                    tag.data[inner_offset + 4],
                                    tag.data[inner_offset + 5],
                                ]) as usize;
                                6
                            } else {
                                2
                            };

                            let body_start = inner_offset + header_size;
                            let next_tag = body_start + len;
                            if next_tag > tag.data.len() { break; }

                            if code == 26 && len >= 5 {
                                let flags = tag.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let inner_char_id = u16::from_le_bytes([tag.data[body_start + 3], tag.data[body_start + 4]]);
                                    if needed_ids.insert(inner_char_id) {
                                        changed = true;
                                    }
                                }
                            } else if code == 70 && len >= 6 {
                                let flags = tag.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let inner_char_id = u16::from_le_bytes([tag.data[body_start + 4], tag.data[body_start + 5]]);
                                    if needed_ids.insert(inner_char_id) {
                                        changed = true;
                                    }
                                }
                            }

                            if code == 0 { break; }
                            inner_offset = next_tag;
                        }
                    }
                }
            }
        }

        // 4. Import definition tags from source_swf (DefineShape and DefineSprite)
        let mut new_imported_tags = Vec::new();
        new_imported_tags.push(empty_tag);

        for tag in &source_swf.tags {
            // DefineShape tags: 2, 22, 32, 83
            if tag.tag_code == 2 || tag.tag_code == 22 || tag.tag_code == 32 || tag.tag_code == 83 {
                if tag.data.len() >= 2 {
                    let old_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if !needed_ids.contains(&old_id) {
                        continue;
                    }
                    let mut tag_copy = tag.clone();
                    let new_id = old_id.saturating_add(offset);
                    tag_copy.data[0] = (new_id & 0xFF) as u8;
                    tag_copy.data[1] = ((new_id >> 8) & 0xFF) as u8;

                    if !color_map.is_empty() && matches!(tag_copy.tag_code, 2 | 22 | 32 | 83) {
                        replace_shape_colors(tag_copy.tag_code, &mut tag_copy.data, color_map);
                    }
                    new_imported_tags.push(tag_copy);
                }
            } else if tag.tag_code == 39 {
                // DefineSprite
                if tag.data.len() >= 4 {
                    let old_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if !needed_ids.contains(&old_id) {
                        continue;
                    }
                    let mut tag_copy = tag.clone();
                    let new_id = old_id.saturating_add(offset);
                    tag_copy.data[0] = (new_id & 0xFF) as u8;
                    tag_copy.data[1] = ((new_id >> 8) & 0xFF) as u8;

                    // Remap all inner PlaceObject2 and PlaceObject3 tags
                    let mut inner_offset = 4;
                    while inner_offset + 2 <= tag_copy.data.len() {
                        let h = u16::from_le_bytes([tag_copy.data[inner_offset], tag_copy.data[inner_offset + 1]]);
                        let code = h >> 6;
                        let mut len = (h & 0x3F) as usize;
                        let header_size = if len == 0x3F {
                            if inner_offset + 6 > tag_copy.data.len() { break; }
                            len = u32::from_le_bytes([
                                tag_copy.data[inner_offset + 2],
                                tag_copy.data[inner_offset + 3],
                                tag_copy.data[inner_offset + 4],
                                tag_copy.data[inner_offset + 5],
                            ]) as usize;
                            6
                        } else {
                            2
                        };

                        let body_start = inner_offset + header_size;
                        let next_tag = body_start + len;
                        if next_tag > tag_copy.data.len() { break; }

                        if code == 26 {
                            // PlaceObject2:
                            // byte 0: flags
                            // byte 1..3: depth (2 bytes)
                            // byte 3..5: characterId (2 bytes) if (flags & 0x02) != 0
                            if len >= 5 {
                                let flags = tag_copy.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let old_char_id = u16::from_le_bytes([
                                        tag_copy.data[body_start + 3],
                                        tag_copy.data[body_start + 4],
                                    ]);
                                    let new_char_id = old_char_id.saturating_add(offset);
                                    tag_copy.data[body_start + 3] = (new_char_id & 0xFF) as u8;
                                    tag_copy.data[body_start + 4] = ((new_char_id >> 8) & 0xFF) as u8;
                                }
                            }
                        } else if code == 70 {
                            // PlaceObject3:
                            if len >= 6 {
                                let flags = tag_copy.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let old_char_id = u16::from_le_bytes([
                                        tag_copy.data[body_start + 4],
                                        tag_copy.data[body_start + 5],
                                    ]);
                                    let new_char_id = old_char_id.saturating_add(offset);
                                    tag_copy.data[body_start + 4] = (new_char_id & 0xFF) as u8;
                                    tag_copy.data[body_start + 5] = ((new_char_id >> 8) & 0xFF) as u8;
                                }
                            }
                        }

                        if code == 0 { break; }
                        inner_offset = next_tag;
                    }

                    new_imported_tags.push(tag_copy);
                }
            }
        }

        // 5. Insert new tags into self right before SymbolClass tag
        if let Some(pos) = self.tags.iter().position(|t| t.tag_code == 76) {
            for (i, t) in new_imported_tags.into_iter().enumerate() {
                self.tags.insert(pos + i, t);
            }
        } else {
            self.tags.extend(new_imported_tags);
        }

        // 6. Remap target symbols in self's SymbolClass
        let target_needle = format!("_{}", target_suffix);
        let base_target_needle = if target_suffix.ends_with("New") {
            Some(format!("_{}", target_suffix.trim_end_matches("New")))
        } else {
            None
        };
        let new_target_needle = format!("_{}New", target_suffix);

        for tag in &mut self.tags {
            if tag.tag_code == 76 {
                let mut symbols = parse_symbol_class(&tag.data)?;
                let mut additions = Vec::new();

                for (sym_id, sym_name) in symbols.iter_mut() {
                    // CRITICAL: NEVER touch _All symbols (e.g. a_Shades_AllWitch)
                    if sym_name.contains("_All") {
                        continue;
                    }

                    let prefix_opt = if let Some(p) = sym_name.strip_suffix(&target_needle) {
                        Some(p)
                    } else if let Some(ref base_needle) = base_target_needle {
                        sym_name.strip_suffix(base_needle)
                    } else if sym_name.ends_with(&new_target_needle) {
                        sym_name.strip_suffix(&new_target_needle)
                    } else if target_suffix.eq_ignore_ascii_case("Viking") && sym_name == "a_Arm1_Sleeveless" {
                        Some("a_Arm1")
                    } else {
                        None
                    };

                    if let Some(prefix) = prefix_opt {
                        if is_any_weapon_symbol(prefix) {
                            continue; // Leave weapon symbols untouched during costume import!
                        }
                        let prefix_lower = prefix.to_lowercase();
                        if let Some(&(new_id, _)) = imported_map.get(&prefix_lower) {
                            *sym_id = new_id;
                            remapped_count += 1;
                        } else {
                            let p_low = prefix_lower.as_str();
                            if p_low.contains("nose") || p_low.contains("beard") || p_low.contains("mouth") || p_low.contains("eyes") || p_low.contains("hair") || p_low.contains("hat") || p_low.contains("glasses") || p_low.contains("shades") {
                                *sym_id = empty_sprite_id;
                                remapped_count += 1;
                            }
                        }
                    }
                }

                // Add any symbols from imported_map that target didn't have (preserving original casing!)
                for (_prefix_lower, (new_id, orig_prefix)) in &imported_map {
                    if target_suffix.eq_ignore_ascii_case("Viking") && orig_prefix.eq_ignore_ascii_case("a_Arm1") {
                        continue; // a_Arm1_Sleeveless already handles a_Arm1 on Viking!
                    }
                    let expected_name = format!("{}_{}", orig_prefix, target_suffix);
                    if !symbols.iter().any(|(_, name)| name.eq_ignore_ascii_case(&expected_name)) {
                        additions.push((*new_id, expected_name));
                        remapped_count += 1;
                    }
                }

                symbols.extend(additions);
                tag.data = serialize_symbol_class(&symbols);
            }
        }

        crate::telemetry::log("INFO", "SWF-IMPORT", &format!("Imported and remapped {} costume symbols (target: '{}', source: '{}')", remapped_count, target_suffix, source_suffix), None);

        Ok(remapped_count)
    }

    /// Imports only the weapon tags and symbols from an external SWF into this SWF for a specific weapon type.
    /// Strictly protects character body parts, eyes, nose, and head from any modifications or blanking.
    pub fn import_weapon_from_swf(
        &mut self,
        source_swf: &SwfPackage,
        source_suffix: &str,
        target_suffix: &str,
        base_weapon: &str,
        color_map: &HashMap<u32, u32>,
    ) -> Result<usize, String> {
        let mut remapped_count = 0;

        // 1. Calculate max character id across definition tags and SymbolClass in self
        let mut max_id: u16 = 0;
        for tag in &self.tags {
            if is_definition_tag(tag.tag_code) && tag.data.len() >= 2 {
                let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if id > max_id {
                    max_id = id;
                }
            }
            if tag.tag_code == 76 {
                if let Ok(syms) = parse_symbol_class(&tag.data) {
                    for (id, _) in syms {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                }
            }
        }

        let offset = max_id.saturating_add(10);

        // 2. Find source symbols matching source_suffix AND is_weapon_symbol_for(prefix, base_weapon)
        let mut actual_source_suffix = source_suffix.to_string();
        let mut source_symbols: Vec<(u16, String)> = Vec::new();

        for tag in &source_swf.tags {
            if tag.tag_code == 76 {
                if let Ok(syms) = parse_symbol_class(&tag.data) {
                    source_symbols.extend(syms);
                }
            }
        }

        let direct_needle = format!("_{}", source_suffix);
        let has_direct_match = source_symbols.iter().any(|(_, name)| name.ends_with(&direct_needle));

        if !has_direct_match {
            for (_, name) in &source_symbols {
                if let Some(pos) = name.rfind(source_suffix) {
                    let candidate = &name[pos..];
                    if !candidate.is_empty() && candidate != source_suffix {
                        actual_source_suffix = candidate.to_string();
                        break;
                    }
                }
            }
        }

        let actual_needle = format!("_{}", actual_source_suffix);
        let mut imported_map: HashMap<String, (u16, String)> = HashMap::new();
        let mut needed_ids: HashSet<u16> = HashSet::new();

        for (old_id, sym_name) in &source_symbols {
            if sym_name.contains("_All") {
                continue;
            }
            if let Some(prefix) = sym_name.strip_suffix(&actual_needle) {
                if is_weapon_symbol_for(prefix, base_weapon) {
                    let new_id = old_id.saturating_add(offset);
                    imported_map.insert(prefix.to_lowercase(), (new_id, prefix.to_string()));
                    needed_ids.insert(*old_id);
                }
            }
        }

        if imported_map.is_empty() {
            return Ok(0);
        }

        // Collect inner tags for needed sprites recursively
        let mut changed = true;
        while changed {
            changed = false;
            for tag in &source_swf.tags {
                if tag.tag_code == 39 && tag.data.len() >= 4 {
                    let sprite_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if needed_ids.contains(&sprite_id) {
                        let mut inner_offset = 4;
                        while inner_offset + 2 <= tag.data.len() {
                            let h = u16::from_le_bytes([tag.data[inner_offset], tag.data[inner_offset + 1]]);
                            let code = h >> 6;
                            let mut len = (h & 0x3F) as usize;
                            let header_size = if len == 0x3F {
                                if inner_offset + 6 > tag.data.len() { break; }
                                len = u32::from_le_bytes([
                                    tag.data[inner_offset + 2],
                                    tag.data[inner_offset + 3],
                                    tag.data[inner_offset + 4],
                                    tag.data[inner_offset + 5],
                                ]) as usize;
                                6
                            } else {
                                2
                            };
                            let body_start = inner_offset + header_size;
                            let next_tag = body_start + len;
                            if next_tag > tag.data.len() { break; }
                            if code == 26 && len >= 5 {
                                let flags = tag.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let inner_char_id = u16::from_le_bytes([tag.data[body_start + 3], tag.data[body_start + 4]]);
                                    if needed_ids.insert(inner_char_id) {
                                        changed = true;
                                    }
                                }
                            } else if code == 70 && len >= 6 {
                                let flags = tag.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let inner_char_id = u16::from_le_bytes([tag.data[body_start + 4], tag.data[body_start + 5]]);
                                    if needed_ids.insert(inner_char_id) {
                                        changed = true;
                                    }
                                }
                            }
                            if code == 0 { break; }
                            inner_offset = next_tag;
                        }
                    }
                }
            }
        }

        // 3. Import definition tags that are in needed_ids
        let mut new_imported_tags = Vec::new();

        for tag in &source_swf.tags {
            if tag.tag_code == 2 || tag.tag_code == 22 || tag.tag_code == 32 || tag.tag_code == 83 {
                if tag.data.len() >= 2 {
                    let old_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if !needed_ids.contains(&old_id) {
                        continue;
                    }
                    let mut tag_copy = tag.clone();
                    let new_id = old_id.saturating_add(offset);
                    tag_copy.data[0] = (new_id & 0xFF) as u8;
                    tag_copy.data[1] = ((new_id >> 8) & 0xFF) as u8;

                    if !color_map.is_empty() && matches!(tag_copy.tag_code, 2 | 22 | 32 | 83) {
                        replace_shape_colors(tag_copy.tag_code, &mut tag_copy.data, color_map);
                    }
                    new_imported_tags.push(tag_copy);
                }
            } else if tag.tag_code == 39 {
                if tag.data.len() >= 4 {
                    let old_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if !needed_ids.contains(&old_id) {
                        continue;
                    }
                    let mut tag_copy = tag.clone();
                    let new_id = old_id.saturating_add(offset);
                    tag_copy.data[0] = (new_id & 0xFF) as u8;
                    tag_copy.data[1] = ((new_id >> 8) & 0xFF) as u8;

                    let mut inner_offset = 4;
                    while inner_offset + 2 <= tag_copy.data.len() {
                        let h = u16::from_le_bytes([tag_copy.data[inner_offset], tag_copy.data[inner_offset + 1]]);
                        let code = h >> 6;
                        let mut len = (h & 0x3F) as usize;
                        let header_size = if len == 0x3F {
                            if inner_offset + 6 > tag_copy.data.len() { break; }
                            len = u32::from_le_bytes([
                                tag_copy.data[inner_offset + 2],
                                tag_copy.data[inner_offset + 3],
                                tag_copy.data[inner_offset + 4],
                                tag_copy.data[inner_offset + 5],
                            ]) as usize;
                            6
                        } else {
                            2
                        };

                        let body_start = inner_offset + header_size;
                        let next_tag = body_start + len;
                        if next_tag > tag_copy.data.len() { break; }

                        if code == 26 && len >= 5 {
                            let flags = tag_copy.data[body_start];
                            if (flags & 0x02) != 0 {
                                let old_char_id = u16::from_le_bytes([
                                    tag_copy.data[body_start + 3],
                                    tag_copy.data[body_start + 4],
                                ]);
                                let new_char_id = old_char_id.saturating_add(offset);
                                tag_copy.data[body_start + 3] = (new_char_id & 0xFF) as u8;
                                tag_copy.data[body_start + 4] = ((new_char_id >> 8) & 0xFF) as u8;
                            }
                        } else if code == 70 && len >= 6 {
                            let flags = tag_copy.data[body_start];
                            if (flags & 0x02) != 0 {
                                let old_char_id = u16::from_le_bytes([
                                    tag_copy.data[body_start + 4],
                                    tag_copy.data[body_start + 5],
                                ]);
                                let new_char_id = old_char_id.saturating_add(offset);
                                tag_copy.data[body_start + 4] = (new_char_id & 0xFF) as u8;
                                tag_copy.data[body_start + 5] = ((new_char_id >> 8) & 0xFF) as u8;
                            }
                        }

                        if code == 0 { break; }
                        inner_offset = next_tag;
                    }

                    new_imported_tags.push(tag_copy);
                }
            }
        }

        // Insert new tags right before SymbolClass
        if let Some(pos) = self.tags.iter().position(|t| t.tag_code == 76) {
            for (i, t) in new_imported_tags.into_iter().enumerate() {
                self.tags.insert(pos + i, t);
            }
        } else {
            self.tags.extend(new_imported_tags);
        }

        // 4. Remap target weapon symbols in self's SymbolClass
        let target_needle = format!("_{}", target_suffix);

        for tag in &mut self.tags {
            if tag.tag_code == 76 {
                let mut symbols = parse_symbol_class(&tag.data)?;
                let mut additions = Vec::new();

                for (sym_id, sym_name) in symbols.iter_mut() {
                    if sym_name.contains("_All") {
                        continue;
                    }
                    if let Some(prefix) = sym_name.strip_suffix(&target_needle) {
                        if is_weapon_symbol_for(prefix, base_weapon) {
                            let prefix_lower = prefix.to_lowercase();
                            if let Some(&(new_id, _)) = imported_map.get(&prefix_lower) {
                                *sym_id = new_id;
                                remapped_count += 1;
                            }
                        }
                    }
                }

                // Add any missing weapon symbols from imported_map
                for (_prefix_lower, (new_id, orig_prefix)) in &imported_map {
                    let expected_name = format!("{}_{}", orig_prefix, target_suffix);
                    if !symbols.iter().any(|(_, name)| name.eq_ignore_ascii_case(&expected_name)) {
                        additions.push((*new_id, expected_name));
                        remapped_count += 1;
                    }
                }

                symbols.extend(additions);
                tag.data = serialize_symbol_class(&symbols);
            }
        }

        crate::telemetry::log("INFO", "SWF-WEAPON-IMPORT", &format!("Imported and remapped {} weapon symbols ({}) from '{}' to '{}'", remapped_count, base_weapon, source_suffix, target_suffix), None);

        Ok(remapped_count)
    }

    /// Renames symbols with `_{from_suffix}` to `_{to_suffix}` (used for cross-SWF costume transplantation).
    pub fn rename_symbols_suffix(&mut self, from_suffix: &str, to_suffix: &str) -> usize {
        let mut count = 0;
        let needle = format!("_{}", from_suffix);
        let target_replacement = format!("_{}", to_suffix);

        for tag in &mut self.tags {
            if tag.tag_code == 76 {
                if let Ok(mut symbols) = parse_symbol_class(&tag.data) {
                    for (_id, name) in symbols.iter_mut() {
                        if let Some(prefix) = name.strip_suffix(&needle) {
                            *name = format!("{}{}", prefix, target_replacement);
                            count += 1;
                        }
                    }
                    tag.data = serialize_symbol_class(&symbols);
                }
            }
        }

        count
    }

    /// Remaps SFX/VFX weapon symbols for crossover/epic skins.
    /// Guarantees character isolation while matching dual hero aliases (e.g. Viking/Bodvar, Pirate/Thatch).
    pub fn remap_sfx_symbols(&mut self, hero_key: &str, swap_suffix: &str) -> usize {
        let mut remapped_count = 0;
        let suffix_needle1 = format!("_{}", swap_suffix);
        let suffix_needle2 = swap_suffix.to_string();
        let hero_lower = hero_key.to_lowercase();
        let hero_alias = match hero_lower.as_str() {
            "viking" => "bodvar",
            "bodvar" => "viking",
            "thatch" => "pirate",
            "pirate" => "thatch",
            "dwarf" => "ulgrim",
            "ulgrim" => "dwarf",
            "gargoyle" => "onyx",
            "onyx" => "gargoyle",
            "sellsword" => "jaeyun",
            "jaeyun" => "sellsword",
            _ => "",
        };

        for tag in &mut self.tags {
            if tag.tag_code == 76 {
                // SymbolClass
                if let Ok(mut symbols) = parse_symbol_class(&tag.data) {
                    let mut swap_map: HashMap<String, u16> = HashMap::new();

                    for (sym_id, sym_name) in &symbols {
                        let is_swap = sym_name.ends_with(&suffix_needle1) || sym_name.ends_with(&suffix_needle2);
                        if is_swap {
                            let sym_lower = sym_name.to_lowercase();
                            let hero_matches = hero_key.is_empty()
                                || sym_lower.contains(&hero_lower)
                                || (!hero_alias.is_empty() && sym_lower.contains(hero_alias))
                                || sym_lower.contains(&swap_suffix.to_lowercase());

                            if hero_matches {
                                let base_name = if let Some(stripped) = sym_name.strip_suffix(&suffix_needle1) {
                                    stripped
                                } else if let Some(stripped) = sym_name.strip_suffix(&suffix_needle2) {
                                    stripped
                                } else {
                                    continue;
                                };
                                swap_map.insert(base_name.to_lowercase(), *sym_id);
                            }
                        }
                    }

                    if swap_map.is_empty() {
                        continue;
                    }

                    for (sym_id, sym_name) in symbols.iter_mut() {
                        let name_lower = sym_name.to_lowercase();
                        if let Some(&new_id) = swap_map.get(&name_lower) {
                            *sym_id = new_id;
                            remapped_count += 1;
                        } else if !hero_lower.is_empty()
                            && (name_lower.contains(&hero_lower) || (!hero_alias.is_empty() && name_lower.contains(hero_alias)))
                            && (name_lower.starts_with("a_sfxattackspecial") || name_lower.starts_with("a_hfxattackspecial") || name_lower.starts_with("a_afxattackspecial") || name_lower.starts_with("a_rlfxattackspecial"))
                        {
                            // Signature attack variant fallback: map un-swapped base special FX to the crossover's authentic special FX
                            let mut sorted_candidates: Vec<(&String, &u16)> = swap_map.iter().collect();
                            sorted_candidates.sort_by_key(|(k, _)| *k);
                            for (candidate_base, new_id) in sorted_candidates {
                                if candidate_base.starts_with("a_sfxattackspecial")
                                    || candidate_base.starts_with("a_hfxattackspecial")
                                    || candidate_base.starts_with("a_afxattackspecial")
                                    || candidate_base.starts_with("a_rlfxattackspecial")
                                    || candidate_base.contains("swooshattackspecial")
                                {
                                    *sym_id = *new_id;
                                    remapped_count += 1;
                                    break;
                                }
                            }
                        }
                    }

                    tag.data = serialize_symbol_class(&symbols);
                }
            }
        }

        remapped_count
    }

    /// Imports SFX/VFX symbols from a standalone crossover SFX SWF (e.g. SFX_Chewbacca.swf, SFX_DestinyTitan.swf)
    /// into a core weapon SFX SWF (e.g. SFX_Sword.swf, SFX_Cannon.swf).
    pub fn import_sfx_symbols_from_swf(
        &mut self,
        source_swf: &SwfPackage,
        swap_suffix: &str,
    ) -> Result<usize, String> {
        let mut remapped_count = 0;

        // 1. Calculate max character id in self
        let mut max_id: u16 = 0;
        for tag in &self.tags {
            if is_definition_tag(tag.tag_code) && tag.data.len() >= 2 {
                let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if id > max_id {
                    max_id = id;
                }
            }
            if tag.tag_code == 76 {
                if let Ok(syms) = parse_symbol_class(&tag.data) {
                    for (id, _) in syms {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                }
            }
        }
        let offset = max_id.saturating_add(10);

        // 2. Read destination symbols in self
        let mut dest_symbol_names: HashSet<String> = HashSet::new();
        for tag in &self.tags {
            if tag.tag_code == 76 {
                if let Ok(syms) = parse_symbol_class(&tag.data) {
                    for (_, name) in syms {
                        dest_symbol_names.insert(name.to_lowercase());
                    }
                }
            }
        }

        // 3. Find source symbols matching swap_suffix whose base name exists in self
        let suffix_needle1 = format!("_{}", swap_suffix);
        let suffix_needle2 = swap_suffix.to_string();
        let mut imported_map: HashMap<String, (u16, u16)> = HashMap::new(); // base_name_lower -> (old_id, new_id)

        for tag in &source_swf.tags {
            if tag.tag_code == 76 {
                if let Ok(syms) = parse_symbol_class(&tag.data) {
                    for (old_id, sym_name) in syms {
                        let base_name = if let Some(stripped) = sym_name.strip_suffix(&suffix_needle1) {
                            stripped
                        } else if let Some(stripped) = sym_name.strip_suffix(&suffix_needle2) {
                            stripped
                        } else {
                            continue;
                        };
                        let base_lower = base_name.to_lowercase();
                        if dest_symbol_names.contains(&base_lower) {
                            let new_id = old_id.saturating_add(offset);
                            imported_map.insert(base_lower, (old_id, new_id));
                        }
                    }
                }
            }
        }

        if imported_map.is_empty() {
            return Ok(0);
        }

        // 4. Collect needed character IDs recursively from source_swf
        let mut needed_ids: HashSet<u16> = HashSet::new();
        for (_, (old_id, _)) in &imported_map {
            needed_ids.insert(*old_id);
        }

        let mut changed = true;
        while changed {
            changed = false;
            for tag in &source_swf.tags {
                if tag.tag_code == 39 && tag.data.len() >= 4 {
                    let sprite_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                    if needed_ids.contains(&sprite_id) {
                        let mut inner_offset = 4;
                        while inner_offset + 2 <= tag.data.len() {
                            let h = u16::from_le_bytes([tag.data[inner_offset], tag.data[inner_offset + 1]]);
                            let code = h >> 6;
                            let mut len = (h & 0x3F) as usize;
                            let header_size = if len == 0x3F {
                                if inner_offset + 6 > tag.data.len() { break; }
                                len = u32::from_le_bytes([
                                    tag.data[inner_offset + 2],
                                    tag.data[inner_offset + 3],
                                    tag.data[inner_offset + 4],
                                    tag.data[inner_offset + 5],
                                ]) as usize;
                                6
                            } else {
                                2
                            };

                            let body_start = inner_offset + header_size;
                            let next_tag = body_start + len;
                            if next_tag > tag.data.len() { break; }

                            if code == 26 && len >= 5 {
                                let flags = tag.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let inner_char_id = u16::from_le_bytes([tag.data[body_start + 3], tag.data[body_start + 4]]);
                                    if needed_ids.insert(inner_char_id) {
                                        changed = true;
                                    }
                                }
                            } else if code == 70 && len >= 6 {
                                let flags = tag.data[body_start];
                                if (flags & 0x02) != 0 {
                                    let inner_char_id = u16::from_le_bytes([tag.data[body_start + 4], tag.data[body_start + 5]]);
                                    if needed_ids.insert(inner_char_id) {
                                        changed = true;
                                    }
                                }
                            }

                            if code == 0 { break; }
                            inner_offset = next_tag;
                        }
                    }
                }
            }
        }

        // 5. Clone and offset definition tags from source_swf
        let mut new_tags = Vec::new();
        for tag in &source_swf.tags {
            if matches!(tag.tag_code, 2 | 22 | 32 | 83) && tag.data.len() >= 2 {
                let old_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if needed_ids.contains(&old_id) {
                    let mut tag_copy = tag.clone();
                    let new_id = old_id.saturating_add(offset);
                    tag_copy.data[0] = (new_id & 0xFF) as u8;
                    tag_copy.data[1] = ((new_id >> 8) & 0xFF) as u8;
                    new_tags.push(tag_copy);
                }
            } else if tag.tag_code == 39 && tag.data.len() >= 4 {
                let old_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if needed_ids.contains(&old_id) {
                    let mut tag_copy = tag.clone();
                    let new_id = old_id.saturating_add(offset);
                    tag_copy.data[0] = (new_id & 0xFF) as u8;
                    tag_copy.data[1] = ((new_id >> 8) & 0xFF) as u8;

                    // Remap inner PlaceObject tags
                    let mut inner_offset = 4;
                    while inner_offset + 2 <= tag_copy.data.len() {
                        let h = u16::from_le_bytes([tag_copy.data[inner_offset], tag_copy.data[inner_offset + 1]]);
                        let code = h >> 6;
                        let mut len = (h & 0x3F) as usize;
                        let header_size = if len == 0x3F {
                            if inner_offset + 6 > tag_copy.data.len() { break; }
                            len = u32::from_le_bytes([
                                tag_copy.data[inner_offset + 2],
                                tag_copy.data[inner_offset + 3],
                                tag_copy.data[inner_offset + 4],
                                tag_copy.data[inner_offset + 5],
                            ]) as usize;
                            6
                        } else {
                            2
                        };

                        let body_start = inner_offset + header_size;
                        let next_tag = body_start + len;
                        if next_tag > tag_copy.data.len() { break; }

                        if code == 26 && len >= 5 {
                            let flags = tag_copy.data[body_start];
                            if (flags & 0x02) != 0 {
                                let inner_char_id = u16::from_le_bytes([tag_copy.data[body_start + 3], tag_copy.data[body_start + 4]]);
                                let new_inner_id = inner_char_id.saturating_add(offset);
                                tag_copy.data[body_start + 3] = (new_inner_id & 0xFF) as u8;
                                tag_copy.data[body_start + 4] = ((new_inner_id >> 8) & 0xFF) as u8;
                            }
                        } else if code == 70 && len >= 6 {
                            let flags = tag_copy.data[body_start];
                            if (flags & 0x02) != 0 {
                                let inner_char_id = u16::from_le_bytes([tag_copy.data[body_start + 4], tag_copy.data[body_start + 5]]);
                                let new_inner_id = inner_char_id.saturating_add(offset);
                                tag_copy.data[body_start + 4] = (new_inner_id & 0xFF) as u8;
                                tag_copy.data[body_start + 5] = ((new_inner_id >> 8) & 0xFF) as u8;
                            }
                        }

                        if code == 0 { break; }
                        inner_offset = next_tag;
                    }

                    new_tags.push(tag_copy);
                }
            }
        }

        // 6. Insert new definition tags before SymbolClass
        let insert_idx = self.tags.iter().position(|t| t.tag_code == 76).unwrap_or(self.tags.len());
        for (i, t) in new_tags.into_iter().enumerate() {
            self.tags.insert(insert_idx + i, t);
        }

        // 7. In SymbolClass (tag 76), remap matching destination symbols to their new IDs
        for tag in &mut self.tags {
            if tag.tag_code == 76 {
                if let Ok(mut symbols) = parse_symbol_class(&tag.data) {
                    for (sym_id, sym_name) in symbols.iter_mut() {
                        let name_lower = sym_name.to_lowercase();
                        if let Some(&(_, new_id)) = imported_map.get(&name_lower) {
                            if *sym_id != new_id {
                                *sym_id = new_id;
                                remapped_count += 1;
                            }
                        }
                    }
                    tag.data = serialize_symbol_class(&symbols);
                }
            }
        }

        Ok(remapped_count)
    }

    pub fn apply_color_palette(&mut self, color_map: &HashMap<u32, u32>) -> usize {
        if color_map.is_empty() {
            return 0;
        }

        let mut replaced = 0;
        for tag in &mut self.tags {
            // DefineShape tags: 2, 22, 32, 83
            if matches!(tag.tag_code, 2 | 22 | 32 | 83) {
                replaced += replace_shape_colors(tag.tag_code, &mut tag.data, color_map);
            }
        }
        replaced
    }

    pub fn get_symbol_classes(&self) -> Result<Vec<(u16, String)>, String> {
        let mut all = Vec::new();
        for tag in &self.tags {
            if tag.tag_code == 76 {
                let syms = parse_symbol_class(&tag.data)?;
                all.extend(syms);
            }
        }
        Ok(all)
    }

    /// Comprehensive structural validation of this SWF package.
    /// Ensures 100% binary, tag, symbol, and dependency integrity before writing to disk.
    pub fn validate(&self) -> Result<(), String> {
        if self.version < 8 {
            return Err(format!("Invalid SWF version: {}", self.version));
        }
        if self.header_rect_and_frames.len() < 5 {
            return Err("SWF header bounds and frames too short".into());
        }
        if self.tags.is_empty() {
            return Err("SWF contains no tags".into());
        }

        // Tag stream must end with EndTag (tag_code 0)
        let last_tag = self.tags.last().unwrap();
        if last_tag.tag_code != 0 {
            return Err(format!("SWF tag stream does not terminate with EndTag (last tag code: {})", last_tag.tag_code));
        }

        // 1. Collect all defined character IDs
        let mut def_ids: HashSet<u16> = HashSet::new();
        for tag in &self.tags {
            if is_definition_tag(tag.tag_code) && tag.data.len() >= 2 {
                let id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                if id == 0 {
                    return Err(format!("Invalid definition tag {} with ID 0", tag.tag_code));
                }
                def_ids.insert(id);
            }
        }

        // 2. Validate all SymbolClass references point to existing definition tags
        for tag in &self.tags {
            if tag.tag_code == 76 {
                let syms = parse_symbol_class(&tag.data)?;
                for (sym_id, sym_name) in syms {
                    if sym_id == 0 {
                        return Err(format!("SymbolClass has invalid 0 ID for symbol '{}'", sym_name));
                    }
                    if !def_ids.contains(&sym_id) {
                        return Err(format!(
                            "SymbolClass symbol '{}' references non-existent definition tag ID {}",
                            sym_name, sym_id
                        ));
                    }
                }
            }
        }

        // 3. Validate all DefineSprite inner PlaceObject character references
        for tag in &self.tags {
            if tag.tag_code == 39 {
                if tag.data.len() < 4 {
                    return Err("DefineSprite tag data shorter than 4 bytes".into());
                }
                let sprite_id = u16::from_le_bytes([tag.data[0], tag.data[1]]);
                let mut offset = 4;
                while offset + 2 <= tag.data.len() {
                    let h = u16::from_le_bytes([tag.data[offset], tag.data[offset + 1]]);
                    let code = h >> 6;
                    let mut len = (h & 0x3F) as usize;
                    let header_size = if len == 0x3F {
                        if offset + 6 > tag.data.len() {
                            return Err(format!("Sprite {} has truncated long tag header", sprite_id));
                        }
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
                    if next_tag > tag.data.len() {
                        return Err(format!("Sprite {} inner tag {} extends past sprite boundary", sprite_id, code));
                    }

                    if code == 26 && len >= 5 {
                        let flags = tag.data[body_start];
                        if (flags & 0x02) != 0 {
                            let inner_char_id = u16::from_le_bytes([tag.data[body_start + 3], tag.data[body_start + 4]]);
                            if !def_ids.contains(&inner_char_id) {
                                return Err(format!(
                                    "Sprite {} PlaceObject2 references non-existent character ID {}",
                                    sprite_id, inner_char_id
                                ));
                            }
                        }
                    } else if code == 70 && len >= 6 {
                        let flags1 = tag.data[body_start];
                        if (flags1 & 0x02) != 0 {
                            let inner_char_id = u16::from_le_bytes([tag.data[body_start + 4], tag.data[body_start + 5]]);
                            if !def_ids.contains(&inner_char_id) {
                                return Err(format!(
                                    "Sprite {} PlaceObject3 references non-existent character ID {}",
                                    sprite_id, inner_char_id
                                ));
                            }
                        }
                    }

                    if code == 0 {
                        break;
                    }
                    offset = next_tag;
                }
            }
        }

        Ok(())
    }
}

pub fn parse_symbol_class(data: &[u8]) -> Result<Vec<(u16, String)>, String> {
    if data.len() < 2 {
        return Ok(Vec::new());
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let mut symbols = Vec::with_capacity(count);
    let mut offset = 2;

    for _ in 0..count {
        if offset + 2 > data.len() {
            break;
        }
        let sym_id = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let str_start = offset;
        while offset < data.len() && data[offset] != 0 {
            offset += 1;
        }
        let sym_name = String::from_utf8_lossy(&data[str_start..offset]).to_string();
        offset += 1; // skip null byte

        symbols.push((sym_id, sym_name));
    }

    Ok(symbols)
}

pub fn serialize_symbol_class(symbols: &[(u16, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(symbols.len() as u16).to_le_bytes());
    for (sym_id, sym_name) in symbols {
        out.extend_from_slice(&sym_id.to_le_bytes());
        out.extend_from_slice(sym_name.as_bytes());
        out.push(0); // null terminator
    }
    out
}

pub struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, bit_pos: 0 }
    }

    pub fn read_bits(&mut self, n: usize) -> Option<u32> {
        if n == 0 {
            return Some(0);
        }
        let mut val = 0u32;
        for _ in 0..n {
            let byte_idx = self.bit_pos / 8;
            let bit_idx = 7 - (self.bit_pos % 8);
            if byte_idx >= self.data.len() {
                return None;
            }
            let bit = ((self.data[byte_idx] >> bit_idx) & 1) as u32;
            val = (val << 1) | bit;
            self.bit_pos += 1;
        }
        Some(val)
    }

    pub fn align_byte(&mut self) {
        let rem = self.bit_pos % 8;
        if rem != 0 {
            self.bit_pos += 8 - rem;
        }
    }

    pub fn byte_offset(&self) -> usize {
        (self.bit_pos + 7) / 8
    }
}

pub fn skip_rect(reader: &mut BitReader) -> bool {
    if let Some(nbits) = reader.read_bits(5) {
        let total = (nbits as usize) * 4;
        if reader.read_bits(total).is_some() {
            reader.align_byte();
            return true;
        }
    }
    false
}

pub fn skip_matrix(reader: &mut BitReader) -> bool {
    let has_scale = match reader.read_bits(1) {
        Some(v) => v,
        None => return false,
    };
    if has_scale == 1 {
        let n_scale_bits = match reader.read_bits(5) {
            Some(v) => v as usize,
            None => return false,
        };
        if reader.read_bits(n_scale_bits * 2).is_none() {
            return false;
        }
    }
    let has_rotate = match reader.read_bits(1) {
        Some(v) => v,
        None => return false,
    };
    if has_rotate == 1 {
        let n_rotate_bits = match reader.read_bits(5) {
            Some(v) => v as usize,
            None => return false,
        };
        if reader.read_bits(n_rotate_bits * 2).is_none() {
            return false;
        }
    }
    let n_translate_bits = match reader.read_bits(5) {
        Some(v) => v as usize,
        None => return false,
    };
    if reader.read_bits(n_translate_bits * 2).is_none() {
        return false;
    }
    reader.align_byte();
    true
}

/// Safely replaces colors strictly within the FILLSTYLEARRAY and LINESTYLEARRAY of DefineShape tags.
/// NEVER modifies bytes inside the SHAPERECORDS bitstream, preserving 100% vector geometry integrity.
pub fn replace_shape_colors(tag_code: u16, data: &mut [u8], map: &HashMap<u32, u32>) -> usize {
    if map.is_empty() || data.len() < 4 {
        return 0;
    }
    let mut replacements: Vec<(usize, u32)> = Vec::new();
    let mut reader = BitReader::new(data);

    // Skip ShapeId (2 bytes)
    if reader.read_bits(16).is_none() {
        return 0;
    }
    // Skip ShapeBounds RECT
    if !skip_rect(&mut reader) {
        return 0;
    }

    if tag_code == 83 {
        // DefineShape4: EdgeBounds RECT + Flags byte
        if !skip_rect(&mut reader) {
            return 0;
        }
        if reader.read_bits(8).is_none() {
            return 0;
        }
    }

    // 1. FILLSTYLEARRAY
    let mut fill_count = match reader.read_bits(8) {
        Some(v) => v as usize,
        None => return 0,
    };
    if tag_code != 2 && fill_count == 0xFF {
        let low = match reader.read_bits(8) {
            Some(v) => v as usize,
            None => return 0,
        };
        let high = match reader.read_bits(8) {
            Some(v) => v as usize,
            None => return 0,
        };
        fill_count = (high << 8) | low;
    }

    for _ in 0..fill_count {
        let fill_type = match reader.read_bits(8) {
            Some(v) => v as u8,
            None => break,
        };
        if fill_type == 0x00 {
            // Solid fill
            let byte_idx = reader.byte_offset();
            if tag_code == 32 || tag_code == 83 {
                // RGBA (4 bytes)
                if byte_idx + 3 < data.len() {
                    let rgb = ((data[byte_idx] as u32) << 16)
                        | ((data[byte_idx + 1] as u32) << 8)
                        | (data[byte_idx + 2] as u32);
                    if let Some(&new_color) = map.get(&rgb) {
                        replacements.push((byte_idx, new_color));
                    }
                }
                if reader.read_bits(32).is_none() {
                    break;
                }
            } else {
                // RGB (3 bytes)
                if byte_idx + 2 < data.len() {
                    let rgb = ((data[byte_idx] as u32) << 16)
                        | ((data[byte_idx + 1] as u32) << 8)
                        | (data[byte_idx + 2] as u32);
                    if let Some(&new_color) = map.get(&rgb) {
                        replacements.push((byte_idx, new_color));
                    }
                }
                if reader.read_bits(24).is_none() {
                    break;
                }
            }
        } else if fill_type == 0x10 || fill_type == 0x12 || fill_type == 0x13 {
            // Gradient
            if !skip_matrix(&mut reader) {
                break;
            }
            let grad_header = match reader.read_bits(8) {
                Some(v) => v,
                None => break,
            };
            let num_grads = (grad_header & 0x0F) as usize;
            let is_rgba = tag_code == 32 || tag_code == 83;
            for _ in 0..num_grads {
                // Skip ratio (1 byte)
                if reader.read_bits(8).is_none() {
                    break;
                }
                let color_idx = reader.byte_offset();
                if is_rgba {
                    if color_idx + 3 < data.len() {
                        let rgb = ((data[color_idx] as u32) << 16)
                            | ((data[color_idx + 1] as u32) << 8)
                            | (data[color_idx + 2] as u32);
                        if let Some(&new_color) = map.get(&rgb) {
                            replacements.push((color_idx, new_color));
                        }
                    }
                    if reader.read_bits(32).is_none() {
                        break;
                    }
                } else {
                    if color_idx + 2 < data.len() {
                        let rgb = ((data[color_idx] as u32) << 16)
                            | ((data[color_idx + 1] as u32) << 8)
                            | (data[color_idx + 2] as u32);
                        if let Some(&new_color) = map.get(&rgb) {
                            replacements.push((color_idx, new_color));
                        }
                    }
                    if reader.read_bits(24).is_none() {
                        break;
                    }
                }
            }
            if fill_type == 0x13 {
                // FocalPoint: 2 bytes comes AFTER gradient records in SWF specification
                if reader.read_bits(16).is_none() {
                    break;
                }
            }
        } else if fill_type >= 0x40 && fill_type <= 0x43 {
            // Bitmap: BitmapId (16 bits) + Matrix
            if reader.read_bits(16).is_none() {
                break;
            }
            if !skip_matrix(&mut reader) {
                break;
            }
        } else {
            break;
        }
    }

    // 2. LINESTYLEARRAY
    let mut line_count = match reader.read_bits(8) {
        Some(v) => v as usize,
        None => 0,
    };
    if tag_code != 2 && line_count == 0xFF {
        let low = match reader.read_bits(8) {
            Some(v) => v as usize,
            None => 0,
        };
        let high = match reader.read_bits(8) {
            Some(v) => v as usize,
            None => 0,
        };
        line_count = (high << 8) | low;
    }

    for _ in 0..line_count {
        // Width (16 bits)
        if reader.read_bits(16).is_none() {
            break;
        }
        if tag_code == 2 || tag_code == 22 {
            let color_idx = reader.byte_offset();
            if color_idx + 2 < data.len() {
                let rgb = ((data[color_idx] as u32) << 16)
                    | ((data[color_idx + 1] as u32) << 8)
                    | (data[color_idx + 2] as u32);
                if let Some(&new_color) = map.get(&rgb) {
                    replacements.push((color_idx, new_color));
                }
            }
            if reader.read_bits(24).is_none() {
                break;
            }
        } else if tag_code == 32 {
            let color_idx = reader.byte_offset();
            if color_idx + 3 < data.len() {
                let rgb = ((data[color_idx] as u32) << 16)
                    | ((data[color_idx + 1] as u32) << 8)
                    | (data[color_idx + 2] as u32);
                if let Some(&new_color) = map.get(&rgb) {
                    replacements.push((color_idx, new_color));
                }
            }
            if reader.read_bits(32).is_none() {
                break;
            }
        } else if tag_code == 83 {
            // LineStyle2 in DefineShape4
            let flags = match reader.read_bits(16) {
                Some(v) => v as u16,
                None => break,
            };
            if (flags & 0x08) != 0 {
                // HasFill: parse fill style
                let fill_type = match reader.read_bits(8) {
                    Some(v) => v as u8,
                    None => break,
                };
                if fill_type == 0x00 {
                    let color_idx = reader.byte_offset();
                    if color_idx + 3 < data.len() {
                        let rgb = ((data[color_idx] as u32) << 16)
                            | ((data[color_idx + 1] as u32) << 8)
                            | (data[color_idx + 2] as u32);
                        if let Some(&new_color) = map.get(&rgb) {
                            replacements.push((color_idx, new_color));
                        }
                    }
                    if reader.read_bits(32).is_none() {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                let color_idx = reader.byte_offset();
                if color_idx + 3 < data.len() {
                    let rgb = ((data[color_idx] as u32) << 16)
                        | ((data[color_idx + 1] as u32) << 8)
                        | (data[color_idx + 2] as u32);
                    if let Some(&new_color) = map.get(&rgb) {
                        replacements.push((color_idx, new_color));
                    }
                }
                if reader.read_bits(32).is_none() {
                    break;
                }
            }
            if (flags & 0x04) != 0 {
                // Miter limit factor: 2 bytes
                if reader.read_bits(16).is_none() {
                    break;
                }
            }
        }
    }

    let count = replacements.len();
    for (offset, new_color) in replacements {
        if offset + 2 < data.len() {
            data[offset] = ((new_color >> 16) & 0xFF) as u8;
            data[offset + 1] = ((new_color >> 8) & 0xFF) as u8;
            data[offset + 2] = (new_color & 0xFF) as u8;
        }
    }

    count
}
