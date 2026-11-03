use crate::models::{ColorSchemeInfo, CostumeInfo, GameInstallation, LegendInfo, WeaponSkinInfo};
use crate::swz::{find_swz_key, read_swz_files};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Curated list of supported legends to focus strictly on quality, precision, and zero-defect cosmetic mods.
/// Keys match hero_name, display_name, or bio (case-insensitive).
pub const CURATED_HERO_KEYS: &[&str] = &[
    // Core Curated Legends (5)
    "viking",    // Bodvar / Bödvar
    "bodvar",
    "bödvar",
    "thatch",    // Thatch
    "pirate",
    "dwarf",     // Ulgrim
    "ulgrim",
    "gargoyle",  // Onyx
    "onyx",
    "sellsword", // Jaeyun
    "jaeyun",
    // Restored Legends (15)
    "skeleton",         // Azoth
    "azoth",
    "actualrobot",      // Vector
    "vector",
    "vamplord",         // Volkov
    "volkov",
    "birdbard",         // Munin
    "munin",
    "roboengineer",     // Seven
    "seven",
    "greekspeedster",   // Thea
    "thea",
    "loki",             // Loki
    "imugi",            // Imugi
    "africanking",      // King Zuva
    "king zuva",
    "zuva",
    "bladedancer",      // Priya
    "priya",
    "cybervirus",       // Ransom
    "ransom",
    "cleric",           // Lady Vera
    "lady vera",
    "vera",
    "darkheartmonster", // Rupture
    "rupture",
    "actualgladiator",  // Aurus
    "aurus",
    "astrogirl",        // Qinghua & Baobao
    "qinghua",
    "baobao",
];

pub fn is_curated_hero(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }
    // Explicit exclusion check: never reintroduce Artemis, Yumiko, or Magyar
    if lower.contains("spacegirl") || lower.contains("artemis")
        || lower.contains("foxgirl") || lower.contains("yumiko")
        || lower.contains("hauntedarmor") || lower.contains("magyar")
    {
        return false;
    }
    CURATED_HERO_KEYS.iter().any(|&k| lower == k || lower.contains(k))
}

pub fn is_hero_weapon_compatible(hero: &str, base_weapon: &str) -> bool {
    let h = hero.trim().to_lowercase();
    let bw = base_weapon.trim().to_lowercase();
    match h.as_str() {
        "viking" | "bodvar" | "bödvar" => bw == "hammer" || bw == "sword",
        "thatch" | "pirate" => bw == "sword" || bw == "pistol" || bw == "blasters" || bw == "blaster",
        "dwarf" | "ulgrim" => bw == "axe" || bw == "rocketlance" || bw == "lance",
        "gargoyle" | "onyx" => bw == "gauntlets" || bw == "gauntlet" || bw == "cannon" || bw == "fists",
        "sellsword" | "jaeyun" => bw == "sword" || bw == "greatsword" || bw == "great",
        // 15 restored legends:
        "skeleton" | "azoth" => bw == "bow" || bw == "axe",
        "actualrobot" | "vector" => bw == "rocketlance" || bw == "lance" || bw == "bow",
        "vamplord" | "volkov" => bw == "axe" || bw == "scythe",
        "birdbard" | "munin" => bw == "bow" || bw == "scythe",
        "roboengineer" | "seven" => bw == "spear" || bw == "cannon",
        "greekspeedster" | "thea" => bw == "boots" || bw == "battleboots" || bw == "rocketlance" || bw == "lance",
        "loki" => bw == "katar" || bw == "katars" || bw == "scythe",
        "imugi" => bw == "axe" || bw == "greatsword" || bw == "great",
        "africanking" | "king zuva" | "zuva" => bw == "hammer" || bw == "boots" || bw == "battleboots",
        "bladedancer" | "priya" => bw == "chakram" || bw == "sword",
        "cybervirus" | "ransom" => bw == "chakram" || bw == "bow",
        "cleric" | "lady vera" | "vera" => bw == "chakram" || bw == "scythe",
        "darkheartmonster" | "rupture" => bw == "katar" || bw == "katars" || bw == "rocketlance" || bw == "lance",
        "actualgladiator" | "aurus" => bw == "chakram" || bw == "spear",
        "astrogirl" | "qinghua" | "baobao" => bw == "orb" || bw == "cannon",
        _ => false,
    }
}

