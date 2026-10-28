using System;
using System.Collections.Generic;

namespace BrawlhallaSkinStudio.Core.Models;

public class GameInstallation
{
    public string GamePath { get; set; } = string.Empty;
    public string ExePath { get; set; } = string.Empty;
    public string GameSwzPath { get; set; } = string.Empty;
    public string BrawlhallaAirSwfPath { get; set; } = string.Empty;
    public string Version { get; set; } = "Unknown";
    public string BuildId { get; set; } = "Unknown";
    public int DiscoveredSwfCount { get; set; }
    public bool IsValid { get; set; }
    public List<string> ValidationMessages { get; set; } = [];
    public DateTime DetectedAt { get; set; } = DateTime.UtcNow;
}

public class LegendInfo
{
    public int HeroId { get; set; }
    public string HeroName { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Bio { get; set; } = string.Empty;
    public string Weapon1 { get; set; } = string.Empty;
    public string Weapon2 { get; set; } = string.Empty;
    public string IconName { get; set; } = string.Empty;
    public string PoseSymbol { get; set; } = string.Empty;
    public string PrimarySwf { get; set; } = string.Empty;
    public List<CostumeInfo> Costumes { get; set; } = [];

    public override string ToString() => $"{DisplayName} ({HeroName}) - [{Weapon1}, {Weapon2}]";
}

public class CostumeInfo
{
    public int CostumeId { get; set; }
    public string CostumeName { get; set; } = string.Empty;
    public string OwnerHero { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public bool IsDefault { get; set; }
    public string SwfFile { get; set; } = string.Empty;
    public string SymbolSuffix { get; set; } = string.Empty;
    public string WeaponSet { get; set; } = string.Empty;
    public string IconName { get; set; } = string.Empty;
    public string IconFileName { get; set; } = string.Empty;
    public bool IsCrossover { get; set; }
    public bool IsMetadev { get; set; }
    public Dictionary<string, uint> BaseColors { get; set; } = [];

    public override string ToString() => $"{DisplayName} [{CostumeName}] {(IsDefault ? "(Default)" : "")}";
}

public class WeaponSkinInfo
{
    public int WeaponSkinId { get; set; }
    public string WeaponSkinName { get; set; } = string.Empty;
    public string BaseWeapon { get; set; } = string.Empty;
    public string OwnerHero { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string SwfFile { get; set; } = string.Empty;
    public string SymbolSuffix { get; set; } = string.Empty;
    public string WeaponIcon { get; set; } = string.Empty;
    public bool IsDefault { get; set; }
    public Dictionary<string, uint> BaseColors { get; set; } = [];

    public override string ToString() => $"{DisplayName} ({BaseWeapon})";
}

public class ColorSchemeInfo
{
    public int SchemeId { get; set; }
    public string SchemeName { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Rarity { get; set; } = string.Empty;
    public uint IndicatorColor { get; set; }
    public Dictionary<string, uint> SwapColors { get; set; } = [];

    public override string ToString() => DisplayName;
}

public class ModificationRequest
{
    public LegendInfo TargetLegend { get; set; } = null!;
    public CostumeInfo TargetCostume { get; set; } = null!;
    public CostumeInfo? ReplacementCostume { get; set; }
    public WeaponSkinInfo? TargetWeapon1 { get; set; }
    public WeaponSkinInfo? ReplacementWeapon1 { get; set; }
    public WeaponSkinInfo? TargetWeapon2 { get; set; }
    public WeaponSkinInfo? ReplacementWeapon2 { get; set; }
    public ColorSchemeInfo? SelectedColorScheme { get; set; }
    public Dictionary<string, uint>? CustomPalette { get; set; }
}

public class CompatibilityReport
{
    public bool IsCompatible { get; set; } = true;
    public CompatibilityLevel Level { get; set; } = CompatibilityLevel.Compatible;
    public List<string> Diagnostics { get; set; } = [];
    public List<string> MappedSymbols { get; set; } = [];
    public List<string> MissingSymbols { get; set; } = [];
    public bool RequiresCrossSwfImport { get; set; }
}

public enum CompatibilityLevel
{
    Compatible,
    CompatibleWithMinorOmissions,
    CrossLegendExperimental,
    Incompatible
}

public class ModificationResult
{
    public bool Success { get; set; }
    public string Message { get; set; } = string.Empty;
    public List<string> ModifiedFiles { get; set; } = [];
    public string BackupId { get; set; } = string.Empty;
    public Dictionary<string, string> HashesBefore { get; set; } = [];
    public Dictionary<string, string> HashesAfter { get; set; } = [];
    public List<string> OperationLog { get; set; } = [];
    public DateTime AppliedAt { get; set; } = DateTime.UtcNow;
}

public class Preset
{
    public string Id { get; set; } = Guid.NewGuid().ToString("N");
    public string Name { get; set; } = "New Preset";
    public string Description { get; set; } = string.Empty;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public string LegendName { get; set; } = string.Empty;
    public string TargetCostumeName { get; set; } = string.Empty;
    public string ReplacementCostumeName { get; set; } = string.Empty;
    public string Weapon1Type { get; set; } = string.Empty;
    public string Weapon1SkinName { get; set; } = string.Empty;
    public string Weapon2Type { get; set; } = string.Empty;
    public string Weapon2SkinName { get; set; } = string.Empty;
    public string ColorSchemeName { get; set; } = string.Empty;
    public Dictionary<string, uint> CustomPalette { get; set; } = [];
}

public class BackupRecord
{
    public string BackupId { get; set; } = string.Empty;
    public DateTime Timestamp { get; set; } = DateTime.UtcNow;
    public string Reason { get; set; } = string.Empty;
    public Dictionary<string, string> FileOriginalHashes { get; set; } = [];
    public Dictionary<string, string> BackupFilesRelativePath { get; set; } = [];
}

public class KoEffectInfo
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string AnimFile { get; set; } = string.Empty;
    public string SoundEvent { get; set; } = string.Empty;
    public bool IsDefault { get; set; }
}

public class SidekickInfo
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string AnimFile { get; set; } = string.Empty;
    public string CustomArt { get; set; } = string.Empty;
    public bool IsDefault { get; set; }
}

public class PodiumInfo
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string AnimFile { get; set; } = string.Empty;
    public string CustomArt { get; set; } = string.Empty;
    public string SoundReady { get; set; } = string.Empty;
    public bool IsDefault { get; set; }
}

public class TrailEffectInfo
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string AnimFile { get; set; } = string.Empty;
    public bool IsDefault { get; set; }
}

public class CrossoverSfxInfo
{
    public string SfxFile { get; set; } = string.Empty;
    public string CharacterName { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
}

public class AdvancedModificationRequest
{
    public KoEffectInfo? TargetKoEffect { get; set; }
    public KoEffectInfo? ReplacementKoEffect { get; set; }

    public SidekickInfo? TargetSidekick { get; set; }
    public SidekickInfo? ReplacementSidekick { get; set; }

    public PodiumInfo? TargetPodium { get; set; }
    public PodiumInfo? ReplacementPodium { get; set; }

    public TrailEffectInfo? TargetTrail { get; set; }
    public TrailEffectInfo? ReplacementTrail { get; set; }

    public bool SyncCrossoverSfx { get; set; } = true;
    public string? SelectedCrossoverSfx { get; set; }
    public bool AutoRestart { get; set; } = true;
}

public class AdvancedModificationResult
{
    public bool Success { get; set; }
    public string Message { get; set; } = string.Empty;
    public List<string> ModifiedFiles { get; set; } = [];
    public List<string> OperationLog { get; set; } = [];
    public string BackupId { get; set; } = string.Empty;
}
