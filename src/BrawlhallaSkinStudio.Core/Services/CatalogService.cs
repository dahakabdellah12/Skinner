using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using System.Xml.Linq;
using BrawlhallaSkinStudio.Core.Models;
using BrawlhallaSwz;

namespace BrawlhallaSkinStudio.Core.Services;

public class CatalogService
{
    public List<LegendInfo> Legends { get; } = [];
    public List<CostumeInfo> Costumes { get; } = [];
    public List<WeaponSkinInfo> WeaponSkins { get; } = [];
    public List<ColorSchemeInfo> ColorSchemes { get; } = [];
    public Dictionary<string, List<WeaponSkinInfo>> WeaponSkinsByType { get; } = new(StringComparer.OrdinalIgnoreCase);
    public List<KoEffectInfo> KoEffects { get; } = [];
    public List<SidekickInfo> Sidekicks { get; } = [];
    public List<PodiumInfo> Podiums { get; } = [];
    public List<TrailEffectInfo> TrailEffects { get; } = [];
    public List<CrossoverSfxInfo> CrossoverSfx { get; } = [];

    public bool IsLoaded { get; private set; }
    public string LoadError { get; private set; } = string.Empty;

    public bool LoadFromInstallation(GameInstallation installation)
    {
        if (!installation.IsValid || !File.Exists(installation.GameSwzPath))
        {
            LoadError = "Invalid game installation or missing Game.swz.";
            return false;
        }

        try
        {
            // Find SWZ decryption key
            uint? key = SwzUtils.FindSwzKey(installation.BrawlhallaAirSwfPath, installation.GameSwzPath);
            if (key == null)
            {
                // Fallback to tested key
                key = 154281100u;
            }

            var decryptedFiles = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            using (var fs = new FileStream(installation.GameSwzPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            using (var swz = new SwzReader(fs, key.Value, SwzReaderOptions.IgnoreKeyChecksum))
            {
                foreach (string content in swz.ReadFiles())
                {
                    string fileName = SwzUtils.GetFileName(content);
                    decryptedFiles[fileName] = content;
                }
            }

            // Parse Color Schemes (ColorSchemeTypes.xml)
            if (decryptedFiles.TryGetValue("ColorSchemeTypes.xml", out var colorXml))
            {
                ParseColorSchemes(colorXml);
            }

            // Parse Legends (HeroTypes.xml)
            if (decryptedFiles.TryGetValue("HeroTypes.xml", out var heroXml))
            {
                ParseLegends(heroXml);
            }

            // Parse Costumes (costumeTypes.csv)
            if (decryptedFiles.TryGetValue("costumeTypes.csv", out var costumeCsv))
            {
                ParseCostumes(costumeCsv, installation.GamePath);
            }

            // Parse Weapon Skins (weaponSkinTypes.csv)
            if (decryptedFiles.TryGetValue("weaponSkinTypes.csv", out var weaponCsv))
            {
                ParseWeaponSkins(weaponCsv, installation.GamePath);
            }

            // Group weapon skins by base weapon type
            WeaponSkinsByType.Clear();
            foreach (var ws in WeaponSkins)
            {
                if (string.IsNullOrWhiteSpace(ws.BaseWeapon)) continue;
                if (!WeaponSkinsByType.TryGetValue(ws.BaseWeapon, out var list))
                {
                    list = [];
                    WeaponSkinsByType[ws.BaseWeapon] = list;
                }
                list.Add(ws);
            }

            // Associate Costumes with Legends
            var legendMap = Legends.ToDictionary(l => l.HeroName, StringComparer.OrdinalIgnoreCase);
            foreach (var costume in Costumes)
            {
                if (legendMap.TryGetValue(costume.OwnerHero, out var legend))
                {
                    legend.Costumes.Add(costume);
                    if (costume.IsDefault && string.IsNullOrEmpty(legend.PrimarySwf))
                    {
                        legend.PrimarySwf = costume.SwfFile;
                    }
                }
            }

            // Parse Podiums (PodiumTypes.xml)
            if (decryptedFiles.TryGetValue("PodiumTypes.xml", out var podiumXml))
            {
                ParsePodiums(podiumXml, installation.GamePath);
            }

            // Parse KO Effects & Trails (TrailEffectTypes.xml)
            if (decryptedFiles.TryGetValue("TrailEffectTypes.xml", out var trailXml))
            {
                ParseKoEffectsAndTrails(trailXml, installation.GamePath);
            }

            // Parse Sidekicks (SpawnBotTypes.xml)
            if (decryptedFiles.TryGetValue("SpawnBotTypes.xml", out var botXml))
            {
                ParseSidekicks(botXml, installation.GamePath);
            }

            // Scan Crossover SFX files
            ScanCrossoverSfx(installation.GamePath);

            IsLoaded = true;
            return true;
        }
        catch (Exception ex)
        {
            LoadError = $"Failed to parse game database: {ex.Message}";
            return false;
        }
    }

    private void ParseLegends(string xml)
    {
        Legends.Clear();
        var doc = XDocument.Parse(xml);
        var root = doc.Root;
        if (root == null) return;

        foreach (var elem in root.Elements("HeroType"))
        {
            string heroName = elem.Attribute("HeroName")?.Value ?? string.Empty;
            if (string.IsNullOrWhiteSpace(heroName) ||
                heroName.Equals("Template", StringComparison.OrdinalIgnoreCase) ||
                heroName.Equals("DEFAULT_CHARACTER", StringComparison.OrdinalIgnoreCase))
                continue;

            int heroId = 0;
            _ = int.TryParse(elem.Element("HeroID")?.Value, out heroId);

            string bio = elem.Element("BioName")?.Value ?? string.Empty;
            string displayName = !string.IsNullOrWhiteSpace(bio) ? bio : (elem.Element("HeroDisplayName")?.Value ?? heroName);
            if (displayName == "---" || string.IsNullOrWhiteSpace(displayName))
                displayName = heroName;
            if (heroName.Equals("Random", StringComparison.OrdinalIgnoreCase))
                displayName = "Random";
            string weapon1 = elem.Element("BaseWeapon1")?.Value ?? elem.Element("Weapon1")?.Value ?? string.Empty;
            string weapon2 = elem.Element("BaseWeapon2")?.Value ?? elem.Element("Weapon2")?.Value ?? string.Empty;
            string icon = elem.Element("IconName")?.Value ?? string.Empty;

            Legends.Add(new LegendInfo
            {
                HeroId = heroId,
                HeroName = heroName,
                DisplayName = displayName,
                Bio = bio,
                Weapon1 = weapon1,
                Weapon2 = weapon2,
                IconName = icon,
                PoseSymbol = $"a_Roster_Pose_{displayName.Replace(" ", "").Replace("ö", "o")}"
            });
        }

        // Sort by ID, placing Random at the end of the roster list
        Legends.Sort((a, b) =>
        {
            if (a.HeroName.Equals("Random", StringComparison.OrdinalIgnoreCase)) return 1;
            if (b.HeroName.Equals("Random", StringComparison.OrdinalIgnoreCase)) return -1;
            return a.HeroId.CompareTo(b.HeroId);
        });
    }

    private void ParseCostumes(string csv, string gamePath)
    {
        Costumes.Clear();
        using var reader = new StringReader(csv);
        _ = reader.ReadLine(); // table name
        string? headerLine = reader.ReadLine();
        if (headerLine == null) return;

        string[] headers = ParseCsvLine(headerLine);
        var colMap = headers.Select((h, i) => (h, i)).ToDictionary(x => x.h, x => x.i, StringComparer.OrdinalIgnoreCase);

        string? line;
        while ((line = reader.ReadLine()) != null)
        {
            if (string.IsNullOrWhiteSpace(line)) continue;
            string[] cols = ParseCsvLine(line);
            if (cols.Length < 10) continue;

            string name = GetCol(cols, colMap, "CostumeName");
            string owner = GetCol(cols, colMap, "OwnerHero");
            if (string.IsNullOrWhiteSpace(name) ||
                name.Equals("Template", StringComparison.OrdinalIgnoreCase) ||
                (name.Equals("Default", StringComparison.OrdinalIgnoreCase) && owner.Equals("DEFAULT_CHARACTER", StringComparison.OrdinalIgnoreCase)))
                continue;

            int id = 0;
            _ = int.TryParse(GetCol(cols, colMap, "CostumeID"), out id);
            string dispKey = GetCol(cols, colMap, "DisplayNameKey");
            string customArt = GetCol(cols, colMap, "GfxType.CustomArt");
            if (string.IsNullOrEmpty(customArt) || customArt == "--")
                customArt = GetCol(cols, colMap, "CustomArt");

            string customArt2 = GetCol(cols, colMap, "GfxType.CustomArt2");
            if (string.IsNullOrEmpty(customArt2) || customArt2 == "--")
                customArt2 = GetCol(cols, colMap, "CustomArt2");

            string customArt8 = GetCol(cols, colMap, "GfxType.CustomArt8");
            if (string.IsNullOrEmpty(customArt8) || customArt8 == "--")
                customArt8 = GetCol(cols, colMap, "CustomArt8");

            bool defaultUnlocked = GetCol(cols, colMap, "DefaultUnlocked").Equals("TRUE", StringComparison.OrdinalIgnoreCase);
            bool isCrossover = GetCol(cols, colMap, "IsCrossover").Equals("TRUE", StringComparison.OrdinalIgnoreCase);
            bool isMetadev = GetCol(cols, colMap, "IsMetadev").Equals("TRUE", StringComparison.OrdinalIgnoreCase);

            string swf = "";
            string symbol = "";

            // Primary artwork candidate selection (avoiding hand-only SWFs and Gfx_Player hide masks)
            var customArtCandidates = new List<string>();
            foreach (var colName in new[]
            {
                "GfxType.CustomArt2", "GfxType.CustomArt", "GfxType.CustomArt3", "GfxType.CustomArt4",
                "GfxType.CustomArt5", "GfxType.CustomArt6", "GfxType.CustomArt7", "GfxType.CustomArt9",
                "GfxType.CustomArt10", "GfxType.CustomArt8",
                "CustomArt2", "CustomArt", "CustomArt3", "CustomArt4",
                "CustomArt5", "CustomArt6", "CustomArt7", "CustomArt9",
                "CustomArt10", "CustomArt8"
            })
            {
                string val = GetCol(cols, colMap, colName);
                if (!string.IsNullOrEmpty(val) && val != "--" && val.Contains('/') && !customArtCandidates.Contains(val))
                {
                    customArtCandidates.Add(val);
                }
            }

            string artToUse = customArtCandidates.FirstOrDefault(c => IsValidCostumeArt(c, owner)) ?? "";
            if (string.IsNullOrEmpty(artToUse))
            {
                artToUse = customArtCandidates.FirstOrDefault(c => !c.Contains("Gfx_Hands.swf") && !c.Contains("/No"))
                    ?? customArtCandidates.FirstOrDefault() ?? "";
            }

            if (!string.IsNullOrEmpty(artToUse) && artToUse.Contains('/'))
            {
                var parts = artToUse.Split('/');
                swf = parts[0];
                symbol = parts[1];
            }
            else if (!string.IsNullOrWhiteSpace(owner) && owner != "--" && owner != "DEFAULT_CHARACTER")
            {
                swf = $"Gfx_{owner}.swf";
                symbol = name;
            }
            else
            {
                swf = "Gfx_Costumes.swf";
                symbol = name;
            }

            if (!string.IsNullOrEmpty(swf) && !swf.EndsWith(".swf", StringComparison.OrdinalIgnoreCase))
            {
                swf += ".swf";
            }

            // Derive display name from costume name or key
            string friendlyName = FormatDisplayName(name, dispKey);

            string icon = GetCol(cols, colMap, "CostumeIcon");
            string iconFile = GetCol(cols, colMap, "CostumeIconFileName");
            if (string.IsNullOrEmpty(iconFile) || iconFile == "--" || iconFile == "0")
                iconFile = "UI_Icons.swf";
            else if (!iconFile.EndsWith(".swf", StringComparison.OrdinalIgnoreCase))
                iconFile += ".swf";

            string weaponSet = GetCol(cols, colMap, "WeaponSet");
            if (weaponSet == "--") weaponSet = "";

            // Hands style
            string handsArt = customArtCandidates.FirstOrDefault(c => c.StartsWith("Gfx_Hands.swf/", StringComparison.OrdinalIgnoreCase)) ?? "";
            string handsSuffix = "";
            if (!string.IsNullOrEmpty(handsArt) && handsArt.Contains('/'))
            {
                handsSuffix = handsArt.Split('/')[1];
            }

            // Capes & Head Art & Overrides
            string defaultCape = GetCol(cols, colMap, "DefaultCape");
            if (defaultCape == "--" || defaultCape == "0" || defaultCape.Equals("FALSE", StringComparison.OrdinalIgnoreCase)) defaultCape = "";

            string headGfx = GetCol(cols, colMap, "HeadGfxCustomArt");
            if (headGfx == "--" || headGfx == "0") headGfx = "";

            string boneOverride = GetCol(cols, colMap, "BoneOverride");
            if (boneOverride == "--" || boneOverride.Equals("FALSE", StringComparison.OrdinalIgnoreCase)) boneOverride = "";

            string platformAnim = GetCol(cols, colMap, "PlatformAnimOverride");
            if (platformAnim == "--" || platformAnim.Equals("FALSE", StringComparison.OrdinalIgnoreCase)) platformAnim = "";

            // Crossover Swaps (Animations and SFX)
            var swapArts = new List<string>();
            foreach (var swapCol in new[] { "SwapCustomArt", "SwapCustomArt2", "SwapCustomArt3", "SwapCustomArt4", "SwapCustomArt5" })
            {
                string sVal = GetCol(cols, colMap, swapCol);
                if (!string.IsNullOrEmpty(sVal) && sVal != "--" && sVal.Contains('/'))
                {
                    swapArts.Add(sVal);
                }
            }

            var costume = new CostumeInfo
            {
                CostumeId = id,
                CostumeName = name,
                OwnerHero = owner,
                DisplayName = friendlyName,
                IsDefault = (defaultUnlocked && (name.Equals(owner, StringComparison.OrdinalIgnoreCase) || string.IsNullOrEmpty(dispKey)))
                    || (owner.Equals("Random", StringComparison.OrdinalIgnoreCase) && name.Equals("Random", StringComparison.OrdinalIgnoreCase)),
                SwfFile = swf,
                SymbolSuffix = symbol,
                WeaponSet = weaponSet,
                IconName = (icon != "--" && !string.IsNullOrWhiteSpace(icon)) ? icon : "",
                IconFileName = iconFile,
                IsCrossover = isCrossover,
                IsMetadev = isMetadev,
                HandsArt = handsArt,
                HandsSuffix = handsSuffix,
                CustomArtLayers = customArtCandidates,
                DefaultCape = defaultCape,
                HeadGfxCustomArt = headGfx,
                BoneOverride = boneOverride,
                PlatformAnimOverride = platformAnim,
                SwapCustomArts = swapArts
            };

            // Parse base palette define colors
            foreach (var kvp in colMap)
            {
                if (kvp.Key.EndsWith("_Define", StringComparison.OrdinalIgnoreCase))
                {
                    string hex = GetCol(cols, colMap, kvp.Key);
                    if (TryParseHex(hex, out uint val))
                    {
                        costume.BaseColors[kvp.Key.Replace("_Define", "")] = val;
                    }
                }
            }

            Costumes.Add(costume);
        }
    }

    private void ParseWeaponSkins(string csv, string gamePath)
    {
        WeaponSkins.Clear();
        using var reader = new StringReader(csv);
        _ = reader.ReadLine(); // table name
        string? headerLine = reader.ReadLine();
        if (headerLine == null) return;

        string[] headers = ParseCsvLine(headerLine);
        var colMap = headers.Select((h, i) => (h, i)).ToDictionary(x => x.h, x => x.i, StringComparer.OrdinalIgnoreCase);

        string? line;
        while ((line = reader.ReadLine()) != null)
        {
            if (string.IsNullOrWhiteSpace(line)) continue;
            string[] cols = ParseCsvLine(line);
            if (cols.Length < 6) continue;

            string name = GetCol(cols, colMap, "WeaponSkinName");
            if (string.IsNullOrWhiteSpace(name) || name.Equals("Template", StringComparison.OrdinalIgnoreCase))
                continue;

            int id = 0;
            _ = int.TryParse(GetCol(cols, colMap, "WeaponSkinID"), out id);
            string baseWeapon = GetCol(cols, colMap, "BaseWeapon");
            string owner = GetCol(cols, colMap, "OwnerHero");
            string dispKey = GetCol(cols, colMap, "DisplayNameKey");
            string customArt = GetCol(cols, colMap, "CustomArt");
            string customArt2 = GetCol(cols, colMap, "CustomArt2");
            string customArt3 = GetCol(cols, colMap, "CustomArt3");

            string artToUse = "";
            if (!string.IsNullOrEmpty(customArt) && customArt != "--" && !customArt.Contains("Gfx_Hands.swf", StringComparison.OrdinalIgnoreCase))
                artToUse = customArt;
            else if (!string.IsNullOrEmpty(customArt2) && customArt2 != "--")
                artToUse = customArt2;
            else if (!string.IsNullOrEmpty(customArt3) && customArt3 != "--")
                artToUse = customArt3;
            else if (!string.IsNullOrEmpty(customArt) && customArt != "--")
                artToUse = customArt;

            if (artToUse.StartsWith("C:", StringComparison.OrdinalIgnoreCase))
                artToUse = artToUse[2..];

            string swf = "";
            string symbol = "";
            if (!string.IsNullOrEmpty(artToUse) && artToUse.Contains('/'))
            {
                var parts = artToUse.Split('/');
                swf = parts[0];
                symbol = parts[1];
            }
            else if (!string.IsNullOrWhiteSpace(owner) && owner != "--")
            {
                swf = $"Gfx_{owner}.swf";
                symbol = name;
            }
            else
            {
                swf = "Gfx_Weapons.swf";
                symbol = name;
            }

            if (swf.StartsWith("C:", StringComparison.OrdinalIgnoreCase))
                swf = swf[2..];

            if (!string.IsNullOrEmpty(swf) && !swf.EndsWith(".swf", StringComparison.OrdinalIgnoreCase))
            {
                swf += ".swf";
            }

            string friendlyName = FormatDisplayName(name, dispKey);
            string weaponIcon = GetCol(cols, colMap, "WeaponIcon");
            if (weaponIcon == "--") weaponIcon = "";
            string inheritCostume = GetCol(cols, colMap, "InheritCostumeDefines");
            if (inheritCostume == "--") inheritCostume = "";
            string soundBank = GetCol(cols, colMap, "SoundBank");
            if (soundBank == "--") soundBank = "";

            var soundOverrides = new List<string>();
            for (int so = 1; so <= 20; so++)
            {
                string ovr = GetCol(cols, colMap, $"AttackSoundOverride{so}");
                if (!string.IsNullOrWhiteSpace(ovr) && ovr != "--")
                {
                    soundOverrides.Add(ovr);
                }
            }

            var weapon = new WeaponSkinInfo
            {
                WeaponSkinId = id,
                WeaponSkinName = name,
                BaseWeapon = baseWeapon,
                OwnerHero = owner,
                DisplayName = friendlyName,
                SwfFile = swf,
                SymbolSuffix = symbol,
                WeaponIcon = weaponIcon,
                InheritCostumeDefines = inheritCostume,
                SoundBank = soundBank,
                AttackSoundOverrides = soundOverrides,
                IsDefault = name.EndsWith(owner, StringComparison.OrdinalIgnoreCase) || id <= 15
            };

            // Parse base palette define colors
            foreach (var kvp in colMap)
            {
                if (kvp.Key.EndsWith("_Define", StringComparison.OrdinalIgnoreCase))
                {
                    string hex = GetCol(cols, colMap, kvp.Key);
                    if (TryParseHex(hex, out uint val))
                    {
                        weapon.BaseColors[kvp.Key.Replace("_Define", "")] = val;
                    }
                }
            }

            WeaponSkins.Add(weapon);
        }
    }

    private void ParseColorSchemes(string xml)
    {
        ColorSchemes.Clear();
        var doc = XDocument.Parse(xml);
        var root = doc.Root;
        if (root == null) return;

        foreach (var elem in root.Elements("ColorSchemeType"))
        {
            string name = elem.Attribute("ColorSchemeName")?.Value ?? string.Empty;
            if (string.IsNullOrWhiteSpace(name) || name.Equals("Template", StringComparison.OrdinalIgnoreCase))
                continue;

            int id = 0;
            _ = int.TryParse(elem.Element("ColorSchemeID")?.Value, out id);
            string dispKey = elem.Element("DisplayNameKey")?.Value ?? name;
            string friendlyName = FormatDisplayName(name, dispKey);
            string rarity = elem.Element("Rarity")?.Value ?? "Common";

            uint indicator = 0xFFFFFF;
            string? indStr = elem.Element("IndicatorColor")?.Value;
            if (!string.IsNullOrEmpty(indStr))
                TryParseHex(indStr, out indicator);

            var scheme = new ColorSchemeInfo
            {
                SchemeId = id,
                SchemeName = name,
                DisplayName = friendlyName,
                Rarity = rarity,
                IndicatorColor = indicator
            };

            foreach (var child in elem.Elements())
            {
                if (child.Name.LocalName.EndsWith("_Swap", StringComparison.OrdinalIgnoreCase))
                {
                    string key = child.Name.LocalName.Replace("_Swap", "");
                    if (TryParseHex(child.Value, out uint val))
                    {
                        scheme.SwapColors[key] = val;
                    }
                }
            }

            ColorSchemes.Add(scheme);
        }

        // Sort by ID
        ColorSchemes.Sort((a, b) => a.SchemeId.CompareTo(b.SchemeId));
    }

    private static string GetCol(string[] cols, Dictionary<string, int> map, string colName)
    {
        if (map.TryGetValue(colName, out int idx) && idx < cols.Length)
        {
            return cols[idx].Trim('"', ' ', '\t');
        }
        return string.Empty;
    }

    private static string FormatDisplayName(string internalName, string key)
    {
        if (!string.IsNullOrWhiteSpace(key))
        {
            string clean = key.Replace("CostumeType_", "")
                              .Replace("WeaponSkinType_", "")
                              .Replace("ColorSchemeType_", "")
                              .Replace("_DisplayName", "");
            if (!string.IsNullOrWhiteSpace(clean))
            {
                // Split CamelCase
                return Regex.Replace(clean, "([a-z])([A-Z])", "$1 $2");
            }
        }
        return Regex.Replace(internalName, "([a-z])([A-Z])", "$1 $2");
    }

    private static bool TryParseHex(string str, out uint result)
    {
        result = 0;
        if (string.IsNullOrWhiteSpace(str)) return false;
        str = str.Trim();
        if (str.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
            str = str.Substring(2);
        else if (str.StartsWith("#"))
            str = str.Substring(1);

        return uint.TryParse(str, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out result);
    }

    private static string[] ParseCsvLine(string line)
    {
        var result = new List<string>();
        bool inQuotes = false;
        int start = 0;

        for (int i = 0; i < line.Length; i++)
        {
            if (line[i] == '"')
            {
                inQuotes = !inQuotes;
            }
            else if (line[i] == ',' && !inQuotes)
            {
                result.Add(line.Substring(start, i - start));
                start = i + 1;
            }
        }
        result.Add(line.Substring(start));
        return result.ToArray();
    }

    private void ParsePodiums(string xml, string gamePath)
    {
        Podiums.Clear();
        try
        {
            var doc = XDocument.Parse(xml);
            foreach (var elem in doc.Root?.Elements("PodiumType") ?? Enumerable.Empty<XElement>())
            {
                string name = elem.Attribute("PodiumName")?.Value ?? "";
                if (string.IsNullOrWhiteSpace(name) || name.Equals("Template", StringComparison.OrdinalIgnoreCase))
                    continue;

                int.TryParse(elem.Element("PodiumID")?.Value, out int id);
                string dispKey = elem.Element("DisplayNameKey")?.Value ?? name;
                string animFile = elem.Element("AnimFile")?.Value ?? "UI_Podiums.swf";
                if (!string.IsNullOrEmpty(animFile) && !animFile.EndsWith(".swf", StringComparison.OrdinalIgnoreCase))
                    animFile += ".swf";

                string customArt = elem.Element("AnimCustomArt")?.Value ?? "";
                string soundReady = elem.Element("SoundEventReady")?.Value ?? "";
                bool isDefault = name.Equals("Default", StringComparison.OrdinalIgnoreCase) || id == 1;

                Podiums.Add(new PodiumInfo
                {
                    Id = id,
                    Name = name,
                    DisplayName = FormatDisplayName(name, dispKey),
                    AnimFile = animFile,
                    CustomArt = customArt,
                    SoundReady = soundReady,
                    IsDefault = isDefault
                });
            }
        }
        catch { }
    }

    private void ParseKoEffectsAndTrails(string xml, string gamePath)
    {
        KoEffects.Clear();
        TrailEffects.Clear();
        try
        {
            var doc = XDocument.Parse(xml);
            foreach (var elem in doc.Root?.Elements("TrailEffectType") ?? Enumerable.Empty<XElement>())
            {
                string name = elem.Attribute("TrailEffectName")?.Value ?? "";
                if (string.IsNullOrWhiteSpace(name) || name.Equals("Template", StringComparison.OrdinalIgnoreCase))
                    continue;

                int.TryParse(elem.Element("TrailEffectID")?.Value, out int id);
                string dispKey = elem.Element("DisplayNameKey")?.Value ?? name;
                string animFile = elem.Element("KOGfx")?.Element("AnimFile")?.Value ?? "SFX_KO.swf";
                if (!string.IsNullOrEmpty(animFile) && !animFile.EndsWith(".swf", StringComparison.OrdinalIgnoreCase))
                    animFile += ".swf";

                string soundEvent = elem.Element("KOSoundEvent")?.Value ?? "UI_InGame_KO_Default_Play";
                bool isDefault = name.Equals("SmokeTrailEffect", StringComparison.OrdinalIgnoreCase) || id == 1;

                var ko = new KoEffectInfo
                {
                    Id = id,
                    Name = name,
                    DisplayName = FormatDisplayName(name, dispKey),
                    AnimFile = animFile,
                    SoundEvent = soundEvent,
                    IsDefault = isDefault
                };
                KoEffects.Add(ko);

                TrailEffects.Add(new TrailEffectInfo
                {
                    Id = id,
                    Name = name,
                    DisplayName = FormatDisplayName(name, dispKey),
                    AnimFile = animFile,
                    IsDefault = isDefault
                });
            }
        }
        catch { }
    }

    private void ParseSidekicks(string xml, string gamePath)
    {
        Sidekicks.Clear();
        try
        {
            var doc = XDocument.Parse(xml);
            foreach (var elem in doc.Root?.Elements("SpawnBotType") ?? Enumerable.Empty<XElement>())
            {
                string name = elem.Attribute("SpawnBotName")?.Value ?? "";
                if (string.IsNullOrWhiteSpace(name) || name.Equals("Template", StringComparison.OrdinalIgnoreCase))
                    continue;

                int.TryParse(elem.Element("SpawnBotID")?.Value, out int id);
                string dispKey = elem.Element("DisplayNameKey")?.Value ?? name;
                string animFile = elem.Element("GfxType")?.Element("AnimFile")?.Value ?? "Animation_Robot.swf";
                if (!string.IsNullOrEmpty(animFile) && !animFile.EndsWith(".swf", StringComparison.OrdinalIgnoreCase))
                    animFile += ".swf";

                string customArt = elem.Element("GfxType")?.Element("CustomArt")?.Value ?? "";
                bool isDefault = name.Equals("DEFAULT", StringComparison.OrdinalIgnoreCase) || id == 1;

                Sidekicks.Add(new SidekickInfo
                {
                    Id = id,
                    Name = name,
                    DisplayName = FormatDisplayName(name, dispKey),
                    AnimFile = animFile,
                    CustomArt = customArt,
                    IsDefault = isDefault
                });
            }
        }
        catch { }
    }

    private void ScanCrossoverSfx(string gamePath)
    {
        CrossoverSfx.Clear();
        if (!Directory.Exists(gamePath)) return;

        var baseSfx = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "SFX_1.swf", "SFX_Axe.swf", "SFX_Bow.swf", "SFX_Cannon.swf", "SFX_Chakram.swf",
            "SFX_Fists.swf", "SFX_GameModes.swf", "SFX_GreatSword.swf", "SFX_Hammer.swf",
            "SFX_HitReacts.swf", "SFX_Katar.swf", "SFX_KO.swf", "SFX_KO2.swf", "SFX_Lance.swf",
            "SFX_Level.swf", "SFX_Orb.swf", "SFX_Pistols.swf", "SFX_Scythe.swf", "SFX_Spear.swf",
            "SFX_Sword.swf", "SFX_Emote.swf", "SFX_SWBlasters.swf", "SFX_Sabers.swf"
        };

        var files = Directory.GetFiles(gamePath, "SFX_*.swf");
        foreach (var file in files)
        {
            string fileName = Path.GetFileName(file);
            if (baseSfx.Contains(fileName)) continue;

            string charName = fileName.Substring(4, fileName.Length - 8); // remove SFX_ and .swf
            CrossoverSfx.Add(new CrossoverSfxInfo
            {
                SfxFile = fileName,
                CharacterName = charName,
                DisplayName = Regex.Replace(charName, "([a-z])([A-Z])", "$1 $2")
            });
        }
    }

    private static bool IsValidCostumeArt(string art, string owner)
    {
        if (string.IsNullOrWhiteSpace(art) || art == "--" || !art.Contains('/')) return false;
        var parts = art.Split('/');
        string swf = parts[0];
        string sym = parts[1];

        // Reject hands-only accessory layer
        if (swf.Equals("Gfx_Hands.swf", StringComparison.OrdinalIgnoreCase)) return false;

        // Reject Gfx_Player hide masks (NoAccent, NoEyes, NoNose, NoMouth, NoJaw, NoFoot)
        if (swf.Equals("Gfx_Player.swf", StringComparison.OrdinalIgnoreCase))
        {
            if (sym.StartsWith("No", StringComparison.OrdinalIgnoreCase)) return false;
            if (owner.Equals("Random", StringComparison.OrdinalIgnoreCase)) return true;
        }

        // Reject All... accessory layer (CustomArt8)
        if (sym.StartsWith("All", StringComparison.OrdinalIgnoreCase)) return false;

        return true;
    }
}