#[derive(Default, Clone, Debug)]
pub struct Catalog {
    pub legends: Vec<LegendInfo>,
    pub costumes: Vec<CostumeInfo>,
    pub weapon_skins: Vec<WeaponSkinInfo>,
    pub color_schemes: Vec<ColorSchemeInfo>,
    pub weapon_skins_by_type: HashMap<String, Vec<WeaponSkinInfo>>,
    pub is_loaded: bool,
    pub load_error: String,
}

impl Catalog {
    /// Filters the loaded catalog to only include the curated, quality-tested legends (Bodvar, Thatch, Ulgrim, Onyx, Jaeyun)
    /// and their associated costumes and weapon skins.
    pub fn filter_curated_legends(&mut self) {
        self.legends.retain(|l| {
            is_curated_hero(&l.hero_name) || is_curated_hero(&l.display_name) || is_curated_hero(&l.bio)
        });

        let allowed_hero_names: HashSet<String> = self
            .legends
            .iter()
            .map(|l| l.hero_name.to_lowercase())
            .collect();

        self.costumes.retain(|c| {
            allowed_hero_names.contains(&c.owner_hero.to_lowercase())
                || is_curated_hero(&c.owner_hero)
        });

        // Retain all weapon skins matching weapon types wielded by curated heroes.
        // In Brawlhalla, weapon skins are universal across all heroes who can wield that base weapon type.
        // Preserves authentic owner_hero, SWF file, symbol, ID, and associated data without artificial mutation.
        let curated_weapon_types: HashSet<String> = self
            .legends
            .iter()
            .flat_map(|l| vec![l.weapon1.trim().to_lowercase(), l.weapon2.trim().to_lowercase()])
            .filter(|w| !w.is_empty())
            .collect();

        self.weapon_skins.retain_mut(|w| {
            let bw = w.base_weapon.trim().to_lowercase();
            let matches_type = curated_weapon_types.contains(&bw)
                || curated_weapon_types.iter().any(|cw| {
                    match cw.as_str() {
                        "pistol" | "blasters" | "blaster" => bw == "pistol" || bw == "blasters" || bw == "blaster",
                        "rocketlance" | "lance" => bw == "rocketlance" || bw == "lance",
                        "gauntlet" | "gauntlets" | "fists" => bw == "gauntlet" || bw == "gauntlets" || bw == "fists",
                        "greatsword" | "great" => bw == "greatsword" || bw == "great",
                        _ => cw == &bw,
                    }
                });

            if !matches_type {
                return false;
            }

            // Authentic owner hero inference from SWF if owner is unset
            if (w.owner_hero.is_empty() || w.owner_hero == "--") && w.swf_file.starts_with("Gfx_") && !w.swf_file.starts_with("Gfx_Weapons") {
                let hero_name = w.swf_file
                    .trim_start_matches("Gfx_")
                    .trim_end_matches(".swf");
                if !hero_name.is_empty() {
                    w.owner_hero = hero_name.to_string();
                }
            }

            true
        });

        self.weapon_skins_by_type.clear();
        for ws in &self.weapon_skins {
            if ws.base_weapon.trim().is_empty() {
                continue;
            }
            self.weapon_skins_by_type
                .entry(ws.base_weapon.to_lowercase())
                .or_default()
                .push(ws.clone());
        }
    }

    pub fn load_from_installation(&mut self, installation: &GameInstallation) -> Result<(), String> {
        if !installation.is_valid {
            self.load_error = "Invalid game installation.".into();
            return Err(self.load_error.clone());
        }

        let game_swz_path = Path::new(&installation.game_swz_path);
        let air_swf_path = Path::new(&installation.air_swf_path);

        let key = find_swz_key(air_swf_path, game_swz_path).unwrap_or(154281100u32);
        let files = read_swz_files(game_swz_path, key)?;

        // 1. Color Schemes
        if let Some(color_xml) = files.get("ColorSchemeTypes.xml") {
            self.color_schemes = parse_color_schemes(color_xml);
        }

        // 2. Legends
        if let Some(hero_xml) = files.get("HeroTypes.xml") {
            self.legends = parse_legends(hero_xml);
        }

        // 3. Costumes
        if let Some(costume_csv) = files.get("costumeTypes.csv") {
            self.costumes = parse_costumes(costume_csv);
        }

        // 4. Weapon Skins
        if let Some(weapon_csv) = files.get("weaponSkinTypes.csv") {
            self.weapon_skins = parse_weapon_skins(weapon_csv);
        }

        // Group weapon skins by base weapon
        self.weapon_skins_by_type.clear();
        for ws in &self.weapon_skins {
            if ws.base_weapon.trim().is_empty() {
                continue;
            }
            self.weapon_skins_by_type
                .entry(ws.base_weapon.to_lowercase())
                .or_default()
                .push(ws.clone());
        }

        // Associate Costumes with Legends
        let mut legend_map: HashMap<String, usize> = HashMap::new();
        for (i, legend) in self.legends.iter().enumerate() {
            legend_map.insert(legend.hero_name.to_lowercase(), i);
        }

        for costume in &self.costumes {
            if let Some(&idx) = legend_map.get(&costume.owner_hero.to_lowercase()) {
                let legend = &mut self.legends[idx];
                legend.costumes.push(costume.clone());
                if costume.is_default && legend.primary_swf.is_empty() {
                    legend.primary_swf = costume.swf_file.clone();
                }
            }
        }

        self.is_loaded = true;
        Ok(())
    }
}

