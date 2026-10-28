using System;
using System.IO;
using System.Linq;
using BrawlhallaSkinStudio.Core.Models;
using BrawlhallaSkinStudio.Core.Services;
using BrawlhallaSwz;
using Xunit;

namespace BrawlhallaSkinStudio.Tests;

public class CatalogTests
{
    [Fact]
    public void TestGameDetection()
    {
        var installation = GameDetector.DetectGameInstallation();
        Assert.NotNull(installation);
        Assert.True(installation.IsValid, string.Join("; ", installation.ValidationMessages));
        Assert.True(File.Exists(installation.ExePath));
        Assert.True(File.Exists(installation.GameSwzPath));
        Assert.True(installation.DiscoveredSwfCount > 100);
        Console.WriteLine($"Detected game at: {installation.GamePath}, SWFs: {installation.DiscoveredSwfCount}, Build: {installation.BuildId}");
    }

    [Fact]
    public void TestCatalogLoading()
    {
        var installation = GameDetector.DetectGameInstallation();
        Assert.True(installation.IsValid);

        var catalog = new CatalogService();
        bool loaded = catalog.LoadFromInstallation(installation);
        Assert.True(loaded, catalog.LoadError);

        Assert.NotEmpty(catalog.Legends);
        Assert.Equal(70, catalog.Legends.Count);
        Assert.Equal("Viking", catalog.Legends[0].HeroName);

        Assert.NotEmpty(catalog.Costumes);
        Assert.True(catalog.Costumes.Count >= 500, $"Expected >= 500 costumes, got {catalog.Costumes.Count}");

        Assert.NotEmpty(catalog.WeaponSkins);
        Assert.True(catalog.WeaponSkins.Count >= 500, $"Expected >= 500 weapon skins, got {catalog.WeaponSkins.Count}");

        Assert.NotEmpty(catalog.ColorSchemes);
        Assert.True(catalog.ColorSchemes.Count >= 70, $"Expected >= 70 color schemes, got {catalog.ColorSchemes.Count}");

        // Verify specific known legend: Viking (Bödvar)
        var bodvar = catalog.Legends.FirstOrDefault(l => l.HeroName.Equals("Viking", StringComparison.OrdinalIgnoreCase));
        Assert.NotNull(bodvar);
        Assert.Equal("Hammer", bodvar.Weapon1);
        Assert.Equal("Sword", bodvar.Weapon2);
        Assert.NotEmpty(bodvar.Costumes);

        string uiIcons = Path.Combine(installation.GamePath, "UI_Icons.swf");
        using var fs = File.OpenRead(uiIcons);
        var swf = SwfLib.SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SwfLib.Tags.ControlTags.SymbolClassTag>().First();
        var allSymbolNames = new HashSet<string>(sym.References.Select(r => r.SymbolName));

        var overrides = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["Lord Vraxx"] = "a_Roster_Pose_Vraxx",
            ["Alien"] = "a_Roster_Pose_Vraxx",
            ["Gnash"] = "a_Roster_Pose_Thrulk",
            ["Caveman"] = "a_Roster_Pose_Thrulk",
            ["Sir Roland"] = "a_Roster_Pose_Hotspur",
            ["Knight"] = "a_Roster_Pose_Hotspur",
            ["Scarlet"] = "a_Roster_Pose_Roboengineer",
            ["Steampunk"] = "a_Roster_Pose_Roboengineer",
            ["Brynn"] = "a_Roster_Pose_Valkyrie",
            ["ActualValk"] = "a_Roster_Pose_Valkyrie",
            ["Diana"] = "a_Roster_Pose_MonsterHunter",
            ["Marksman"] = "a_Roster_Pose_MonsterHunter",
            ["Mirage"] = "a_Roster_Pose_EgyptianShoujo",
            ["Egyptian"] = "a_Roster_Pose_EgyptianShoujo",
            ["Yumiko"] = "a_Roster_Pose_Kitsune",
            ["Ninetails"] = "a_Roster_Pose_Kitsune",
            ["Artemis"] = "a_Roster_Pose_SpaceHunter",
            ["Spacehunter"] = "a_Roster_Pose_SpaceHunter",
            ["Petra"] = "a_Roster_Pose_Ragefighter",
            ["RageFighter"] = "a_Roster_Pose_Ragefighter",
            ["Thea"] = "a_Roster_Pose_Speedster",
            ["GreekSpeedster"] = "a_Roster_Pose_Speedster",
            ["Ransom"] = "a_Roster_Pose_Cybervirus",
            ["CyberVirus"] = "a_Roster_Pose_Cybervirus",
            ["Aurus"] = "a_Roster_Pose_Gladiator",
            ["ActualGladiator"] = "a_Roster_Pose_Gladiator"
        };

        var missing = new List<string>();
        foreach (var leg in catalog.Legends)
        {
            if (overrides.TryGetValue(leg.DisplayName, out var pose) || overrides.TryGetValue(leg.HeroName, out pose))
            {
                Assert.True(allSymbolNames.Contains(pose), $"Override for {leg.DisplayName} ({pose}) not found!");
                continue;
            }

            string clean = leg.DisplayName.Replace(" ", "").Replace("ö", "o").Replace("é", "e");
            var candidates = new[]
            {
                $"a_Roster_Pose_{clean}",
                $"a_Roster_Pose_{leg.HeroName}",
                $"a_Roster_Pose_{leg.DisplayName.Replace(" ", "")}",
                leg.PoseSymbol
            };

            var match = candidates.FirstOrDefault(c => allSymbolNames.Contains(c));
            if (match == null)
            {
                missing.Add($"{leg.DisplayName} (Hero: {leg.HeroName})");
            }
        }

        Assert.Empty(missing);

        // Verify specific known weapon: HammerViking
        var hammer = catalog.WeaponSkins.FirstOrDefault(w => w.WeaponSkinName.Equals("HammerViking", StringComparison.OrdinalIgnoreCase));
        Assert.NotNull(hammer);
        Assert.Equal("Hammer", hammer.BaseWeapon);
    }
}
