using System;
using System.IO;
using System.Linq;
using System.Diagnostics;
using System.Collections.Generic;
using BrawlhallaSkinStudio.Core.Services;
using BrawlhallaSkinStudio.Core.Engine;
using BrawlhallaSkinStudio.Core.Models;
using BrawlhallaSwz;
using Xunit;
using Xunit.Abstractions;

namespace BrawlhallaSkinStudio.Tests;

public class LiveGameTests
{
    private readonly ITestOutputHelper _output;

    public LiveGameTests(ITestOutputHelper output)
    {
        _output = output;
    }
    [Fact]
    public void TestLiveGameFileAccess()
    {
        var inst = GameDetector.DetectGameInstallation();
        Assert.True(inst.IsValid);

        // Verify Game.swz can be read
        using (var fs = new FileStream(inst.GameSwzPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            Assert.True(fs.Length > 0);
        }

        // Verify loose SWF can be read and written with FileShare.ReadWrite
        string testSwf = Path.Combine(inst.GamePath, "Gfx_Viking.swf");
        Assert.True(File.Exists(testSwf));

        using (var fs = new FileStream(testSwf, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            Assert.True(fs.Length > 0);
        }
    }

    [Fact]
    public void TestLiveGameModificationCycle()
    {
        var inst = GameDetector.DetectGameInstallation();
        Assert.True(inst.IsValid);

        string testSwf = Path.Combine(inst.GamePath, "Gfx_Viking.swf");
        Assert.True(File.Exists(testSwf));

        byte[] originalBytes = File.ReadAllBytes(testSwf);
        string originalHash = BackupManager.ComputeSha256(testSwf);

        // Test File.Copy overwrite
        string tmpCopy = testSwf + ".test_tmp";
        File.WriteAllBytes(tmpCopy, originalBytes);
        File.Copy(tmpCopy, testSwf, overwrite: true);
        File.Delete(tmpCopy);

        // Test File.Move overwrite
        string tmpMove = testSwf + ".test_tmp2";
        File.WriteAllBytes(tmpMove, originalBytes);
        File.Move(tmpMove, testSwf, overwrite: true);

        // Verify hash remains identical
        string finalHash = BackupManager.ComputeSha256(testSwf);
        Assert.Equal(originalHash, finalHash);
    }

    [Fact]
    public void TestLiveGameModificationEngineCycle()
    {
        var inst = GameDetector.DetectGameInstallation();
        Assert.True(inst.IsValid);

        var backupMgr = new BackupManager(inst.GamePath);
        var modEngine = new ModificationEngine(inst.GamePath, backupMgr);
        var catalog = new CatalogService();
        catalog.LoadFromInstallation(inst);

        var bodvar = catalog.Legends.Find(l => l.DisplayName == "Bödvar");
        Assert.NotNull(bodvar);

        var costumes = catalog.Costumes.FindAll(c => c.OwnerHero == bodvar.HeroName);
        var targetCostume = costumes.Find(c => c.IsDefault) ?? costumes[0];
        var replCostume = costumes.Find(c => !c.IsDefault);
        Assert.NotNull(replCostume);

        var req = new ModificationRequest
        {
            TargetLegend = bodvar,
            TargetCostume = targetCostume,
            ReplacementCostume = replCostume
        };

        // 1. Validate
        var comp = modEngine.ValidateModification(req);
        Assert.True(comp.IsCompatible);

        // 2. Apply while live game is running
        var result = modEngine.ApplyModification(req);
        Assert.True(result.Success, result.Message);

        // 3. Verify backup was created
        Assert.True(backupMgr.IsFileModified(targetCostume.SwfFile));

        // 4. Restore original safely
        bool restored = backupMgr.RestoreFile(targetCostume.SwfFile);
        Assert.True(restored);
        Assert.False(backupMgr.IsFileModified(targetCostume.SwfFile));
    }

    [Fact]
    public void TestInspectAllSwfFilePaths()
    {
        var inst = GameDetector.DetectGameInstallation();
        Assert.True(inst.IsValid);

        var catalog = new CatalogService();
        catalog.LoadFromInstallation(inst);

        var badWeapons = new List<string>();
        foreach (var w in catalog.WeaponSkins)
        {
            string path = Path.Combine(inst.GamePath, w.SwfFile);
            if (!File.Exists(path))
            {
                badWeapons.Add($"{w.WeaponSkinName} -> SwfFile: '{w.SwfFile}'");
            }
        }

        var badCostumes = new List<string>();
        foreach (var c in catalog.Costumes)
        {
            string path = Path.Combine(inst.GamePath, c.SwfFile);
            if (!File.Exists(path))
            {
                badCostumes.Add($"{c.CostumeName} -> SwfFile: '{c.SwfFile}'");
            }
        }

        Assert.True(badWeapons.Count == 0, $"Bad weapons ({badWeapons.Count}):\n" + string.Join("\n", badWeapons.Take(25)));
        Assert.True(badCostumes.Count == 0, $"Bad costumes ({badCostumes.Count}):\n" + string.Join("\n", badCostumes.Take(25)));
    }
}
