using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using BrawlhallaSkinStudio.Core.Engine;
using BrawlhallaSkinStudio.Core.Models;
using BrawlhallaSkinStudio.Core.Services;
using SwfLib;
using SwfLib.Tags.ControlTags;
using Xunit;

namespace BrawlhallaSkinStudio.Tests;

public class ModificationTests : IDisposable
{
    private readonly string _testDir;
    private readonly string _realGameDir;
    private readonly BackupManager _backupManager;
    private readonly ModificationEngine _engine;

    public ModificationTests()
    {
        _testDir = Path.Combine(Path.GetTempPath(), "BrawlhallaTest_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(_testDir);

        var installation = GameDetector.DetectGameInstallation();
        _realGameDir = installation.GamePath;

        // Copy required test fixtures (Gfx_Viking.swf, Gfx_Cowgirl.swf, Gfx_Gargoyle.swf, Gfx_Player_02.swf)
        File.Copy(Path.Combine(_realGameDir, "Gfx_Viking.swf"), Path.Combine(_testDir, "Gfx_Viking.swf"));
        File.Copy(Path.Combine(_realGameDir, "Gfx_Cowgirl.swf"), Path.Combine(_testDir, "Gfx_Cowgirl.swf"));
        if (File.Exists(Path.Combine(_realGameDir, "Gfx_Gargoyle.swf")))
            File.Copy(Path.Combine(_realGameDir, "Gfx_Gargoyle.swf"), Path.Combine(_testDir, "Gfx_Gargoyle.swf"));
        if (File.Exists(Path.Combine(_realGameDir, "Gfx_Player_02.swf")))
            File.Copy(Path.Combine(_realGameDir, "Gfx_Player_02.swf"), Path.Combine(_testDir, "Gfx_Player_02.swf"));

        _backupManager = new BackupManager(_testDir);
        _engine = new ModificationEngine(_testDir, _backupManager);
    }

    public void Dispose()
    {
        try
        {
            if (Directory.Exists(_testDir))
            {
                Directory.Delete(_testDir, recursive: true);
            }
        }
        catch { }
    }

    [Fact]
    public void TestBackupAndRestoreCycle()
    {
        string targetFile = Path.Combine(_testDir, "Gfx_Viking.swf");
        string originalHash = BackupManager.ComputeSha256(targetFile);

        // Create backup
        string backupId = _backupManager.CreateBackup([targetFile], "Unit test backup");
        Assert.False(string.IsNullOrEmpty(backupId));

        // Mutate target file
        File.AppendAllText(targetFile, "MODIFIED_APPENDED_BYTES");
        string mutatedHash = BackupManager.ComputeSha256(targetFile);
        Assert.NotEqual(originalHash, mutatedHash);
        Assert.True(_backupManager.IsFileModified("Gfx_Viking.swf"));

        // Restore file
        bool restored = _backupManager.RestoreFile("Gfx_Viking.swf");
        Assert.True(restored);

        string restoredHash = BackupManager.ComputeSha256(targetFile);
        Assert.Equal(originalHash, restoredHash);
        Assert.False(_backupManager.IsFileModified("Gfx_Viking.swf"));
    }

    [Fact]
    public void TestIntraSwfCostumeModification()
    {
        string targetFile = Path.Combine(_testDir, "Gfx_Viking.swf");
        string originalHash = BackupManager.ComputeSha256(targetFile);

        var request = new ModificationRequest
        {
            TargetLegend = new LegendInfo { HeroName = "Viking", DisplayName = "Bödvar" },
            TargetCostume = new CostumeInfo
            {
                CostumeName = "Viking",
                DisplayName = "Classic Bödvar",
                SwfFile = "Gfx_Viking.swf",
                SymbolSuffix = "Viking"
            },
            ReplacementCostume = new CostumeInfo
            {
                CostumeName = "Bear",
                DisplayName = "Beardvar",
                SwfFile = "Gfx_Viking.swf",
                SymbolSuffix = "Bear"
            }
        };

        var report = _engine.ValidateModification(request);
        Assert.True(report.IsCompatible);
        Assert.Equal(CompatibilityLevel.Compatible, report.Level);

        var result = _engine.ApplyModification(request);
        Assert.True(result.Success, result.Message);
        Assert.Contains("Gfx_Viking.swf", result.ModifiedFiles);

        string modifiedHash = BackupManager.ComputeSha256(targetFile);
        Assert.NotEqual(originalHash, modifiedHash);

        // Verify that target symbol references now point to Bear symbols
        using (var fs = File.OpenRead(targetFile))
        {
            var swf = SwfFile.ReadFrom(fs);
            var symClass = swf.Tags.OfType<SymbolClassTag>().First();

            var torsoViking = symClass.References.First(r => r.SymbolName == "a_Torso1_Viking");
            var torsoBear = symClass.References.First(r => r.SymbolName == "a_Torso1_Bear");

            // Torso of Viking should now point to Torso of Bear's character ID!
            Assert.Equal(torsoBear.SymbolID, torsoViking.SymbolID);
        }

        // Test restoring original
        int restoredCount = _backupManager.RestoreAll();
        Assert.True(restoredCount >= 1);
        Assert.Equal(originalHash, BackupManager.ComputeSha256(targetFile));
    }

    [Fact]
    public void TestCrossSwfWeaponReplacement()
    {
        string vikingFile = Path.Combine(_testDir, "Gfx_Viking.swf");
        string originalVikingHash = BackupManager.ComputeSha256(vikingFile);

        var request = new ModificationRequest
        {
            TargetLegend = new LegendInfo { HeroName = "Viking", DisplayName = "Bödvar" },
            TargetCostume = new CostumeInfo { CostumeName = "Viking", SwfFile = "Gfx_Viking.swf", SymbolSuffix = "Viking" },
            TargetWeapon1 = new WeaponSkinInfo
            {
                WeaponSkinName = "HammerViking",
                BaseWeapon = "Hammer",
                DisplayName = "Bödvar's Hammer",
                SwfFile = "Gfx_Viking.swf",
                SymbolSuffix = "VikingHammer1"
            },
            ReplacementWeapon1 = new WeaponSkinInfo
            {
                WeaponSkinName = "HammerCowgirl",
                BaseWeapon = "Hammer",
                DisplayName = "Cassidy's Hammer",
                SwfFile = "Gfx_Cowgirl.swf",
                SymbolSuffix = "Cowgirl01"
            }
        };

        var report = _engine.ValidateModification(request);
        Assert.True(report.IsCompatible);

        var result = _engine.ApplyModification(request);
        Assert.True(result.Success, result.Message);

        string modifiedHash = BackupManager.ComputeSha256(vikingFile);
        Assert.NotEqual(originalVikingHash, modifiedHash);

        // Verify that target hammer in Gfx_Viking.swf now resolves cleanly
        using (var fs = File.OpenRead(vikingFile))
        {
            var swf = SwfFile.ReadFrom(fs);
            var symClass = swf.Tags.OfType<SymbolClassTag>().First();
            var hammerRef = symClass.References.FirstOrDefault(r => r.SymbolName == "a_WeaponHammer_VikingHammer1");
            Assert.NotNull(hammerRef);

            // Sprite must exist in tags
            var sprite = swf.Tags.OfType<SwfLib.Tags.DefineSpriteTag>().FirstOrDefault(s => s.SpriteID == hammerRef.SymbolID);
            Assert.NotNull(sprite);
        }

        // Restore
        _backupManager.RestoreAll();
        Assert.Equal(originalVikingHash, BackupManager.ComputeSha256(vikingFile));
    }

    [Fact]
    public void TestCrossLegendCostumeConversion()
    {
        string gargoyleFile = Path.Combine(_testDir, "Gfx_Gargoyle.swf");
        if (!File.Exists(gargoyleFile)) return;

        string originalHash = BackupManager.ComputeSha256(gargoyleFile);

        var request = new ModificationRequest
        {
            TargetLegend = new LegendInfo { HeroName = "Gargoyle", DisplayName = "Onyx" },
            TargetCostume = new CostumeInfo
            {
                CostumeName = "Gargoyle",
                DisplayName = "Classic Onyx",
                SwfFile = "Gfx_Gargoyle.swf",
                SymbolSuffix = "Gargoyle"
            },
            ReplacementCostume = new CostumeInfo
            {
                CostumeName = "Skeleton",
                DisplayName = "Azoth",
                SwfFile = "Gfx_Player_02.swf",
                SymbolSuffix = "Skeleton"
            }
        };

        var report = _engine.ValidateModification(request);
        Assert.True(report.IsCompatible);
        Assert.Equal(CompatibilityLevel.CrossLegendExperimental, report.Level);
        Assert.True(report.RequiresCrossSwfImport);

        var result = _engine.ApplyModification(request);
        Assert.True(result.Success, result.Message);

        string modifiedHash = BackupManager.ComputeSha256(gargoyleFile);
        Assert.NotEqual(originalHash, modifiedHash);

        // Verify that target symbol references exist in modified SWF
        using (var fs = File.OpenRead(gargoyleFile))
        {
            var swf = SwfFile.ReadFrom(fs);
            var symClass = swf.Tags.OfType<SymbolClassTag>().First();
            Assert.Contains(symClass.References, r => r.SymbolName.EndsWith("_Gargoyle"));
        }

        // Restore
        _backupManager.RestoreAll();
        Assert.Equal(originalHash, BackupManager.ComputeSha256(gargoyleFile));
    }

    [Fact]
    public void TestPreviewRendererGeneratesValidSvg()
    {
        string vikingSwf = Path.Combine(_testDir, "Gfx_Viking.swf");
        string? svg = PreviewRenderer.RenderSymbolToSvg(vikingSwf, "a_WeaponHammer_VikingHammer1", _testDir);

        Assert.NotNull(svg);
        Assert.Contains("<svg", svg);
        Assert.Contains("viewBox", svg);
    }

    [Fact]
    public void TestIncompatibleWeaponTypeRejection()
    {
        var request = new ModificationRequest
        {
            TargetLegend = new LegendInfo { HeroName = "Viking" },
            TargetCostume = new CostumeInfo { CostumeName = "Viking", SwfFile = "Gfx_Viking.swf", SymbolSuffix = "Viking" },
            TargetWeapon1 = new WeaponSkinInfo
            {
                WeaponSkinName = "HammerViking",
                BaseWeapon = "Hammer",
                SwfFile = "Gfx_Viking.swf",
                SymbolSuffix = "VikingHammer1"
            },
            ReplacementWeapon1 = new WeaponSkinInfo
            {
                WeaponSkinName = "BowDiana",
                BaseWeapon = "Bow",
                SwfFile = "Gfx_Diana.swf",
                SymbolSuffix = "Diana01"
            }
        };

        var report = _engine.ValidateModification(request);
        Assert.False(report.IsCompatible);
        Assert.Equal(CompatibilityLevel.Incompatible, report.Level);
        Assert.Contains(report.Diagnostics, d => d.Contains("mismatch"));
    }

    [Fact]
    public void TestPresetManagerOperations()
    {
        string presetDir = Path.Combine(_testDir, "presets");
        var manager = new PresetManager(presetDir);

        var preset = new Preset
        {
            Name = "Beardvar Heavy",
            Description = "Custom loadout",
            LegendName = "Viking",
            TargetCostumeName = "Viking",
            ReplacementCostumeName = "Bear",
            Weapon1Type = "Hammer",
            Weapon1SkinName = "HammerCowgirl",
            Weapon2Type = "Sword",
            Weapon2SkinName = "Bodvar01",
            ColorSchemeName = "Red"
        };

        manager.SavePreset(preset);
        Assert.Single(manager.Presets);

        // Duplicate
        var duplicated = manager.DuplicatePreset(preset.Id, "Beardvar Heavy Copy");
        Assert.Equal(2, manager.Presets.Count);
        Assert.Equal("Beardvar Heavy Copy", duplicated.Name);

        // Export & Import
        string json = manager.ExportPresetJson(preset.Id);
        Assert.Contains("Beardvar Heavy", json);

        var imported = manager.ImportPresetJson(json);
        Assert.Equal(3, manager.Presets.Count);

        // Delete
        bool deleted = manager.DeletePreset(duplicated.Id);
        Assert.True(deleted);
        Assert.Equal(2, manager.Presets.Count);
    }

    [Fact]
    public void TestDarthVaderSwordAudioModificationAllowedOnlyOnTargetSwords()
    {
        string audioDir = Path.Combine(_testDir, "audio", "pc");
        Directory.CreateDirectory(audioDir);

        string srcSwordBnk = Path.Combine(_realGameDir, "audio", "pc", "WEP_Sword.bnk");
        string srcSaber01Bnk = Path.Combine(_realGameDir, "audio", "pc", "WEP_Saber_01.bnk");
        string srcSaber06Bnk = Path.Combine(_realGameDir, "audio", "pc", "WEP_Saber_06.bnk");
        string testSwordBnk = Path.Combine(audioDir, "WEP_Sword.bnk");
        string testSaber01Bnk = Path.Combine(audioDir, "WEP_Saber_01.bnk");
        string testSaber06Bnk = Path.Combine(audioDir, "WEP_Saber_06.bnk");

        File.Copy(srcSwordBnk, testSwordBnk);
        if (File.Exists(srcSaber01Bnk)) File.Copy(srcSaber01Bnk, testSaber01Bnk);
        if (File.Exists(srcSaber06Bnk)) File.Copy(srcSaber06Bnk, testSaber06Bnk);

        string srcVaderSwf = Path.Combine(_realGameDir, "Gfx_DarthVader_Sword.swf");
        if (File.Exists(srcVaderSwf))
        {
            File.Copy(srcVaderSwf, Path.Combine(_testDir, "Gfx_DarthVader_Sword.swf"));
        }
        string srcWeaponsSwf = Path.Combine(_realGameDir, "Gfx_Weapons.swf");
        if (File.Exists(srcWeaponsSwf))
        {
            File.Copy(srcWeaponsSwf, Path.Combine(_testDir, "Gfx_Weapons.swf"));
        }
        string srcEzioSwf = Path.Combine(_realGameDir, "Gfx_Ezio.swf");
        if (File.Exists(srcEzioSwf))
        {
            File.Copy(srcEzioSwf, Path.Combine(_testDir, "Gfx_Ezio.swf"));
        }
        string srcAsgardSwf = Path.Combine(_realGameDir, "Gfx_AsgardSabers.swf");
        if (File.Exists(srcAsgardSwf))
        {
            File.Copy(srcAsgardSwf, Path.Combine(_testDir, "Gfx_AsgardSabers.swf"));
        }

        var inst = GameDetector.DetectGameInstallation();
        var catalog = new CatalogService();
        catalog.LoadFromInstallation(inst!);

        var engine = new ModificationEngine(_testDir, _backupManager, catalog);

        var vaderSword = catalog.WeaponSkins.First(w => w.WeaponSkinName.Equals("SwordDarthVader", StringComparison.OrdinalIgnoreCase));
        
        // 1. Regular sword (Bödvar default sword) -> Base sound bank WEP_Sword.bnk must NEVER be modified!
        var bodvarSword = catalog.WeaponSkins.First(w => w.WeaponSkinName.Equals("SwordViking", StringComparison.OrdinalIgnoreCase));
        var viking = catalog.Legends.First(l => l.HeroName.Equals("Viking", StringComparison.OrdinalIgnoreCase));
        var vikingCostume = viking.Costumes.First(c => c.IsDefault);

        var reqNormal = new ModificationRequest
        {
            TargetLegend = viking,
            TargetCostume = vikingCostume,
            TargetWeapon1 = bodvarSword,
            ReplacementWeapon1 = vaderSword
        };

        var resultNormal = engine.ApplyModification(reqNormal);
        Assert.True(resultNormal.Success, string.Join("; ", resultNormal.OperationLog));
        // Verify audio was NOT modified (normal swords must remain 100% untouched)
        Assert.DoesNotContain(resultNormal.ModifiedFiles, f => f.EndsWith(".bnk", StringComparison.OrdinalIgnoreCase));

        // 2. Ezio sword (SwordEzio / Brotherhood Sword) -> Allowed target sword, audio IS modified!
        var ezioSword = catalog.WeaponSkins.First(w => w.WeaponSkinName.Equals("SwordEzio", StringComparison.OrdinalIgnoreCase));
        var ezioLegend = catalog.Legends.First(l => l.HeroName.Equals("Ezio", StringComparison.OrdinalIgnoreCase));
        var ezioCostume = ezioLegend.Costumes.First(c => c.IsDefault);

        var reqEzio = new ModificationRequest
        {
            TargetLegend = ezioLegend,
            TargetCostume = ezioCostume,
            TargetWeapon1 = ezioSword,
            ReplacementWeapon1 = vaderSword
        };

        var resultEzio = engine.ApplyModification(reqEzio);
        Assert.True(resultEzio.Success, string.Join("; ", resultEzio.OperationLog));
        Assert.Contains(resultEzio.ModifiedFiles, f => f.EndsWith("WEP_Sword.bnk", StringComparison.OrdinalIgnoreCase));

        // 3. Switching back to normal sword -> Base sound bank WEP_Sword.bnk is automatically restored to clean state!
        var resultRestore = engine.ApplyModification(reqNormal);
        Assert.True(resultRestore.Success, string.Join("; ", resultRestore.OperationLog));
        Assert.False(_backupManager.IsFileModified("audio/pc/WEP_Sword.bnk"), "WEP_Sword.bnk should be restored when normal sword is applied!");

        // 4. Dedicated saber sword (SwordAsgardSaber) -> Dedicated bank WEP_Saber_01.bnk is modified, WEP_Sword.bnk untouched!
        var asgardSword = catalog.WeaponSkins.First(w => w.WeaponSkinName.Equals("SwordAsgardSaber", StringComparison.OrdinalIgnoreCase));
        var reqAsgard = new ModificationRequest
        {
            TargetLegend = viking,
            TargetCostume = vikingCostume,
            TargetWeapon1 = asgardSword,
            ReplacementWeapon1 = vaderSword
        };

        var resultAsgard = engine.ApplyModification(reqAsgard);
        Assert.True(resultAsgard.Success, string.Join("; ", resultAsgard.OperationLog));
        Assert.Contains(resultAsgard.ModifiedFiles, f => f.EndsWith("WEP_Saber_01.bnk", StringComparison.OrdinalIgnoreCase));
        Assert.DoesNotContain(resultAsgard.ModifiedFiles, f => f.EndsWith("WEP_Sword.bnk", StringComparison.OrdinalIgnoreCase));

        // Verify that WEP_Saber_01.bnk now has WEP_Saber_01 bank ID
        byte[] bytes = File.ReadAllBytes(testSaber01Bnk);
        Assert.Equal("BKHD", System.Text.Encoding.ASCII.GetString(bytes, 0, 4));
        uint bankId = BitConverter.ToUInt32(bytes, 12);
        Assert.Equal(ModificationEngine.FnvHash("WEP_Saber_01"), bankId);
    }
}
