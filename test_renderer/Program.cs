using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using BrawlhallaSkinStudio.Core.Services;

class Program
{
    static void Main(string[] args)
    {
        // Read JSON requests line-by-line from stdin, write JSON response to stdout
        Console.OutputEncoding = System.Text.Encoding.UTF8;
        string? line;
        while ((line = Console.ReadLine()) != null)
        {
            if (string.IsNullOrWhiteSpace(line)) continue;
            try
            {
                using var doc = JsonDocument.Parse(line);
                var root = doc.RootElement;
                string? type = root.TryGetProperty("type", out var tProp) ? tProp.GetString() : null;
                string gamePath = root.TryGetProperty("gamePath", out var gpProp) ? gpProp.GetString() ?? "" : "";
                string uiIcons = Path.Combine(gamePath, "UI_Icons.swf");

                string? svg = null;

                if ("legend".Equals(type, StringComparison.OrdinalIgnoreCase))
                {
                    string? name = root.TryGetProperty("name", out var nProp) ? nProp.GetString() : null;
                    string? hero = root.TryGetProperty("hero", out var hProp) ? hProp.GetString() : null;
                    Dictionary<uint, uint>? colorMap = ParseColorMap(root);
                    svg = ResolveLegendPose(uiIcons, name, hero, colorMap);
                }
                else if ("color".Equals(type, StringComparison.OrdinalIgnoreCase))
                {
                    string? icon = root.TryGetProperty("icon", out var iProp) ? iProp.GetString() : null;
                    if (!string.IsNullOrEmpty(icon))
                    {
                        svg = PreviewRenderer.RenderSymbolToSvg(uiIcons, icon, null, null);
                    }
                }
                else if ("koEffect".Equals(type, StringComparison.OrdinalIgnoreCase) || "ko".Equals(type, StringComparison.OrdinalIgnoreCase))
                {
                    string? animFile = root.TryGetProperty("animFile", out var fProp) ? fProp.GetString() : null;
                    if (string.IsNullOrEmpty(animFile) && root.TryGetProperty("swfFile", out var sfPropKo))
                    {
                        animFile = sfPropKo.GetString();
                    }
                    string? animClass = root.TryGetProperty("animClass", out var cProp) ? cProp.GetString() : null;
                    if (string.IsNullOrEmpty(animClass) && root.TryGetProperty("symbol", out var sPropKo))
                    {
                        animClass = sPropKo.GetString();
                    }

                    if (!string.IsNullOrEmpty(animFile) && !string.IsNullOrEmpty(animClass))
                    {
                        string fullSwf = Path.Combine(gamePath, animFile);
                        if (File.Exists(fullSwf))
                        {
                            svg = PreviewRenderer.RenderSymbolToSvg(fullSwf, animClass, null, null);
                        }
                    }
                }
                else if ("costume".Equals(type, StringComparison.OrdinalIgnoreCase))
                {
                    string? icon = root.TryGetProperty("icon", out var iProp) ? iProp.GetString() : null;
                    string? iconFile = root.TryGetProperty("iconFile", out var ifProp) ? ifProp.GetString() : null;
                    string? costumeName = root.TryGetProperty("costumeName", out var cnProp) ? cnProp.GetString() : null;
                    string? ownerHero = root.TryGetProperty("ownerHero", out var ohProp) ? ohProp.GetString() : null;
                    string? swfFile = root.TryGetProperty("swfFile", out var sfProp) ? sfProp.GetString() : null;

                    Dictionary<uint, uint>? colorMap = ParseColorMap(root);

                    // Check classic
                    if ((!string.IsNullOrEmpty(icon) && icon.EndsWith("_Classic", StringComparison.OrdinalIgnoreCase)) ||
                        (!string.IsNullOrEmpty(costumeName) && costumeName.Equals("Classic", StringComparison.OrdinalIgnoreCase)))
                    {
                        svg = ResolveLegendPose(uiIcons, ownerHero ?? costumeName, ownerHero, colorMap);
                        if (svg == null && !string.IsNullOrEmpty(icon))
                        {
                            svg = PreviewRenderer.RenderSymbolToSvg(uiIcons, icon, null, colorMap);
                        }
                    }

                    if (svg == null)
                    {
                        string targetFile = !string.IsNullOrEmpty(iconFile) ? Path.Combine(gamePath, iconFile) : uiIcons;
                        if (!File.Exists(targetFile)) targetFile = uiIcons;

                        if (!string.IsNullOrEmpty(icon) && !icon.EndsWith("_Classic", StringComparison.OrdinalIgnoreCase))
                        {
                            svg = PreviewRenderer.RenderSymbolToSvg(targetFile, icon, null, colorMap);
                        }
                    }

                    if (svg == null && !string.IsNullOrEmpty(costumeName) && !costumeName.Equals("Classic", StringComparison.OrdinalIgnoreCase))
                    {
                        svg = PreviewRenderer.RenderSymbolToSvg(uiIcons, $"a_SkinIcon_{ownerHero}_{costumeName}", null, colorMap)
                           ?? PreviewRenderer.RenderSymbolToSvg(uiIcons, $"a_SkinIcon_{costumeName}", null, colorMap);
                    }

                    if (svg == null && !string.IsNullOrEmpty(swfFile))
                    {
                        string fullSwf = Path.Combine(gamePath, swfFile);
                        svg = PreviewRenderer.RenderSymbolToSvg(fullSwf, $"a_Head_{costumeName}", null, colorMap)
                           ?? PreviewRenderer.RenderSymbolToSvg(fullSwf, $"a_Torso_{costumeName}", null, colorMap);
                    }

                    if (svg == null && !string.IsNullOrEmpty(ownerHero))
                    {
                        svg = ResolveLegendPose(uiIcons, ownerHero, ownerHero, colorMap);
                    }
                }
                else if ("weapon".Equals(type, StringComparison.OrdinalIgnoreCase))
                {
                    string? swf = root.TryGetProperty("swfFile", out var swfProp) ? swfProp.GetString() : null;
                    string? symbol = root.TryGetProperty("symbol", out var symProp) ? symProp.GetString() : null;
                    string? baseWeapon = root.TryGetProperty("baseWeapon", out var bwProp) ? bwProp.GetString() : null;
                    string? ownerHero = root.TryGetProperty("ownerHero", out var ohProp) ? ohProp.GetString() : null;
                    Dictionary<uint, uint>? colorMap = ParseColorMap(root);

                    string bw = baseWeapon?.Trim() ?? "";
                    bool isPairedType = bw.Equals("Katar", StringComparison.OrdinalIgnoreCase) ||
                                       bw.Equals("Katars", StringComparison.OrdinalIgnoreCase) ||
                                       bw.Equals("Boots", StringComparison.OrdinalIgnoreCase) ||
                                       bw.Equals("BattleBoots", StringComparison.OrdinalIgnoreCase) ||
                                       bw.Equals("Fists", StringComparison.OrdinalIgnoreCase) ||
                                       bw.Equals("Gauntlet", StringComparison.OrdinalIgnoreCase) ||
                                       bw.Equals("Gauntlets", StringComparison.OrdinalIgnoreCase) ||
                                       bw.Equals("Chakram", StringComparison.OrdinalIgnoreCase);

                    if (isPairedType)
                    {
                        svg = PreviewRenderer.RenderPairedWeaponToSvg(gamePath, bw, symbol ?? "", swf, ownerHero, colorMap);
                    }

                    if (svg == null && !string.IsNullOrEmpty(symbol))
                    {
                        var swfList = new List<string>();
                        if (!string.IsNullOrEmpty(swf)) swfList.Add(Path.Combine(gamePath, swf));
                        if (!string.IsNullOrEmpty(ownerHero) && !ownerHero.Equals("--", StringComparison.OrdinalIgnoreCase))
                        {
                            string heroSwf = Path.Combine(gamePath, $"Gfx_{ownerHero}.swf");
                            if (File.Exists(heroSwf) && !swfList.Contains(heroSwf, StringComparer.OrdinalIgnoreCase))
                                swfList.Add(heroSwf);
                        }
                        string[] fallbacks = new[]
                        {
                            Path.Combine(gamePath, "Gfx_Weapons.swf"),
                            Path.Combine(gamePath, "Gfx_Weapons_02.swf"),
                            Path.Combine(gamePath, "Gfx_Weapons_04.swf"),
                            Path.Combine(gamePath, "Gfx_Weapons_Chakram.swf"),
                            Path.Combine(gamePath, "Gfx_Weapons_OneOff.swf"),
                            Path.Combine(gamePath, "Gfx_Weapons_Sets.swf"),
                            Path.Combine(gamePath, "Gfx_Super.swf")
                        };
                        foreach (var fb in fallbacks)
                        {
                            if (!swfList.Contains(fb, StringComparer.OrdinalIgnoreCase) && File.Exists(fb))
                                swfList.Add(fb);
                        }

                        var candidateSymbols = GetWeaponCandidateSymbols(bw, symbol);
                        foreach (var targetSwf in swfList)
                        {
                            if (!File.Exists(targetSwf)) continue;
                            foreach (var sym in candidateSymbols)
                            {
                                svg = PreviewRenderer.RenderSymbolToSvg(targetSwf, sym, null, colorMap);
                                if (svg != null) break;
                            }
                            if (svg != null) break;
                        }
                    }
                }

                var resp = new { svg = svg };
                Console.WriteLine(JsonSerializer.Serialize(resp));
            }
            catch (Exception ex)
            {
                var errResp = new { svg = (string?)null, error = ex.Message };
                Console.WriteLine(JsonSerializer.Serialize(errResp));
            }
        }
    }