pub fn parse_legends(xml: &str) -> Vec<LegendInfo> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut legends = Vec::new();
    let mut current_hero: Option<LegendInfo> = None;
    let mut current_tag = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag_name == "HeroType" {
                    let mut hero_name = String::new();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"HeroName" {
                            hero_name = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                    if !hero_name.is_empty()
                        && !hero_name.eq_ignore_ascii_case("Template")
                        && !hero_name.eq_ignore_ascii_case("DEFAULT_CHARACTER")
                    {
                        current_hero = Some(LegendInfo {
                            hero_id: 0,
                            hero_name: hero_name.clone(),
                            hero_key: hero_name.clone(),
                            display_name: hero_name,
                            bio: String::new(),
                            weapon1: String::new(),
                            weapon2: String::new(),
                            icon_name: String::new(),
                            pose_symbol: String::new(),
                            primary_swf: String::new(),
                            costumes: Vec::new(),
                        });
                    }
                } else if current_hero.is_some() {
                    current_tag = tag_name;
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut hero) = current_hero {
                    let text = e.unescape().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "HeroID" => hero.hero_id = text.parse().unwrap_or(0),
                        "BioName" => {
                            if !text.trim().is_empty() {
                                hero.bio = text.clone();
                                hero.display_name = text;
                            }
                        }
                        "HeroDisplayName" => {
                            if hero.bio.is_empty() && !text.trim().is_empty() && text != "---" {
                                hero.display_name = text;
                            }
                        }
                        "BaseWeapon1" | "Weapon1" if hero.weapon1.is_empty() => hero.weapon1 = text,
                        "BaseWeapon2" | "Weapon2" if hero.weapon2.is_empty() => hero.weapon2 = text,
                        "SpecialPower1" => {
                            if let Some((_, k)) = text.split_once("SmashNeutral") {
                                let key = k.trim();
                                if !key.is_empty() {
                                    hero.hero_key = key.to_string();
                                }
                            }
                        }
                        "IconName" => hero.icon_name = text,
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = String::from_utf8_lossy(name.as_ref());
                if tag_name == "HeroType" {
                    if let Some(mut hero) = current_hero.take() {
                        let clean_pose = hero
                            .display_name
                            .replace(' ', "")
                            .replace('ö', "o")
                            .replace('Ö', "O");
                        hero.pose_symbol = format!("a_Roster_Pose_{}", clean_pose);
                        legends.push(hero);
                    }
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    legends.sort_by(|a, b| {
        if a.hero_name.eq_ignore_ascii_case("Random") {
            std::cmp::Ordering::Greater
        } else if b.hero_name.eq_ignore_ascii_case("Random") {
            std::cmp::Ordering::Less
        } else {
            a.hero_id.cmp(&b.hero_id)
        }
    });

    legends
}

pub fn parse_costumes(csv: &str) -> Vec<CostumeInfo> {
    let mut lines = csv.lines();
    let _table_name = lines.next();
    let header_line = match lines.next() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let headers = parse_csv_line(header_line);
    let col_map: HashMap<String, usize> = headers
        .into_iter()
        .enumerate()
        .map(|(i, h)| (h.to_lowercase(), i))
        .collect();

    let mut costumes = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols = parse_csv_line(line);
        if cols.len() < 10 {
            continue;
        }

        let name = get_col(&cols, &col_map, "costumename");
        let owner = get_col(&cols, &col_map, "ownerhero");

        if name.is_empty()
            || name.eq_ignore_ascii_case("Template")
            || (name.eq_ignore_ascii_case("Default") && owner.eq_ignore_ascii_case("DEFAULT_CHARACTER"))
        {
            continue;
        }

        let costume_id = get_col(&cols, &col_map, "costumeid").parse().unwrap_or(0);
        let disp_key = get_col(&cols, &col_map, "displaynamekey");
        let def_raw = get_col(&cols, &col_map, "defaultunlocked");
        let default_unlocked = def_raw.eq_ignore_ascii_case("true") || def_raw == "1";
        let is_crossover = get_col(&cols, &col_map, "iscrossover").eq_ignore_ascii_case("true");
        let is_metadev = get_col(&cols, &col_map, "ismetadev").eq_ignore_ascii_case("true");

        let mut custom_arts = Vec::new();
        for col_name in &[
            "gfxtype.customart2",
            "gfxtype.customart",
            "gfxtype.customart3",
            "gfxtype.customart4",
            "customart2",
            "customart",
            "customart3",
            "customart4",
        ] {
            let val = get_col(&cols, &col_map, col_name);
            if !val.is_empty() && val != "--" && val.contains('/') && !custom_arts.contains(&val) {
                custom_arts.push(val);
            }
        }

        let art_to_use = custom_arts
            .iter()
            .find(|c| !c.contains("Gfx_Hands.swf") && !c.contains("/No"))
            .cloned()
            .unwrap_or_else(|| custom_arts.first().cloned().unwrap_or_default());

        let (mut swf, symbol) = if let Some((s, sym)) = art_to_use.split_once('/') {
            (s.to_string(), sym.to_string())
        } else if !owner.is_empty() && owner != "--" && owner != "DEFAULT_CHARACTER" {
            (format!("Gfx_{}.swf", owner), name.clone())
        } else {
            ("Gfx_Costumes.swf".to_string(), name.clone())
        };

        if !swf.is_empty() && !swf.to_lowercase().ends_with(".swf") {
            swf.push_str(".swf");
        }

        let friendly_name = format_display_name(&name, &disp_key);
        let icon = get_col(&cols, &col_map, "costumeicon");
        let mut icon_file = get_col(&cols, &col_map, "costumeiconfilename");
        if icon_file.is_empty() || icon_file == "--" || icon_file == "0" {
            icon_file = "UI_Icons.swf".to_string();
        } else if !icon_file.to_lowercase().ends_with(".swf") {
            icon_file.push_str(".swf");
        }

        let weapon_set = get_col(&cols, &col_map, "weaponset");
        let hands_art = custom_arts
            .iter()
            .find(|c| c.to_lowercase().starts_with("gfx_hands.swf/"))
            .cloned()
            .unwrap_or_default();
        let hands_suffix = hands_art.split_once('/').map(|(_, s)| s).unwrap_or("").to_string();

        let default_cape = get_col(&cols, &col_map, "defaultcape");
        let head_gfx = get_col(&cols, &col_map, "headgfxcustomart");
        let bone_override = get_col(&cols, &col_map, "boneoverride");
        let name_sound_event = get_col(&cols, &col_map, "namesoundevent");
        let on_selected_sound_event = get_col(&cols, &col_map, "onselectedsoundevent");
        let replacement_portrait = get_col(&cols, &col_map, "replacementportrait");
        let crossover_gem = get_col(&cols, &col_map, "crossovergem");

        let mut swap_custom_arts = Vec::new();
        for col_name in &[
            "swapcustomart",
            "swapcustomart2",
            "swapcustomart3",
            "swapcustomart4",
            "swapcustomart5",
        ] {
            let val = get_col(&cols, &col_map, col_name);
            if !val.is_empty() && val != "--" && val.contains('/') && !swap_custom_arts.contains(&val) {
                swap_custom_arts.push(val);
            }
        }

        let hands_lt_swap = get_col(&cols, &col_map, "handslt_swap");
        let hands_dk_swap = get_col(&cols, &col_map, "handsdk_swap");
        let hands_skin_lt_swap = get_col(&cols, &col_map, "handsskinlt_swap");
        let hands_skin_dk_swap = get_col(&cols, &col_map, "handsskindk_swap");

        let mut base_colors = HashMap::new();
        for (col_key, idx) in &col_map {
            if col_key.ends_with("_define") && *idx < cols.len() {
                let hex_str = &cols[*idx];
                if let Some(val) = parse_hex_color(hex_str) {
                    let clean_key = col_key.trim_end_matches("_define");
                    base_colors.insert(clean_key.to_lowercase(), val);
                }
            }
        }

        let is_default = name.eq_ignore_ascii_case(&owner)
            || (default_unlocked && (disp_key.is_empty() || name.eq_ignore_ascii_case("Default")))
            || (owner.eq_ignore_ascii_case("Random") && name.eq_ignore_ascii_case("Random"));

        costumes.push(CostumeInfo {
            costume_id,
            costume_name: name,
            owner_hero: owner,
            display_name: friendly_name,
            is_default,
            swf_file: swf,
            symbol_suffix: symbol,
            weapon_set,
            icon_name: if icon != "--" { icon } else { String::new() },
            icon_file_name: icon_file,
            is_crossover,
            is_metadev,
            hands_art,
            hands_suffix,
            hands_lt_swap,
            hands_dk_swap,
            hands_skin_lt_swap,
            hands_skin_dk_swap,
            custom_art_layers: custom_arts,
            swap_custom_arts,
            default_cape,
            head_gfx,
            bone_override,
            name_sound_event,
            on_selected_sound_event,
            replacement_portrait,
            crossover_gem,
            base_colors,
        });
    }

    costumes
}

pub fn parse_weapon_skins(csv: &str) -> Vec<WeaponSkinInfo> {
    let mut lines = csv.lines();
    let _table_name = lines.next();
    let header_line = match lines.next() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let headers = parse_csv_line(header_line);
    let col_map: HashMap<String, usize> = headers
        .into_iter()
        .enumerate()
        .map(|(i, h)| (h.to_lowercase(), i))
        .collect();

    let mut weapons = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols = parse_csv_line(line);
        if cols.len() < 8 {
            continue;
        }

        let name = get_col(&cols, &col_map, "weaponskinname");
        let base_weapon = get_col(&cols, &col_map, "baseweapon");
        let mut owner = get_col(&cols, &col_map, "ownerhero");

        if name.is_empty()
            || name.eq_ignore_ascii_case("Template")
            || base_weapon.is_empty()
            || base_weapon == "--"
        {
            continue;
        }

        let weapon_id = get_col(&cols, &col_map, "weaponskinid").parse().unwrap_or(0);
        let disp_key = get_col(&cols, &col_map, "displaynamekey");
        let friendly_name = format_display_name(&name, &disp_key);

        let custom_art = get_col(&cols, &col_map, "customart");
        let (mut swf, mut symbol) = if let Some((s, sym)) = custom_art.split_once('/') {
            (s.to_string(), sym.to_string())
        } else if !owner.is_empty() && owner != "--" && owner != "DEFAULT_CHARACTER" {
            (format!("Gfx_{}.swf", owner), name.clone())
        } else {
            ("Gfx_Weapons.swf".to_string(), name.clone())
        };

        if !swf.is_empty() && !swf.to_lowercase().ends_with(".swf") {
            swf.push_str(".swf");
        }

        // Authentic owner hero inference from SWF if owner is unset
        if (owner.is_empty() || owner == "--" || owner == "DEFAULT_CHARACTER")
            && swf.starts_with("Gfx_")
            && !swf.starts_with("Gfx_Weapons")
        {
            let inferred = swf.trim_start_matches("Gfx_").trim_end_matches(".swf");
            if !inferred.is_empty() {
                owner = inferred.to_string();
            }
        }

        // Map HammerStub (Skin ID 710, Detective costume) to authentic Detective Hammer asset in Gfx_Super.swf
        if base_weapon.eq_ignore_ascii_case("Hammer")
            && (symbol.eq_ignore_ascii_case("Stub") || name.eq_ignore_ascii_case("HammerStub") || weapon_id == 710)
        {
            swf = "Gfx_Super.swf".to_string();
            symbol = "Detective".to_string();
        }

        let icon = get_col(&cols, &col_map, "weaponicon");
        let sound_bank = get_col(&cols, &col_map, "soundbank");
        let is_default = get_col(&cols, &col_map, "defaultunlocked").eq_ignore_ascii_case("true");

        let mut base_colors = HashMap::new();
        for (col_key, idx) in &col_map {
            if col_key.ends_with("_define") && *idx < cols.len() {
                let hex_str = &cols[*idx];
                if let Some(val) = parse_hex_color(hex_str) {
                    let clean_key = col_key.trim_end_matches("_define");
                    base_colors.insert(clean_key.to_lowercase(), val);
                }
            }
        }

        weapons.push(WeaponSkinInfo {
            weapon_skin_id: weapon_id,
            weapon_skin_name: name,
            base_weapon,
            owner_hero: owner,
            display_name: friendly_name,
            swf_file: swf,
            symbol_suffix: symbol,
            weapon_icon: icon,
            sound_bank,
            is_default,
            base_colors,
        });
    }

    weapons
}

