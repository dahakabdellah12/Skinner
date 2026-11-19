use crate::models::{ColorSchemeInfo, CostumeInfo, GameInstallation, KoEffectInfo, LegendInfo, WeaponSkinInfo};
use crate::swz::{find_swz_key, read_swz_files};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Explicitly excluded skin records from Skinner catalog based on character ownership and internal Costume ID.
pub const EXCLUDED_COSTUME_IDS: &[(&str, i32)] = &[
    // Cassidy
    ("cowgirl", 358), // Globox
    ("cowgirl", 770), // EpicCowgirl
    ("cowgirl", 989), // Sandy
    // Orion
    ("valkyrie", 354), // BlackKnight
    ("valkyrie", 557), // EpicOrion
    // Lord Vraxx
    ("alien", 401), // Bubblegum
    ("alien", 707), // DestinyWarlock (Destiny Vraxx)
    ("alien", 309), // AlienWarlord
    // Nash (Gnash)
    ("caveman", 355), // Blue (Shovel Knight)
    ("caveman", 409), // Becky (Becky Lynch)
    ("caveman", 482), // ZombieWalker1
    ("caveman", 483), // ZombieWalker2
    ("caveman", 484), // ZombieWalker3
    // Queen Nai
    ("witch", 420), // Asuka
    ("witch", 734), // Mikasa
    ("witch", 701), // EpicWitch
    // Lucien
    ("highwayman", 636), // Nina (Nina Williams)
    ("highwayman", 709), // DestinyHunter
    // Hattori
    ("ninja", 408), // Cena (John Cena)
    ("ninja", 602), // Zuko
    ("ninja", 998), // DarthMaul
    // Sir Roland
    ("knight", 351), // KingKnight
    // Scarlet
    ("steampunk", 665), // TinyTina
    // Thatch
    ("thatch", 140), // Shark
    // Ada
    ("cyber", 680), // Mando
    ("cyber", 454), // Heatblast
    // Sentinel
    ("super", 411), // TheRock
    // Teros
    ("minotaur", 987), // Patrick (Patrick Star)
    ("minotaur", 814), // MythicMinotaur (Ascended Teros)
    ("minotaur", 421), // RomanReigns
    ("minotaur", 396), // Gruagach (Goroagash)
    // Red Raptor
    ("sentai", 678), // EpicRaptor
    ("sentai", 735), // Eren
    // Ember
    ("elf", 479), // Daryl
    ("elf", 583), // EpicEmber
    // Brynn (Barbara, EpicBrynn, and Wolf-Kissed Eivor crossover variants)
    ("actualvalk", 359), // Barbara
    ("actualvalk", 508), // EpicBrynn
    ("actualvalk", 589), // Eivor (Wolf Casey - Wolf-Kissed Eivor)
    ("actualvalk", 590), // EivorMale (Wolf Casey - Wolf-Kissed Eivor Male)
    // Asuri
    ("cat", 642), // Ahsoka
    ("cat", 736), // Levi
    ("cat", 511), // Tigress (Tiger)
    ("cat", 793), // Puss (Puss in Boots)
    // Barraza
    ("apoc", 480), // Rick
    ("apoc", 663), // Krieg
    // Azoth
    ("skeleton", 533), // BP4Azoth (Battle Pass 4 Azoth)
    // Koji
    ("samurai", 562), // StormShadow
    ("samurai", 478), // Michonne
    ("samurai", 376), // CyberSam (Sei Jo-Kai Koji / Shen Sekai)
    // Diana (both Lara Croft records + Epic Diana)
    ("marksman", 440), // Lara (Lara Croft)
    ("marksman", 441), // Croft (Lara Croft)
    ("marksman", 526), // EpicDiana
    // Jhala
    ("barbarian", 399), // Finn
    ("barbarian", 537), // Maggie (Magi)
    ("barbarian", 598), // Simon
    // Kor
    ("golem", 400), // Jake
    ("golem", 603), // Toph
    ("golem", 456), // FourArms
    // Wu Shang
    ("monk", 723), // MythicWuShang (Ascended Wu Shang)
    ("monk", 509), // Po
    ("monk", 601), // Aang
    ("monk", 547), // SFChunLi (Street Fighter Chun-Li)
    // Val
    ("technoninja", 997), // ObiWan
    ("technoninja", 429), // Stevonnie (Stephanie)
    ("technoninja", 522), // Mikey (Michelangelo)
    ("technoninja", 548), // SFAkuma (Akuma)
    // Ragnir
    ("dragon", 523), // Raph (Raphael)
    ("dragon", 771), // EpicDragon
    // Cross
    ("mobster", 681), // HanSolo
    ("mobster", 575), // SFLuke (Luke)
    ("mobster", 393), // Hellboy
    // Mirage
    ("egyptian", 525), // Donnie (Donatello)
    // Nix
    ("reaper", 472), // EpicNix
    ("reaper", 722), // MythicNix (Ascended Nix)
    ("reaper", 352), // SpecterKnight
    // Mordex
    ("werewolf", 627), // EpicMordex
    ("werewolf", 510), // TaiLung
    ("werewolf", 395), // Daimio
    // Yumiko
    ("ninetails", 626), // EpicYumiko
    // Artemis
    ("spacehunter", 659), // EpicArtemis
    // Caspian
    ("thief", 455), // Diamondhead
    ("thief", 353), // PlagueKnight
    // Sidra
    ("corsair", 977), // Arbiter
    // Xull
    ("brute", 791), // Shrek
    ("brute", 428), // Amethyst
    // Isaiah
    ("soldier", 682), // KayVess (Key Vess)
    ("soldier", 976), // MasterChief
    // Kaya
    ("inuit", 427), // Pearl
    // Jiro
    ("shinobi", 524), // Leo (Leonardo)
    ("shinobi", 387), // Dullahan
    ("shinobi", 637), // Yoshimitsu
    // Lin Fei
    ("wuxia", 571), // SFSakura (Sakura)
    // Zariel
    ("celestial", 638), // DevilJin
    ("celestial", 801), // EpicCelestial
    // Rayman
    ("rayman", 986), // Spongebob (SpongeBob)
    ("rayman", 732), // EpicRayman
    ("rayman", 573), // SFDhalsim (Dhalsim)
    // Dusk
    ("elfwar", 394), // Nimue
    // Fait
    ("spellwitch", 386), // Enchantress
    // Thor
    ("thor", 572), // SFBison (M. Bison)
    ("thor", 423), // Undertaker
    // Petra
    ("ragefighter", 430), // Garnet
    ("ragefighter", 549), // SFRyu (Ryu)
    ("ragefighter", 574), // SFKen (Ken)
    // Volkov
    ("vamplord", 617), // EpicWarlock (Epic Volkov)
    // Jaeyun
    // Mako
    ("actualshark", 718), // Grievous (General Grievous)
    // Reno
    ("bountyhunter", 664), // Lilith
    // Munin
    ("birdbard", 545), // HuginDuck (Huginn Duck)
    // Arcadia
    ("fairyqueen", 758), // Luke (Luke Skywalker)
    // Ezio
    ("ezio", 999), // DarthVader (Darth Vader)
    ("ezio", 599), // Alucard
    ("ezio", 641), // Anakin (Anakin Skywalker)
    // Tezca
    ("luchador", 792), // Fiona
    // Vivi
    ("assassin", 666), // Megaman (Mega Man)
];