    static Dictionary<uint, uint>? ParseColorMap(JsonElement root)
    {
        if (root.TryGetProperty("colorMap", out var cmProp) && cmProp.ValueKind == JsonValueKind.Object)
        {
            var map = new Dictionary<uint, uint>();
            foreach (var prop in cmProp.EnumerateObject())
            {
                if (uint.TryParse(prop.Name, out uint k) && prop.Value.TryGetUInt32(out uint v))
                {
                    map[k] = v;
                    map[k & 0xFFFFFF] = v & 0xFFFFFF;
                }
            }
            if (map.Count > 0) return map;
        }
        return null;
    }

    private static readonly Dictionary<string, string> LegendPoseOverrides = new(StringComparer.OrdinalIgnoreCase)
    {
        ["Lord Vraxx"] = "a_Roster_Pose_Vraxx",
        ["Alien"] = "a_Roster_Pose_Vraxx",
        ["Orion"] = "a_Roster_Pose_Volst",
        ["Valkyrie"] = "a_Roster_Pose_Volst",
        ["Gnash"] = "a_Roster_Pose_Thrulk",
        ["Caveman"] = "a_Roster_Pose_Thrulk",
        ["Sir Roland"] = "a_Roster_Pose_Hotspur",
        ["Knight"] = "a_Roster_Pose_Hotspur",
        ["Scarlet"] = "a_SkinIcon_Scarlet_Classic",
        ["Steampunk"] = "a_SkinIcon_Scarlet_Classic",
        ["Seven"] = "a_Roster_Pose_Roboengineer",
        ["Roboengineer"] = "a_Roster_Pose_Roboengineer",
        ["Brynn"] = "a_Roster_Pose_Valkyrie",
        ["ActualValk"] = "a_Roster_Pose_Valkyrie",
        ["Diana"] = "a_Roster_Pose_MonsterHunter",
        ["Marksman"] = "a_Roster_Pose_MonsterHunter",
        ["Mirage"] = "a_Roster_Pose_Assassin",
        ["Egyptian"] = "a_Roster_Pose_Assassin",
        ["Vivi"] = "a_Roster_Pose_AssassinVivi",
        ["Assassin"] = "a_Roster_Pose_AssassinVivi",
        ["Yumiko"] = "a_Roster_Pose_Kitsune",
        ["Ninetails"] = "a_Roster_Pose_Kitsune",
        ["Teros"] = "a_SkinIcon_Minotaur_Classic",
        ["Minotaur"] = "a_SkinIcon_Minotaur_Classic",
        ["Jhala"] = "a_SkinIcon_Barbarian_Classic",
        ["Barbarian"] = "a_SkinIcon_Barbarian_Classic",
        ["Wu Shang"] = "a_SkinIcon_Monk_Classic",
        ["Monk"] = "a_SkinIcon_Monk_Classic",
        ["Ragnir"] = "a_SkinIcon_Dragon_Classic",
        ["Dragon"] = "a_SkinIcon_Dragon_Classic",
        ["Nix"] = "a_SkinIcon_Reaper_Classic",
        ["Reaper"] = "a_SkinIcon_Reaper_Classic",
        ["Artemis"] = "a_SkinIcon_SpaceHunter_Classic",
        ["Spacehunter"] = "a_SkinIcon_SpaceHunter_Classic",
        ["SpaceHunter"] = "a_SkinIcon_SpaceHunter_Classic",
        ["Isaiah"] = "a_SkinIcon_Soldier_Classic",
        ["Soldier"] = "a_SkinIcon_Soldier_Classic",
        ["Thor"] = "a_SkinIcon_Thor_Classic",
        ["Magyar"] = "a_SkinIcon_GhostArmor_Classic",
        ["GhostArmor"] = "a_SkinIcon_GhostArmor_Classic",
        ["Arcadia"] = "a_SkinIcon_FairyQueen_Classic",
        ["FairyQueen"] = "a_SkinIcon_FairyQueen_Classic",
        ["Zariel"] = "a_SkinIcon_Celestial_Classic",
        ["Celestial"] = "a_SkinIcon_Celestial_Classic",
        ["Petra"] = "a_Roster_Pose_Ragefighter",
        ["RageFighter"] = "a_Roster_Pose_Ragefighter",
        ["Thea"] = "a_Roster_Pose_Speedster",
        ["GreekSpeedster"] = "a_Roster_Pose_Speedster",
        ["Ransom"] = "a_Roster_Pose_Cybervirus",
        ["CyberVirus"] = "a_Roster_Pose_Cybervirus",
        ["Aurus"] = "a_Roster_Pose_Gladiator",
        ["ActualGladiator"] = "a_Roster_Pose_Gladiator",
        ["Random"] = "a_Roster_Pose_Random",
        ["Viking"] = "a_Roster_Pose_Bodvar",
        ["Bodvar"] = "a_Roster_Pose_Bodvar",
        ["Bödvar"] = "a_Roster_Pose_Bodvar",
        ["Onyx"] = "a_Roster_Pose_Gargoyle",
        ["Gargoyle"] = "a_Roster_Pose_Gargoyle",
        ["Azoth"] = "a_Roster_Pose_Azoth",
        ["Skeleton"] = "a_Roster_Pose_Azoth",
        ["Vector"] = "a_Roster_Pose_ActualRobot",
        ["ActualRobot"] = "a_Roster_Pose_ActualRobot",
        ["Volkov"] = "a_Roster_Pose_VampLord",
        ["VampLord"] = "a_Roster_Pose_VampLord",
        ["Munin"] = "a_Roster_Pose_BirdBard",
        ["BirdBard"] = "a_Roster_Pose_BirdBard",
        ["Loki"] = "a_Roster_Pose_Loki",
        ["Imugi"] = "a_Roster_Pose_Imugi",
        ["King Zuva"] = "a_Roster_Pose_AfricanKing",
        ["Zuva"] = "a_Roster_Pose_AfricanKing",
        ["AfricanKing"] = "a_Roster_Pose_AfricanKing",
        ["Priya"] = "a_Roster_Pose_BladeDancer",
        ["BladeDancer"] = "a_Roster_Pose_BladeDancer",
        ["Lady Vera"] = "a_Roster_Pose_Cleric",
        ["Vera"] = "a_Roster_Pose_Cleric",
        ["Cleric"] = "a_Roster_Pose_Cleric",
        ["Rupture"] = "a_Roster_Pose_DarkheartMonster",
        ["DarkheartMonster"] = "a_Roster_Pose_DarkheartMonster",
        ["Qinghua & Baobao"] = "a_Roster_Pose_AstroGirl",
        ["Qinghua"] = "a_Roster_Pose_AstroGirl",
        ["Baobao"] = "a_Roster_Pose_AstroGirl",
        ["AstroGirl"] = "a_Roster_Pose_AstroGirl"
    };

