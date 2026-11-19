use flate2::read::ZlibDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

pub struct SwzRandom {
    state: [u32; 16],
    index: usize,
}

impl SwzRandom {
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; 16];
        state[0] = seed;
        for i in 1..16 {
            state[i] = (i as u32)
                .wrapping_add(0x6C078965u32.wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30)));
        }
        Self { state, index: 0 }
    }

    pub fn next(&mut self) -> u32 {
        let mut a = self.state[self.index];
        let mut b = self.state[(self.index + 13) % 16];
        let c = a ^ (a << 16) ^ b ^ (b << 15);
        b = self.state[(self.index + 9) % 16];
        b ^= b >> 11;
        self.state[self.index] = b ^ c;
        a = self.state[self.index];
        let d = a ^ ((a << 5) & 0xDA442D24);
        self.index = (self.index + 15) % 16;
        a = self.state[self.index];
        self.state[self.index] = a ^ (a << 2) ^ (b << 28) ^ c ^ (c << 18) ^ d;
        self.state[self.index]
    }
}

pub fn calculate_key_checksum(key: u32, rand: &mut SwzRandom) -> u32 {
    let mut checksum = 0x2DF4A1CDu32;
    let rounds = (key % 31) + 5;
    for _ in 0..rounds {
        checksum ^= rand.next();
    }
    checksum
}

pub fn key_validates(swz_header: &[u8], key: u32) -> bool {
    if swz_header.len() < 8 {
        return false;
    }
    let checksum = u32::from_be_bytes([swz_header[0], swz_header[1], swz_header[2], swz_header[3]]);
    let seed = u32::from_be_bytes([swz_header[4], swz_header[5], swz_header[6], swz_header[7]]);
    let mut rand = SwzRandom::new(key ^ seed);
    calculate_key_checksum(key, &mut rand) == checksum
}

pub fn find_swz_key(air_swf_path: &Path, swz_path: &Path) -> Option<u32> {
    let mut swz_file = File::open(swz_path).ok()?;
    let mut header = [0u8; 8];
    swz_file.read_exact(&mut header).ok()?;

    // 1. Try default key first
    let default_key = 154281100u32;
    if key_validates(&header, default_key) {
        return Some(default_key);
    }

    // 2. Scan BrawlhallaAir.swf for LEB128 key
    let mut swf_file = File::open(air_swf_path).ok()?;
    let mut swf_data = Vec::new();
    swf_file.read_to_end(&mut swf_data).ok()?;

    if swf_data.len() < 8 {
        return None;
    }

    let body = if &swf_data[0..3] == b"CWS" {
        let mut decoder = ZlibDecoder::new(&swf_data[8..]);
        let mut decomp = Vec::new();
        decoder.read_to_end(&mut decomp).ok()?;
        decomp
    } else {
        swf_data[8..].to_vec()
    };

    if body.len() > 5 {
        for i in 0..body.len() - 5 {
            let mut val = 0u32;
            let mut shift = 0;
            for offset in 0..5 {
                let b = body[i + offset];
                val |= ((b & 0x7F) as u32) << shift;
                if (b & 0x80) == 0 {
                    break;
                }
                shift += 7;
                if shift > 28 {
                    break;
                }
            }
            if val > 100_000 && key_validates(&header, val) {
                return Some(val);
            }
        }
    }

    None
}

pub fn read_swz_files(swz_path: &Path, key: u32) -> Result<HashMap<String, String>, String> {
    let file = File::open(swz_path).map_err(|e| format!("Failed to open Game.swz: {}", e))?;
    let mut file = BufReader::with_capacity(65536, file);
    let mut header = [0u8; 8];
    file.read_exact(&mut header)
        .map_err(|e| format!("Failed to read SWZ header: {}", e))?;

    let _checksum = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let seed = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);

    let mut rand = SwzRandom::new(seed ^ key);
    let _calc_key_chk = calculate_key_checksum(key, &mut rand);

    let mut result = HashMap::new();
    let mut len_buf = [0u8; 4];

    while file.read_exact(&mut len_buf).is_ok() {
        let comp_size = u32::from_be_bytes(len_buf) ^ rand.next();

        if comp_size == 0 || comp_size > 64 * 1024 * 1024 {
            break;
        }

        if file.read_exact(&mut len_buf).is_err() {
            break;
        }
        let decomp_size = (u32::from_be_bytes(len_buf) ^ rand.next()) as usize;

        if file.read_exact(&mut len_buf).is_err() {
            break;
        }
        let _file_checksum = u32::from_be_bytes(len_buf);

        let mut encrypted = vec![0u8; comp_size as usize];
        if file.read_exact(&mut encrypted).is_err() {
            break;
        }

        // Decrypt bytes
        let mut chk = rand.next();
        for (i, b) in encrypted.iter_mut().enumerate() {
            *b ^= (rand.next() >> ((i % 16) as u32)) as u8;
            chk = (*b as u32) ^ chk.rotate_right((i % 7 + 1) as u32);
        }

        // Decompress with zlib with pre-allocated buffer
        let mut decoder = ZlibDecoder::new(Cursor::new(&encrypted));
        let cap = decomp_size.min(64 * 1024 * 1024);
        let mut decompressed = Vec::with_capacity(cap);
        if decoder.read_to_end(&mut decompressed).is_ok() {
            if let Ok(content) = String::from_utf8(decompressed) {
                let name = get_swz_file_name(&content);
                result.insert(name, content);
            }
        }
    }

    Ok(result)
}

fn get_swz_file_name(content: &str) -> String {
    let trimmed = content.trim_start();
    if trimmed.starts_with('<') {
        // XML document
        if let Some(start_tag) = trimmed.find('<') {
            let rest = &trimmed[start_tag + 1..];
            let end_tag = rest.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(rest.len());
            let root_name = &rest[..end_tag];

            if root_name == "LevelDesc" {
                if let Some(pos) = rest.find("LevelName=\"") {
                    let val = &rest[pos + 11..];
                    if let Some(end_val) = val.find('"') {
                        return format!("LevelDesc_{}.xml", &val[..end_val]);
                    }
                }
            } else if root_name == "CutsceneType" {
                if let Some(pos) = rest.find("CutsceneName=\"") {
                    let val = &rest[pos + 14..];
                    if let Some(end_val) = val.find('"') {
                        return format!("CutsceneType_{}.xml", &val[..end_val]);
                    }
                }
            }

            return format!("{}.xml", root_name);
        }
        "Unknown.xml".to_string()
    } else {
        // CSV document: first line is table name
        if let Some(line) = content.lines().next() {
            format!("{}.csv", line.trim())
        } else {
            "Unknown.csv".to_string()
        }
    }
}
