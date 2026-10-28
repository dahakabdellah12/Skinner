using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using BrawlhallaSkinStudio.Core.Engine;
using BrawlhallaSkinStudio.Core.Models;

namespace BrawlhallaSkinStudio.Core.Services;

public class DiagnosticsReport
{
    public DateTime GeneratedAt { get; set; } = DateTime.UtcNow;
    public GameInstallation Installation { get; set; } = new();
    public int TotalLegendsDiscovered { get; set; }
    public int TotalCostumesDiscovered { get; set; }
    public int TotalWeaponSkinsDiscovered { get; set; }
    public int TotalColorSchemesDiscovered { get; set; }
    public List<string> ModifiedFilesDetected { get; set; } = [];
    public int BackupArchivesCount { get; set; }
    public bool IsSteamRunning { get; set; }
    public List<string> HealthChecks { get; set; } = [];
}

public static class DiagnosticsService
{
    public static DiagnosticsReport RunDiagnostics(
        GameInstallation installation,
        CatalogService catalog,
        BackupManager backupManager)
    {
        var report = new DiagnosticsReport
        {
            Installation = installation,
            TotalLegendsDiscovered = catalog.Legends.Count,
            TotalCostumesDiscovered = catalog.Costumes.Count,
            TotalWeaponSkinsDiscovered = catalog.WeaponSkins.Count,
            TotalColorSchemesDiscovered = catalog.ColorSchemes.Count,
            BackupArchivesCount = backupManager.Records.Count,
            IsSteamRunning = System.Diagnostics.Process.GetProcessesByName("steam").Length > 0
        };

        // File integrity checks
        if (File.Exists(installation.ExePath))
            report.HealthChecks.Add("✓ Brawlhalla.exe verified on disk.");
        else
            report.HealthChecks.Add("✗ Brawlhalla.exe is missing.");

        if (File.Exists(installation.GameSwzPath))
            report.HealthChecks.Add($"✓ Game.swz verified ({new FileInfo(installation.GameSwzPath).Length / 1024} KB).");
        else
            report.HealthChecks.Add("✗ Game.swz is missing.");

        if (File.Exists(installation.BrawlhallaAirSwfPath))
            report.HealthChecks.Add("✓ BrawlhallaAir.swf verified.");
        else
            report.HealthChecks.Add("✗ BrawlhallaAir.swf is missing.");

        // Check modified files
        foreach (var record in backupManager.Records)
        {
            foreach (var kvp in record.FileOriginalHashes)
            {
                if (backupManager.IsFileModified(kvp.Key) && !report.ModifiedFilesDetected.Contains(kvp.Key))
                {
                    report.ModifiedFilesDetected.Add(kvp.Key);
                }
            }
        }

        if (report.ModifiedFilesDetected.Count > 0)
        {
            report.HealthChecks.Add($"! {report.ModifiedFilesDetected.Count} file(s) are currently modified by Skin Studio.");
        }
        else
        {
            report.HealthChecks.Add("✓ All game assets match pristine backups (no active modifications).");
        }

        return report;
    }
}