    static string? ResolveLegendPose(string uiIcons, string? name, string? hero, Dictionary<uint, uint>? colorMap = null)
    {
        if (string.IsNullOrEmpty(name) && string.IsNullOrEmpty(hero)) return null;

        var candidates = new List<string>();

        if (!string.IsNullOrEmpty(name) && LegendPoseOverrides.TryGetValue(name, out var ov1))
            candidates.Add(ov1);
        if (!string.IsNullOrEmpty(hero) && LegendPoseOverrides.TryGetValue(hero, out var ov2))
            candidates.Add(ov2);

        if (!string.IsNullOrEmpty(name))
        {
            string clean = name.Replace(" ", "").Replace("ö", "o").Replace("Ö", "O").Replace("é", "e").Replace("\uFFFD", "o");
            candidates.Add($"a_Roster_Pose_{clean}");
            candidates.Add($"a_Roster_Pose_{name.Replace(" ", "")}");
        }
        if (!string.IsNullOrEmpty(hero))
        {
            candidates.Add($"a_Roster_Pose_{hero}");
        }

        foreach (var sym in candidates.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            var svg = PreviewRenderer.RenderSymbolToSvg(uiIcons, sym, null, colorMap);
            if (svg != null) return svg;
        }
        return null;
    }

    static List<string> GetWeaponCandidateSymbols(string bw, string symbol)
    {
        var list = new List<string>();
        bool isDefaultRequest = string.IsNullOrEmpty(symbol)
            || symbol.Equals("Stock", StringComparison.OrdinalIgnoreCase)
            || symbol.Equals("Default", StringComparison.OrdinalIgnoreCase);

        if (bw.Equals("Greatsword", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponGreat_{symbol}");
                list.Add($"a_WeaponGreatsword_{symbol}");
                list.Add($"a_WeaponGreat01_{symbol}");
            }
            if (isDefaultRequest)
            {
                list.Add("a_WeaponGreat");
                list.Add("a_WeaponGreatsword");
            }
        }
        else if (bw.Equals("Hammer", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponHammer_{symbol}");
                list.Add($"a_WeaponHammer01_{symbol}");
                list.Add($"a_WeaponHammerShort_{symbol}");
                list.Add($"a_WeaponHammer1_{symbol}");
                list.Add($"a_WeaponHammer2_{symbol}");
                list.Add($"a_WeaponHammerAttack_{symbol}");
                list.Add($"a_Hammer_{symbol}");
                if (symbol.Equals("Stub", StringComparison.OrdinalIgnoreCase))
                {
                    list.Add("a_WeaponHammer_Detective");
                    list.Add("a_WeaponHammerShort_Detective");
                }
                if (symbol.StartsWith("Steampunk", StringComparison.OrdinalIgnoreCase))
                {
                    string steam = symbol.Replace("Steampunk", "SteamHammer");
                    list.Add($"a_WeaponHammer_{steam}");
                    list.Add(steam);
                }
            }
            if (isDefaultRequest)
            {
                list.Add("a_WeaponHammer");
                list.Add("a_WeaponHammer_Basic");
            }
        }
        else if (bw.Equals("Sword", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponSword_{symbol}");
                list.Add($"a_WeaponSword01_{symbol}");
                list.Add($"a_WeaponSwordAttack_{symbol}");
                list.Add($"a_WeaponSwordLand_{symbol}");
                list.Add($"a_WeaponSword1_{symbol}");
                list.Add($"a_WeaponSword2_{symbol}");
                list.Add($"a_Sword_{symbol}");
            }
            if (isDefaultRequest)
            {
                list.Add("a_WeaponSword");
                list.Add("a_WeaponSword_Basic");
            }
        }
        else if (bw.Equals("Pistol", StringComparison.OrdinalIgnoreCase) || bw.Equals("Blasters", StringComparison.OrdinalIgnoreCase) || bw.Equals("Blaster", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponPistol_{symbol}");
                list.Add($"a_WeaponPistol01_{symbol}");
            }
            if (isDefaultRequest) list.Add("a_WeaponPistol");
        }
        else if (bw.Equals("Spear", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponSpear_{symbol}");
                list.Add($"a_WeaponSpear01_{symbol}");
            }
            if (isDefaultRequest) list.Add("a_WeaponSpear");
        }
        else if (bw.Equals("RocketLance", StringComparison.OrdinalIgnoreCase) || bw.Equals("Lance", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponRocketLance_{symbol}");
                list.Add($"a_WeaponRocketLanceOpen_{symbol}");
                list.Add($"a_WeaponRocketLanceSpin1_{symbol}");
                list.Add($"a_WeaponLance_{symbol}");
            }
            if (isDefaultRequest) list.Add("a_WeaponLance");
        }
        else if (bw.Equals("Axe", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponAxe_{symbol}");
                list.Add($"a_WeaponAxe01_{symbol}");
                list.Add($"a_WeaponAxeSide_{symbol}");
            }
            if (isDefaultRequest) list.Add("a_WeaponAxe");
        }
        else if (bw.Equals("Cannon", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponCannon_{symbol}");
                list.Add($"a_WeaponCannon01_{symbol}");
            }
            if (isDefaultRequest) list.Add("a_WeaponCannon");
        }
        else if (bw.Equals("Katar", StringComparison.OrdinalIgnoreCase) || bw.Equals("Katars", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponKatarBladeTop_{symbol}");
                list.Add($"a_WeaponKatarBlade_{symbol}");
                list.Add($"a_WeaponKatar_{symbol}");
                list.Add($"a_WeaponKatarTop_{symbol}");
            }
            if (isDefaultRequest)
            {
                list.Add("a_WeaponKatarBladeTop");
                list.Add("a_WeaponKatarBlade");
                list.Add("a_WeaponKatar");
                list.Add("a_WeaponKatarTop");
            }
        }
        else if (bw.Equals("Gauntlet", StringComparison.OrdinalIgnoreCase) || bw.Equals("Gauntlets", StringComparison.OrdinalIgnoreCase) || bw.Equals("Fists", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponFists03_{symbol}");
                list.Add($"a_WeaponFists01_{symbol}");
                list.Add($"a_WeaponFists07_{symbol}");
                list.Add($"a_WeaponFists01Large_{symbol}");
                list.Add($"a_WeaponFists_{symbol}");
                list.Add($"a_WeaponGauntlet_{symbol}");
            }
            if (isDefaultRequest)
            {
                list.Add("a_WeaponFists03");
                list.Add("a_WeaponFists01");
                list.Add("a_WeaponFists07");
                list.Add("a_WeaponFists");
                list.Add("a_WeaponGauntlet");
            }
        }
        else if (bw.Equals("Chakram", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponChakramCombined_{symbol}");
                list.Add($"a_WeaponChakram_{symbol}");
                list.Add($"a_WeaponChakramRight_{symbol}");
                list.Add($"a_WeaponChakramCombinedAngleToward_{symbol}");
                list.Add($"a_WeaponChakramCombinedSpin_{symbol}");
            }
            if (isDefaultRequest)
            {
                list.Add("a_WeaponChakramCombined_ChakramBase");
                list.Add("a_WeaponChakramCombined");
                list.Add("a_WeaponChakram");
            }
        }
        else if (bw.Equals("Bow", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponBow_{symbol}");
                list.Add($"a_WeaponBow01_{symbol}");
            }
            if (isDefaultRequest) list.Add("a_WeaponBow");
        }
        else if (bw.Equals("Scythe", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponScythe_{symbol}");
                list.Add($"a_WeaponScythe01_{symbol}");
            }
            if (isDefaultRequest) list.Add("a_WeaponScythe");
        }
        else if (bw.Equals("Orb", StringComparison.OrdinalIgnoreCase))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_WeaponOrb_{symbol}");
                list.Add($"a_WeaponOrb01_{symbol}");
            }
            if (isDefaultRequest) list.Add("a_WeaponOrb");
        }
        else if (!string.IsNullOrEmpty(bw))
        {
            if (!string.IsNullOrEmpty(symbol))
            {
                list.Add($"a_Weapon{bw}_{symbol}");
            }
            if (isDefaultRequest) list.Add($"a_Weapon{bw}");
        }

        if (!string.IsNullOrEmpty(symbol))
        {
            list.Add($"a_Weapon{symbol}");
            list.Add(symbol);
        }
        return list;
    }
}