/// Redundant internal duplicate records to be pruned from the Skinner catalog.
/// Keeps the authentic canonical record (e.g. CyberVirus 719, Roboengineer 639).
pub const DUPLICATE_COSTUME_IDS: &[(&str, i32)] = &[
    ("cybervirus", 720), // CyberVirusStance2
    ("roboengineer", 640), // RoboengineerStance2
    ("bladedancer", 964), // BladeDancerStance2 (Priya)
    ("actualgladiator", 785), // ActualGladiatorStance2 (Aurus)
    ("cleric", 738), // ClericStance2 (Lady Vera)
];

/// Returns true if a costume is excluded from the Skinner catalog.
pub fn is_excluded_costume(owner_hero: &str, costume_id: i32) -> bool {
    let o = owner_hero.trim().to_lowercase();
    EXCLUDED_COSTUME_IDS.iter().any(|(hero, id)| *id == costume_id && *hero == o)
        || DUPLICATE_COSTUME_IDS.iter().any(|(hero, id)| *id == costume_id && *hero == o)
}

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
    pub ko_effects: Vec<KoEffectInfo>,
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

    /// Prunes excluded costumes and cleans duplicate records across the catalog,
    /// ensuring authentic canonical representation while strictly preserving
    /// all distinct visual skins even if display names are identical.
    pub fn clean_and_deduplicate_costumes(&mut self) {
        self.costumes.retain(|c| !is_excluded_costume(&c.owner_hero, c.costume_id));

        let mut seen_ids = HashSet::new();
        self.costumes.retain(|c| seen_ids.insert((c.owner_hero.to_lowercase(), c.costume_id)));
    }

    pub fn load_from_installation(&mut self, installation: &GameInstallation) -> Result<(), String> {
        if !installation.is_valid {
            self.load_error = "Invalid game installation.".into();
            return Err(self.load_error.clone());
        }

        let game_dir = Path::new(&installation.game_path);
        let game_swz_path = Path::new(&installation.game_swz_path);
        let air_swf_path = Path::new(&installation.air_swf_path);

        let key = find_swz_key(air_swf_path, game_swz_path).unwrap_or(154281100u32);
        let files = read_swz_files(game_swz_path, key)?;

        // Authentic language localization table
        let string_table = load_language_strings(game_dir);

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
            self.costumes = parse_costumes(costume_csv, &string_table);
            self.clean_and_deduplicate_costumes();
        }

        // 4. Weapon Skins
        if let Some(weapon_csv) = files.get("weaponSkinTypes.csv") {
            self.weapon_skins = parse_weapon_skins(weapon_csv, &string_table);
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

        // Associate Costumes with Legends and format default costume display names
        let mut legend_map: HashMap<String, usize> = HashMap::new();
        for (i, legend) in self.legends.iter().enumerate() {
            legend_map.insert(legend.hero_name.to_lowercase(), i);
        }

        for costume in &mut self.costumes {
            if let Some(&idx) = legend_map.get(&costume.owner_hero.to_lowercase()) {
                let legend = &mut self.legends[idx];
                if costume.is_default {
                    costume.display_name = format!("Default {}", legend.display_name);
                }
                legend.costumes.push(costume.clone());
                if costume.is_default && legend.primary_swf.is_empty() {
                    legend.primary_swf = costume.swf_file.clone();
                }
            }
        }

        // 5. KO Effects (TrailEffectTypes.xml)
        if let Some(trail_xml) = files.get("TrailEffectTypes.xml") {
            self.ko_effects = parse_ko_effects(trail_xml, &string_table, Some(game_dir));
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

pub fn parse_costumes(csv: &str, string_table: &HashMap<String, String>) -> Vec<CostumeInfo> {
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

    let idx_costumename = col_map.get("costumename").copied();
    let idx_ownerhero = col_map.get("ownerhero").copied();
    let idx_costumeid = col_map.get("costumeid").copied();
    let idx_displaynamekey = col_map.get("displaynamekey").copied();
    let idx_defaultunlocked = col_map.get("defaultunlocked").copied();
    let idx_iscrossover = col_map.get("iscrossover").copied();
    let idx_ismetadev = col_map.get("ismetadev").copied();
    let idx_costumeicon = col_map.get("costumeicon").copied();
    let idx_costumeiconfilename = col_map.get("costumeiconfilename").copied();
    let idx_weaponset = col_map.get("weaponset").copied();
    let idx_defaultcape = col_map.get("defaultcape").copied();
    let idx_headgfxcustomart = col_map.get("headgfxcustomart").copied();
    let idx_boneoverride = col_map.get("boneoverride").copied();
    let idx_namesoundevent = col_map.get("namesoundevent").copied();
    let idx_onselectedsoundevent = col_map.get("onselectedsoundevent").copied();
    let idx_replacementportrait = col_map.get("replacementportrait").copied();
    let idx_crossovergem = col_map.get("crossovergem").copied();
    let idx_handslt_swap = col_map.get("handslt_swap").copied();
    let idx_handsdk_swap = col_map.get("handsdk_swap").copied();
    let idx_handsskinlt_swap = col_map.get("handsskinlt_swap").copied();
    let idx_handsskindk_swap = col_map.get("handsskindk_swap").copied();

    let art_col_indices: Vec<usize> = [
        "gfxtype.customart2",
        "gfxtype.customart",
        "gfxtype.customart3",
        "gfxtype.customart4",
        "customart2",
        "customart",
        "customart3",
        "customart4",
    ]
    .iter()
    .filter_map(|name| col_map.get(*name).copied())
    .collect();

    let swap_col_indices: Vec<usize> = [
        "swapcustomart",
        "swapcustomart2",
        "swapcustomart3",
        "swapcustomart4",
        "swapcustomart5",
    ]
    .iter()
    .filter_map(|name| col_map.get(*name).copied())
    .collect();

    let mut costumes = Vec::with_capacity(1024);

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols = parse_csv_line_borrowed(line);
        if cols.len() < 10 {
            continue;
        }

        let name = get_col_str(&cols, idx_costumename);
        let owner = get_col_str(&cols, idx_ownerhero);

        if name.is_empty()
            || name.eq_ignore_ascii_case("Template")
            || (name.eq_ignore_ascii_case("Default") && owner.eq_ignore_ascii_case("DEFAULT_CHARACTER"))
        {
            continue;
        }

        let costume_id = get_col_str(&cols, idx_costumeid).parse().unwrap_or(0);
        if is_excluded_costume(owner, costume_id) {
            continue;
        }
        let disp_key = get_col_str(&cols, idx_displaynamekey);
        let def_raw = get_col_str(&cols, idx_defaultunlocked);
        let default_unlocked = def_raw.eq_ignore_ascii_case("true") || def_raw == "1";
        let is_crossover = get_col_str(&cols, idx_iscrossover).eq_ignore_ascii_case("true");
        let is_metadev = get_col_str(&cols, idx_ismetadev).eq_ignore_ascii_case("true");

        let mut custom_arts = Vec::with_capacity(4);
        for &idx in &art_col_indices {
            if idx < cols.len() {
                let val = cols[idx];
                if !val.is_empty() && val != "--" && val.contains('/') && !custom_arts.iter().any(|s: &String| s == val) {
                    custom_arts.push(val.to_string());
                }
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
            (format!("Gfx_{}.swf", owner), name.to_string())
        } else {
            ("Gfx_Costumes.swf".to_string(), name.to_string())
        };

        if !swf.is_empty() && !swf.to_lowercase().ends_with(".swf") {
            swf.push_str(".swf");
        }

        let friendly_name = if let Some(v) = string_table.get(disp_key) {
            v.clone()
        } else if let Some(v) = string_table.get(&format!("CostumeType_{}_DisplayName", name)) {
            v.clone()
        } else if let Some(v) = string_table.get(&format!("UI_CostumeName_{}", name)) {
            v.clone()
        } else {
            format_display_name(name, disp_key)
        };

        let icon = get_col_str(&cols, idx_costumeicon).to_string();
        let mut icon_file = get_col_str(&cols, idx_costumeiconfilename).to_string();
        if icon_file.is_empty() || icon_file == "--" || icon_file == "0" {
            icon_file = "UI_Icons.swf".to_string();
        } else if !icon_file.to_lowercase().ends_with(".swf") {
            icon_file.push_str(".swf");
        }

        let weapon_set = get_col_str(&cols, idx_weaponset).to_string();
        let hands_art = custom_arts
            .iter()
            .find(|c| c.to_lowercase().starts_with("gfx_hands.swf/"))
            .cloned()
            .unwrap_or_default();
        let hands_suffix = hands_art.split_once('/').map(|(_, s)| s).unwrap_or("").to_string();

        let default_cape = get_col_str(&cols, idx_defaultcape).to_string();
        let head_gfx = get_col_str(&cols, idx_headgfxcustomart).to_string();
        let bone_override = get_col_str(&cols, idx_boneoverride).to_string();
        let name_sound_event = get_col_str(&cols, idx_namesoundevent).to_string();
        let on_selected_sound_event = get_col_str(&cols, idx_onselectedsoundevent).to_string();
        let replacement_portrait = get_col_str(&cols, idx_replacementportrait).to_string();
        let crossover_gem = get_col_str(&cols, idx_crossovergem).to_string();

        let mut swap_custom_arts = Vec::with_capacity(4);
        for &idx in &swap_col_indices {
            if idx < cols.len() {
                let val = cols[idx];
                if !val.is_empty() && val != "--" && val.contains('/') && !swap_custom_arts.iter().any(|s: &String| s == val) {
                    swap_custom_arts.push(val.to_string());
                }
            }
        }

        let hands_lt_swap = get_col_str(&cols, idx_handslt_swap).to_string();
        let hands_dk_swap = get_col_str(&cols, idx_handsdk_swap).to_string();
        let hands_skin_lt_swap = get_col_str(&cols, idx_handsskinlt_swap).to_string();
        let hands_skin_dk_swap = get_col_str(&cols, idx_handsskindk_swap).to_string();

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
            costume_name: name.to_string(),
            owner_hero: owner.to_string(),
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

pub fn parse_weapon_skins(csv: &str, string_table: &HashMap<String, String>) -> Vec<WeaponSkinInfo> {
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

    let idx_weaponskinname = col_map.get("weaponskinname").copied();
    let idx_baseweapon = col_map.get("baseweapon").copied();
    let idx_ownerhero = col_map.get("ownerhero").copied();
    let idx_weaponskinid = col_map.get("weaponskinid").copied();
    let idx_displaynamekey = col_map.get("displaynamekey").copied();
    let idx_customart = col_map.get("customart").copied();
    let idx_weaponicon = col_map.get("weaponicon").copied();
    let idx_soundbank = col_map.get("soundbank").copied();
    let idx_defaultunlocked = col_map.get("defaultunlocked").copied();

    let define_cols: Vec<(String, usize)> = col_map
        .iter()
        .filter(|(k, _)| k.ends_with("_define"))
        .map(|(k, &idx)| (k.trim_end_matches("_define").to_lowercase(), idx))
        .collect();

    let mut weapons = Vec::with_capacity(2048);

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols = parse_csv_line_borrowed(line);
        if cols.len() < 8 {
            continue;
        }

        let name = get_col_str(&cols, idx_weaponskinname);
        let base_weapon = get_col_str(&cols, idx_baseweapon);
        let mut owner = get_col_str(&cols, idx_ownerhero).to_string();

        if name.is_empty()
            || name.eq_ignore_ascii_case("Template")
            || base_weapon.is_empty()
            || base_weapon == "--"
        {
            continue;
        }

        let weapon_id = get_col_str(&cols, idx_weaponskinid).parse().unwrap_or(0);
        let disp_key = get_col_str(&cols, idx_displaynamekey);
        let friendly_name = if let Some(v) = string_table.get(disp_key) {
            v.clone()
        } else if let Some(v) = string_table.get(&format!("WeaponSkinType_{}_DisplayName", name)) {
            v.clone()
        } else if let Some(v) = string_table.get(&format!("UI_WeaponSkinName_{}", name)) {
            v.clone()
        } else {
            format_display_name(name, disp_key)
        };


        let custom_art = get_col_str(&cols, idx_customart);
        let (mut swf, mut symbol) = if let Some((s, sym)) = custom_art.split_once('/') {
            (s.to_string(), sym.to_string())
        } else if !owner.is_empty() && owner != "--" && owner != "DEFAULT_CHARACTER" {
            (format!("Gfx_{}.swf", owner), name.to_string())
        } else {
            ("Gfx_Weapons.swf".to_string(), name.to_string())
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

        let icon = get_col_str(&cols, idx_weaponicon).to_string();
        let sound_bank = get_col_str(&cols, idx_soundbank).to_string();
        let is_default = get_col_str(&cols, idx_defaultunlocked).eq_ignore_ascii_case("true");

        let mut base_colors = HashMap::new();
        for (clean_key, idx) in &define_cols {
            if *idx < cols.len() {
                let hex_str = cols[*idx];
                if let Some(val) = parse_hex_color(hex_str) {
                    base_colors.insert(clean_key.clone(), val);
                }
            }
        }

        weapons.push(WeaponSkinInfo {
            weapon_skin_id: weapon_id,
            weapon_skin_name: name.to_string(),
            base_weapon: base_weapon.to_string(),
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

    schemes.retain(|s| {
        let name = s.scheme_name.to_lowercase();
        let disp = s.display_name.to_lowercase();
        let is_team = (name.contains("team") || disp.contains("team"))
            && !name.contains("hometeam")
            && !disp.contains("hometeam")
            && !disp.contains("home team");
        !is_team
            && !name.eq_ignore_ascii_case("no_color_scheme")
            && s.scheme_id < 128
    });

    schemes.sort_by_key(|s| s.scheme_id);
    schemes
}

pub fn parse_ko_effects(
    xml: &str,
    string_table: &HashMap<String, String>,
    game_dir: Option<&Path>,
) -> Vec<KoEffectInfo> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut effects = Vec::new();
    let mut current_name = String::new();
    let mut current_id = 0;
    let mut current_disp = String::new();
    let mut current_anim_file = String::new();
    let mut current_anim_class = String::new();
    let mut current_anim_scale = 1.0f32;
    let mut current_sound = String::new();
    let mut tag_stack: Vec<String> = Vec::new();

    // Known symbols in SFX_KO.swf
    let sfx_ko_1_symbols: HashSet<&str> = [
        "a_smoketrailendblue",
        "a_smoketrailendcp",
        "a_smoketrailend_party",
        "a_smoketrailend_arcade",
        "a_smoketrailend_trexbighead",
        "a_smoketrailend_embers",
        "a_smoketrailend_pumpkin",
        "a_smoketrailend_snow",
        "a_smoketrailend_runicthunder",
        "a_smoketrailend_hotlava",
        "a_smoketrailend_dragonflame",
        "a_smoketrailend_spday",
        "a_sfxmedievildragon",
        "a_smoketrailend_demonhand",
        "a_smoketrailend_bhfest2026",
        "a_smoketrailend_gingerbreadman1",
        "a_smoketrailend_bp14",
        "a_smoketrailend_mythicminotaur",
        "a_smoketrailend_mythicwu",
    ].into_iter().collect();

    // Dynamically query symbol classes in installed SFX_KO.swf and SFX_KO2.swf
    let mut sfx_ko_symbols = HashSet::new();
    let mut sfx_ko2_symbols = HashSet::new();

    if let Some(gd) = game_dir {
        let p1 = gd.join("SFX_KO.swf");
        if let Ok(swf) = crate::swf_engine::SwfPackage::read_from_file(&p1) {
            if let Ok(syms) = swf.get_symbol_classes() {
                for (_, name) in syms {
                    sfx_ko_symbols.insert(name.to_lowercase());
                }
            }
        }
        let p2 = gd.join("SFX_KO2.swf");
        if let Ok(swf) = crate::swf_engine::SwfPackage::read_from_file(&p2) {
            if let Ok(syms) = swf.get_symbol_classes() {
                for (_, name) in syms {
                    sfx_ko2_symbols.insert(name.to_lowercase());
                }
            }
        }
    }

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "TrailEffectType" {
                    current_name.clear();
                    current_id = 0;
                    current_disp.clear();
                    current_anim_file.clear();
                    current_anim_class.clear();
                    current_anim_scale = 1.0;
                    current_sound.clear();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"TrailEffectName" {
                            current_name = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
                tag_stack.push(tag);
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if let Some(parent) = tag_stack.last() {
                    match parent.as_str() {
                        "TrailEffectID" => current_id = text.parse().unwrap_or(0),
                        "DisplayNameKey" => current_disp = text,
                        "KOSoundEvent" => current_sound = text,
                        "AnimFile" => {
                            if tag_stack.iter().any(|t| t == "KOGfx") {
                                current_anim_file = text;
                            }
                        }
                        "AnimClass" => {
                            if tag_stack.iter().any(|t| t == "KOGfx") {
                                current_anim_class = text;
                            }
                        }
                        "AnimScale" => {
                            if tag_stack.iter().any(|t| t == "KOGfx") {
                                current_anim_scale = text.parse().unwrap_or(1.0);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "TrailEffectType" {
                    if !current_name.is_empty() && current_name != "Template" && current_id > 0 && !current_anim_class.is_empty() {
                        let is_default = current_id == 1 || current_name.eq_ignore_ascii_case("SmokeTrailEffect");
                        let class_lower = current_anim_class.to_lowercase();
                        
                        // Determine authentic SWF location, or skip if the visual symbol does not exist on disk
                        let real_swf = if sfx_ko_symbols.contains(&class_lower) || sfx_ko_1_symbols.contains(class_lower.as_str()) || current_anim_file.eq_ignore_ascii_case("SFX_KO.swf") {
                            "SFX_KO.swf".to_string()
                        } else if sfx_ko2_symbols.contains(&class_lower) || current_anim_file.eq_ignore_ascii_case("SFX_KO2.swf") {
                            "SFX_KO2.swf".to_string()
                        } else if let Some(gd) = game_dir {
                            if !current_anim_file.is_empty() && gd.join(&current_anim_file).is_file() {
                                current_anim_file.clone()
                            } else {
                                // Missing SWF and symbol not in SFX_KO/SFX_KO2: skip effect to prevent black boxes
                                tag_stack.pop();
                                continue;
                            }
                        } else {
                            "SFX_KO2.swf".to_string()
                        };

                        // Determine friendly Category
                        let name_lower = current_name.to_lowercase();
                        let category = if is_default {
                            "Default".to_string()
                        } else if name_lower.contains("streetfighter")
                            || name_lower.contains("megaman")
                            || name_lower.contains("tmnt")
                            || name_lower.contains("adventuretime")
                            || name_lower.contains("malachite")
                            || name_lower.contains("vilgax")
                            || name_lower.contains("walkerhands")
                            || name_lower.contains("kfp")
                            || name_lower.contains("atla")
                            || name_lower.contains("animus")
                            || name_lower.contains("gruntexplosion")
                            || name_lower.contains("tkpanda")
                            || name_lower.contains("kingjelly")
                            || name_lower.contains("gingerbreadman")
                            || name_lower.contains("titanchomp")
                            || name_lower.contains("shrek")
                        {
                            "Crossover".to_string()
                        } else if name_lower.contains("bp") || name_lower.contains("nidhogg") || name_lower.contains("blackhole") {
                            "Battle Pass".to_string()
                        } else if name_lower.contains("pumpkin")
                            || name_lower.contains("snow")
                            || name_lower.contains("cherub")
                            || name_lower.contains("goldko")
                            || name_lower.contains("heatwave")
                            || name_lower.contains("bhfest")
                        {
                            "Seasonal".to_string()
                        } else {
                            "Store".to_string()
                        };

                        let friendly_disp = if is_default {
                            "Default Smoke Trail".to_string()
                        } else if let Some(v) = string_table.get(&current_disp) {
                            v.clone()
                        } else if let Some(v) = string_table.get(&format!("TrailEffectType_{}_DisplayName", current_name)) {
                            v.clone()
                        } else if let Some(v) = string_table.get(&format!("UI_TrailEffectName_{}", current_name)) {
                            v.clone()
                        } else if current_name == "TitanChompKO" {
                            "Titan Chomp (Attack on Titan)".to_string()
                        } else if current_name == "GingerBreadMan1" {
                            "Gingerbread Man (Shrek)".to_string()
                        } else if current_name == "ATLA" {
                            "My Cabbages! (Avatar)".to_string()
                        } else if current_name == "Animus" {
                            "Animus Desync (Assassin's Creed)".to_string()
                        } else if current_name == "StreetFighter" {
                            "Street Fighter K.O.".to_string()
                        } else if current_name == "TMNT" {
                            "TMNT Pizza Power".to_string()
                        } else if current_name == "AdventureTime" {
                            "Lady Rainicorn (Adventure Time)".to_string()
                        } else if current_name == "Malachite" {
                            "Malachite (Steven Universe)".to_string()
                        } else if current_name == "Vilgax" {
                            "Vilgax (Ben 10)".to_string()
                        } else if current_name == "WalkerHands" {
                            "Walker Hands (The Walking Dead)".to_string()
                        } else if current_name == "KFP" {
                            "Po & Shifu (Kung Fu Panda)".to_string()
                        } else if current_name == "KingJellyfish" {
                            "King Jellyfish (SpongeBob)".to_string()
                        } else if current_name == "MegaMan" {
                            "Mega Man Defeat".to_string()
                        } else if current_name == "TKPanda" {
                            "Panda (Tekken)".to_string()
                        } else if current_name == "GruntExplosion" {
                            "Grunt Birthday Party (Halo)".to_string()
                        } else {
                            format_display_name(&current_name, &current_disp)
                        };

                        effects.push(KoEffectInfo {
                            ko_effect_id: current_id,
                            ko_effect_name: current_name.clone(),
                            display_name: friendly_disp,
                            swf_file: real_swf,
                            anim_class: current_anim_class.clone(),
                            anim_scale: current_anim_scale,
                            sound_event: current_sound.clone(),
                            is_default,
                            category,
                        });
                    }
                }
                tag_stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }


    // Deduplicate by anim_class keeping authentic records
    let mut seen = HashSet::new();
    effects.retain(|e| seen.insert(e.anim_class.to_lowercase()));

    // Ensure default SmokeTrailEffect is first, then sort by display name
    effects.sort_by(|a, b| {
        if a.is_default {
            std::cmp::Ordering::Less
        } else if b.is_default {
            std::cmp::Ordering::Greater
        } else {
            a.display_name.cmp(&b.display_name)
        }
    });

    effects
}

#[inline]
pub fn parse_csv_line_borrowed<'a>(line: &'a str) -> Vec<&'a str> {
    let mut cols = Vec::with_capacity(32);
    let mut in_quotes = false;
    let mut start = 0;
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'"' {
            in_quotes = !in_quotes;
        } else if b == b',' && !in_quotes {
            cols.push(line[start..i].trim().trim_matches('"').trim());
            start = i + 1;
        }
    }
    cols.push(line[start..].trim().trim_matches('"').trim());
    cols
}

#[inline(always)]
fn get_col_str<'a>(cols: &[&'a str], idx: Option<usize>) -> &'a str {
    match idx {
        Some(i) if i < cols.len() => cols[i],
        _ => "",
    }
}

pub fn parse_csv_line(line: &str) -> Vec<String> {
    parse_csv_line_borrowed(line).into_iter().map(|s| s.to_string()).collect()
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
        .replace("TrailEffectType_", "")
        .replace("TrailEffectType", "")
        .replace("TrailEffect_", "")
        .replace("TrailEffect", "")
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

/// Loads authentic localization strings from Brawlhalla's language binary file (e.g. languages/language.1.bin).
pub fn load_language_strings(game_dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let candidates = [
        game_dir.join("languages").join("language.1.bin.original_backup"),
        game_dir.join("languages").join("language.1.bin.new_update_backup"),
        game_dir.join("languages").join("language.1.bin"),
    ];

    for path in &candidates {
        if let Ok(data) = std::fs::read(path) {
            if data.len() > 8 {
                use flate2::read::ZlibDecoder;
                use std::io::Read;
                let mut decoder = ZlibDecoder::new(&data[4..]);
                let mut decomp = Vec::new();
                if decoder.read_to_end(&mut decomp).is_ok() && decomp.len() >= 4 {
                    let mut i = 4;
                    while i + 2 <= decomp.len() {
                        let k_len = u16::from_be_bytes([decomp[i], decomp[i + 1]]) as usize;
                        i += 2;
                        if i + k_len > decomp.len() {
                            break;
                        }
                        let key = String::from_utf8_lossy(&decomp[i..i + k_len]).to_string();
                        i += k_len;

                        if i + 2 > decomp.len() {
                            break;
                        }
                        let v_len = u16::from_be_bytes([decomp[i], decomp[i + 1]]) as usize;
                        i += 2;
                        if i + v_len > decomp.len() {
                            break;
                        }
                        let val = String::from_utf8_lossy(&decomp[i..i + v_len]).to_string();
                        i += v_len;

                        map.insert(key, val);
                    }
                    if !map.is_empty() {
                        return map;
                    }
                }
            }
        }
    }
    map
}