pub fn parse_color_schemes(xml: &str) -> Vec<ColorSchemeInfo> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut schemes = Vec::new();
    let mut current_scheme: Option<ColorSchemeInfo> = None;
    let mut current_tag = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag_name == "ColorSchemeType" {
                    let mut name = String::new();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"ColorSchemeName" {
                            name = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                    if !name.is_empty() && !name.eq_ignore_ascii_case("Template") {
                        current_scheme = Some(ColorSchemeInfo {
                            scheme_id: 0,
                            display_name: format_display_name(&name, ""),
                            scheme_name: name,
                            rarity: "Common".into(),
                            indicator_color: 0x3b82f6,
                            swap_colors: HashMap::new(),
                        });
                    }
                } else if current_scheme.is_some() {
                    current_tag = tag_name;
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut scheme) = current_scheme {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if current_tag == "ColorSchemeID" {
                        scheme.scheme_id = text.parse().unwrap_or(0);
                    } else if current_tag == "DisplayNameKey" {
                        scheme.display_name = format_display_name(&scheme.scheme_name, &text);
                    } else if current_tag == "Rarity" {
                        scheme.rarity = text;
                    } else if current_tag == "IndicatorColor" {
                        if let Some(val) = parse_hex_color(&text) {
                            scheme.indicator_color = val;
                        }
                    } else if current_tag.to_lowercase().ends_with("_swap") {
                        let clean_key = current_tag.trim_end_matches("_Swap").trim_end_matches("_swap");
                        if let Some(val) = parse_hex_color(&text) {
                            scheme.swap_colors.insert(clean_key.to_lowercase(), val);
                            scheme.swap_colors.insert(clean_key.to_string(), val);
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag_name = String::from_utf8_lossy(name.as_ref());
                if tag_name == "ColorSchemeType" {
                    if let Some(scheme) = current_scheme.take() {
                        schemes.push(scheme);
                    }
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    schemes.sort_by_key(|s| s.scheme_id);
    schemes
}

pub fn parse_csv_line(line: &str) -> Vec<String> {
    let mut cols = Vec::new();
    let mut in_quotes = false;
    let mut current = String::new();

    for c in line.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if c == ',' && !in_quotes {
            cols.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    cols.push(current.trim().to_string());
    cols
}

fn get_col(cols: &[String], map: &HashMap<String, usize>, name: &str) -> String {
    if let Some(&idx) = map.get(name) {
        if idx < cols.len() {
            return cols[idx].trim_matches('"').trim().to_string();
        }
    }
    String::new()
}

fn parse_hex_color(s: &str) -> Option<u32> {
    let clean = s.trim().trim_start_matches("0x").trim_start_matches('#');
    let val = u32::from_str_radix(clean, 16).ok()?;
    Some(val & 0xFFFFFF)
}

fn format_display_name(name: &str, key: &str) -> String {
    let raw = if !key.is_empty() && key != "--" {
        key
    } else {
        name
    };

    let clean = raw
        .trim_start_matches("UI_DisplayName_")
        .trim_start_matches("UI_CostumeName_")
        .trim_start_matches("UI_ColorScheme_")
        .replace("CostumeType_", "")
        .replace("CostumeType", "")
        .replace("WeaponSkinType_", "")
        .replace("WeaponSkinType", "")
        .replace("ColorSchemeType_", "")
        .replace("ColorSchemeType", "")
        .replace("_DisplayName", "")
        .replace("DisplayName", "")
        .replace('_', " ");

    let trimmed = clean.trim();
    if trimmed.is_empty() {
        return name.to_string();
    }

    // Insert spaces before capital letters: "Beardvar" -> "Beardvar", "DemonSlayer" -> "Demon Slayer"
    let mut out = String::new();
    let mut prev_is_lower = false;
    for c in trimmed.chars() {
        if prev_is_lower && c.is_uppercase() {
            out.push(' ');
        }
        prev_is_lower = c.is_lowercase();
        out.push(c);
    }
    out.trim().to_string()
}
