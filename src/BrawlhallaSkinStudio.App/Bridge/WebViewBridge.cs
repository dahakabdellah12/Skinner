using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using BrawlhallaSkinStudio.Core.Engine;
using BrawlhallaSkinStudio.Core.Models;
using BrawlhallaSkinStudio.Core.Services;
using Microsoft.Web.WebView2.Core;

namespace BrawlhallaSkinStudio.App.Bridge;

public class WebViewBridge
{
    private static readonly Regex RepoPattern = new(@"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$", RegexOptions.Compiled);
    private static readonly Regex VersionPattern = new(@"^\d+(\.\d+){0,3}$", RegexOptions.Compiled);
    private static readonly Regex Sha256Pattern = new(@"^[a-fA-F0-9]{64}$", RegexOptions.Compiled);

    private const long MaxUpdateArchiveBytes = 512L * 1024 * 1024;
    private const int MaxUpdateArchiveEntries = 20000;

    private static readonly string CurrentVersion = ResolveCurrentVersion();

    private static string ResolveCurrentVersion()
    {
        var version = Assembly.GetExecutingAssembly().GetName().Version;
        return version == null ? "3.1.2" : $"{version.Major}.{version.Minor}.{version.Build}";
    }

    private readonly CoreWebView2 _webView;
    private GameInstallation _installation;
    private CatalogService _catalog;
    private BackupManager _backupManager;
    private ModificationEngine _modEngine;
    private PresetManager _presetManager;
    private readonly Action? _onMinimize;
    private readonly Func<bool>? _onMaximize;
    private readonly Action? _onClose;
    private readonly Action? _onDrag;
    private readonly Func<bool>? _isWindowMaximized;

    // Update download state controls
    private static CancellationTokenSource? _updateCts;
    private static ManualResetEventSlim _updatePauseEvent = new(true);
    private static bool _isUpdatePaused = false;

    private static readonly JsonSerializerOptions _jsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = false,
        Converters = { new JsonStringEnumConverter() }
    };

    public WebViewBridge(
        CoreWebView2 webView,
        GameInstallation installation,
        CatalogService catalog,
        BackupManager backupManager,
        ModificationEngine modEngine,
        PresetManager presetManager,
        Action? onMinimize = null,
        Func<bool>? onMaximize = null,
        Action? onClose = null,
        Action? onDrag = null,
        Func<bool>? isWindowMaximized = null)
    {
        _webView = webView;
        _installation = installation;
        _catalog = catalog;
        _backupManager = backupManager;
        _modEngine = modEngine;
        _presetManager = presetManager;
        _onMinimize = onMinimize;
        _onMaximize = onMaximize;
        _onClose = onClose;
        _onDrag = onDrag;
        _isWindowMaximized = isWindowMaximized;

        _webView.WebMessageReceived += OnWebMessageReceived;
    }

    private async void OnWebMessageReceived(object? sender, CoreWebView2WebMessageReceivedEventArgs e)
    {
        string rawMessage = e.TryGetWebMessageAsString();
        if (string.IsNullOrWhiteSpace(rawMessage)) return;

        string reqId = "";
        string action = "";

        try
        {
            using var doc = JsonDocument.Parse(rawMessage);
            var root = doc.RootElement;

            if (root.TryGetProperty("id", out var idProp))
                reqId = idProp.GetString() ?? "";

            if (root.TryGetProperty("action", out var actionProp))
                action = actionProp.GetString() ?? "";

            App.SafeLog("bridge.log", $"[{DateTime.UtcNow}] Request: action={action}, reqId={reqId}\n");

            JsonElement payload = root.TryGetProperty("payload", out var pProp) ? pProp : default;

            object? result = await HandleActionAsync(action, payload);
            SendResponse(reqId, success: true, data: result, error: null);
        }
        catch (Exception ex)
        {
            SendResponse(reqId, success: false, data: null, error: ex.Message);
        }
    }

    private async Task<object?> HandleActionAsync(string action, JsonElement payload)
    {
        return action switch
        {
            "getSystemInfo" => GetSystemInfo(),
            "getCatalog" => GetCatalog(),
            "validateModification" => ValidateModification(payload),
            "applyModification" => await Task.Run(() => ApplyModification(payload)),
            "applyAdvancedModification" => await Task.Run(() => ApplyAdvancedModification(payload)),
            "restoreCostume" => RestoreCostume(payload),
            "restoreAll" => RestoreAll(),
            "getPresets" => _presetManager.Presets,
            "savePreset" => SavePreset(payload),
            "deletePreset" => DeletePreset(payload),
            "duplicatePreset" => DuplicatePreset(payload),
            "exportPreset" => ExportPreset(payload),
            "importPreset" => ImportPreset(payload),
            "getBackups" => _backupManager.Records,
            "restoreBackupFile" => RestoreBackupFile(payload),
            "getDiagnostics" => DiagnosticsService.RunDiagnostics(_installation, _catalog, _backupManager),
            "launchGame" => LaunchGame(),
            "restartGame" => GameLauncher.RestartGame(_installation.ExePath, preferSteam: true),
            "bringGameToFront" => GameLauncher.BringGameToFront(),
            "isGameRunning" => GameLauncher.IsGameRunning(),
            "renderPreview" => RenderPreview(payload),
            "getAssetSvg" => GetAssetSvg(payload),
            "getVerificationSteps" => GetVerificationSteps(payload),
            "windowMinimize" => HandleMinimize(),
            "windowMaximize" => HandleMaximize(),
            "isWindowMaximized" => _isWindowMaximized?.Invoke() ?? false,
            "windowClose" => HandleClose(),
            "windowDrag" => HandleDrag(),
            "logClientError" => HandleClientError(payload),
            "selectGamePath" => SelectGamePath(payload),
            "browseGameExe" => BrowseGameExe(),
            "clearAssetCache" => ClearAssetCache(),
            "repairGameInstallation" => RepairGameInstallation(),
            "checkForUpdates" => await CheckForUpdatesAsync(payload),
            "installUpdate" => await InstallUpdateAsync(payload),
            "pauseUpdate" => PauseUpdate(),
            "resumeUpdate" => ResumeUpdate(),
            "cancelUpdate" => CancelUpdate(),
            "setGitHubRepo" => SetGitHubRepo(payload),
            "getGitHubRepo" => GetGitHubRepo(),
            "openExternalUrl" => OpenExternalUrl(payload),
            _ => throw new NotSupportedException($"Unknown action: '{action}'")
        };
    }

    private bool HandleClientError(JsonElement payload)
    {
        try
        {
            string msg = payload.TryGetProperty("message", out var mProp) ? mProp.GetString() ?? "" : "";
            string stack = payload.TryGetProperty("stack", out var sProp) ? sProp.GetString() ?? "" : "";
            string compStack = payload.TryGetProperty("componentStack", out var csProp) ? csProp.GetString() ?? "" : "";
            App.SafeLog("bridge.log", $"[{DateTime.UtcNow}] [FRONTEND_ERROR] {msg}\nStack: {stack}\nComponentStack: {compStack}\n");
        }
        catch { }
        return true;
    }

    private bool HandleDrag()
    {
        _onDrag?.Invoke();
        return true;
    }

    private bool HandleMinimize()
    {
        _onMinimize?.Invoke();
        return true;
    }

    private bool HandleMaximize()
    {
        return _onMaximize?.Invoke() ?? false;
    }

    private bool HandleClose()
    {
        _onClose?.Invoke();
        return true;
    }

    private string? BrowseGameExe()
    {
        var dialog = new Microsoft.Win32.OpenFileDialog
        {
            Title = "Select Brawlhalla.exe",
            Filter = "Brawlhalla Executable (Brawlhalla.exe)|Brawlhalla.exe|All Executables (*.exe)|*.exe",
            CheckFileExists = true
        };

        bool? res = dialog.ShowDialog();
        if (res == true)
        {
            return dialog.FileName;
        }
        return null;
    }

    private object SelectGamePath(JsonElement payload)
    {
        string? specifiedPath = null;
        if (payload.ValueKind == JsonValueKind.String)
        {
            specifiedPath = payload.GetString();
        }
        else if (payload.ValueKind == JsonValueKind.Object && payload.TryGetProperty("path", out var pProp))
        {
            specifiedPath = pProp.GetString();
        }

        if (string.IsNullOrWhiteSpace(specifiedPath))
        {
            throw new ArgumentException("Path cannot be empty.");
        }

        string cleanPath = specifiedPath.Trim().Trim('"', '\'');
        if (File.Exists(cleanPath) && Path.GetFileName(cleanPath).Equals("Brawlhalla.exe", StringComparison.OrdinalIgnoreCase))
        {
            cleanPath = Path.GetDirectoryName(cleanPath) ?? cleanPath;
        }

        var detected = GameDetector.DetectGameInstallation(cleanPath);
        if (!detected.IsValid)
        {
            string errors = string.Join("; ", detected.ValidationMessages);
            throw new InvalidOperationException($"Invalid Brawlhalla path: {errors}");
        }

        // Save path to AppData config
        try
        {
            string cfgDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BrawlhallaSkinStudio");
            Directory.CreateDirectory(cfgDir);
            File.WriteAllText(Path.Combine(cfgDir, "game_path.txt"), detected.GamePath);
        }
        catch { }

        // Reinitialize services with new game path
        _installation = detected;
        _backupManager = new BackupManager(_installation.GamePath);
        _catalog.LoadFromInstallation(_installation);
        _modEngine = new ModificationEngine(_installation.GamePath, _backupManager, _catalog);
        _presetManager = new PresetManager(Path.Combine(_installation.GamePath, ".studio_presets"));

        return new
        {
            success = true,
            systemInfo = GetSystemInfo(),
            catalog = GetCatalog()
        };
    }

    private object GetSystemInfo()
    {
        int activeMods = _backupManager.CountModifiedFiles();

        return new
        {
            gamePath = _installation.GamePath,
            version = _installation.Version,
            buildId = _installation.BuildId,
            discoveredSwfCount = _installation.DiscoveredSwfCount,
            isValid = _installation.IsValid,
            isGameRunning = GameLauncher.IsGameRunning(),
            activeModCount = activeMods,
            appVersion = CurrentVersion,
            totalLegends = _catalog.Legends.Count,
            totalCostumes = _catalog.Costumes.Count,
            totalWeaponSkins = _catalog.WeaponSkins.Count,
            totalColorSchemes = _catalog.ColorSchemes.Count
        };
    }

    private object GetCatalog()
    {
        return new
        {
            legends = _catalog.Legends,
            costumes = _catalog.Costumes,
            weaponSkins = _catalog.WeaponSkins,
            colorSchemes = _catalog.ColorSchemes,
            weaponTypes = _catalog.WeaponSkinsByType.Keys.OrderBy(k => k).ToList(),
            koEffects = _catalog.KoEffects,
            sidekicks = _catalog.Sidekicks,
            podiums = _catalog.Podiums,
            trailEffects = _catalog.TrailEffects,
            crossoverSfx = _catalog.CrossoverSfx
        };
    }

    private AdvancedModificationResult ApplyAdvancedModification(JsonElement payload)
    {
        var req = JsonSerializer.Deserialize<AdvancedModificationRequest>(payload.GetRawText(), _jsonOptions)
            ?? new AdvancedModificationRequest();
        var res = _modEngine.ApplyAdvancedModification(req);
        if (res.Success && req.AutoRestart && GameLauncher.IsGameRunning())
        {
            GameLauncher.RestartGame(_installation.ExePath, preferSteam: true);
        }
        return res;
    }

    private CompatibilityReport ValidateModification(JsonElement payload)
    {
        var req = DeserializeRequest(payload);
        return _modEngine.ValidateModification(req);
    }

    private ModificationResult ApplyModification(JsonElement payload)
    {
        var req = DeserializeRequest(payload);
        var result = _modEngine.ApplyModification(req);

        if (result.Success)
        {
            PreviewRenderer.InvalidateCache();
        }

        bool autoRestart = payload.TryGetProperty("autoRestart", out var arProp) ? arProp.GetBoolean() : true;
        if (result.Success && autoRestart && GameLauncher.IsGameRunning())
        {
            GameLauncher.RestartGame(_installation.ExePath, preferSteam: true);
            result.OperationLog.Add("Auto-restarted Brawlhalla to load modifications live.");
        }

        return result;
    }

    private bool RestoreCostume(JsonElement payload)
    {
        string? swfFile = payload.TryGetProperty("swfFile", out var prop) ? prop.GetString() : null;
        if (string.IsNullOrEmpty(swfFile)) return false;
        bool restored = _backupManager.RestoreFile(swfFile);

        if (_backupManager.IsFileModified("Gfx_Hands.swf"))
        {
            _backupManager.RestoreFile("Gfx_Hands.swf");
        }
        foreach (var rec in _backupManager.Records)
        {
            foreach (var key in rec.BackupFilesRelativePath.Keys)
            {
                if (key.StartsWith("SFX_", StringComparison.OrdinalIgnoreCase) && _backupManager.IsFileModified(key))
                {
                    _backupManager.RestoreFile(key);
                }
            }
        }

        if (restored)
        {
            PreviewRenderer.InvalidateCache(swfFile);
        }
        return restored;
    }

    private int RestoreAll()
    {
        int count = _backupManager.RestoreAll();
        PreviewRenderer.InvalidateCache();
        return count;
    }

    private Preset SavePreset(JsonElement payload)
    {
        var preset = JsonSerializer.Deserialize<Preset>(payload.GetRawText(), _jsonOptions);
        if (preset == null) throw new ArgumentException("Invalid preset data");
        _presetManager.SavePreset(preset);
        return preset;
    }

    private bool DeletePreset(JsonElement payload)
    {
        string? id = payload.TryGetProperty("id", out var prop) ? prop.GetString() : null;
        if (string.IsNullOrEmpty(id)) return false;
        return _presetManager.DeletePreset(id);
    }

    private Preset DuplicatePreset(JsonElement payload)
    {
        string? id = payload.TryGetProperty("id", out var idProp) ? idProp.GetString() : null;
        string? newName = payload.TryGetProperty("newName", out var nameProp) ? nameProp.GetString() : "Duplicate Preset";
        if (string.IsNullOrEmpty(id)) throw new ArgumentException("Preset ID required");
        return _presetManager.DuplicatePreset(id, newName ?? "Duplicate Preset");
    }

    private string ExportPreset(JsonElement payload)
    {
        string? id = payload.TryGetProperty("id", out var prop) ? prop.GetString() : null;
        if (string.IsNullOrEmpty(id)) throw new ArgumentException("Preset ID required");
        return _presetManager.ExportPresetJson(id);
    }

    private Preset ImportPreset(JsonElement payload)
    {
        string? json = payload.TryGetProperty("json", out var prop) ? prop.GetString() : null;
        if (string.IsNullOrEmpty(json)) throw new ArgumentException("Preset JSON required");
        return _presetManager.ImportPresetJson(json);
    }

    private bool RestoreBackupFile(JsonElement payload)
    {
        string? relPath = payload.TryGetProperty("relativePath", out var prop) ? prop.GetString() : null;
        if (string.IsNullOrEmpty(relPath)) return false;
        bool restored = _backupManager.RestoreFile(relPath);
        if (restored)
        {
            PreviewRenderer.InvalidateCache(relPath);
        }
        return restored;
    }

    private bool LaunchGame()
    {
        return GameLauncher.LaunchGame(_installation.ExePath, preferSteam: true);
    }

    private string? RenderPreview(JsonElement payload)
    {
        string? swf = payload.TryGetProperty("swfFile", out var swfProp) ? swfProp.GetString() : null;
        string? symbol = payload.TryGetProperty("symbol", out var symProp) ? symProp.GetString() : null;
        if (string.IsNullOrEmpty(symbol)) return null;

        if (string.IsNullOrEmpty(swf)) swf = "UI_Icons.swf";
        string fullSwf = Path.IsPathRooted(swf) ? swf : Path.Combine(_installation.GamePath, swf);
        if (!File.Exists(fullSwf))
        {
            string iconsSwf = Path.Combine(_installation.GamePath, "UI_Icons.swf");
            if (File.Exists(iconsSwf)) fullSwf = iconsSwf;
        }

        return PreviewRenderer.RenderSymbolToSvg(fullSwf, symbol);
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
        ["ActualGladiator"] = "a_Roster_Pose_Gladiator"
    };

    private string? ResolveLegendPoseSvg(string uiIconsPath, string? name, string? hero)
    {
        if (string.IsNullOrEmpty(name) && string.IsNullOrEmpty(hero)) return null;

        var candidates = new List<string>();

        if (!string.IsNullOrEmpty(name) && LegendPoseOverrides.TryGetValue(name, out var ov1))
            candidates.Add(ov1);
        if (!string.IsNullOrEmpty(hero) && LegendPoseOverrides.TryGetValue(hero, out var ov2))
            candidates.Add(ov2);

        var legend = _catalog.Legends.FirstOrDefault(l =>
            (!string.IsNullOrEmpty(name) && (l.DisplayName.Equals(name, StringComparison.OrdinalIgnoreCase) || l.HeroName.Equals(name, StringComparison.OrdinalIgnoreCase))) ||
            (!string.IsNullOrEmpty(hero) && (l.HeroName.Equals(hero, StringComparison.OrdinalIgnoreCase) || l.DisplayName.Equals(hero, StringComparison.OrdinalIgnoreCase))));

        if (legend != null)
        {
            if (LegendPoseOverrides.TryGetValue(legend.DisplayName, out var ov3)) candidates.Add(ov3);
            if (LegendPoseOverrides.TryGetValue(legend.HeroName, out var ov4)) candidates.Add(ov4);
            if (!string.IsNullOrEmpty(legend.PoseSymbol)) candidates.Add(legend.PoseSymbol);

            string legClean = legend.DisplayName.Replace(" ", "").Replace("ö", "o").Replace("é", "e");
            candidates.Add($"a_Roster_Pose_{legClean}");
            candidates.Add($"a_Roster_Pose_{legend.HeroName}");
        }

        if (!string.IsNullOrEmpty(name))
        {
            string clean = name.Replace(" ", "").Replace("ö", "o").Replace("é", "e");
            candidates.Add($"a_Roster_Pose_{clean}");
            candidates.Add($"a_Roster_Pose_{name.Replace(" ", "")}");
        }
        if (!string.IsNullOrEmpty(hero))
        {
            candidates.Add($"a_Roster_Pose_{hero}");
        }

        foreach (var sym in candidates.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            var svg = PreviewRenderer.RenderSymbolToSvg(uiIconsPath, sym);
            if (svg != null) return svg;
        }

        return null;
    }

    private string? GetAssetSvg(JsonElement payload)
    {
        string? type = payload.TryGetProperty("type", out var tProp) ? tProp.GetString() : null;
        if (string.IsNullOrEmpty(type)) return null;

        string uiIcons = Path.Combine(_installation.GamePath, "UI_Icons.swf");

        if (type.Equals("legend", StringComparison.OrdinalIgnoreCase))
        {
            string? name = payload.TryGetProperty("name", out var nProp) ? nProp.GetString() : null;
            string? hero = payload.TryGetProperty("hero", out var hProp) ? hProp.GetString() : null;
            return ResolveLegendPoseSvg(uiIcons, name, hero);
        }
        else if (type.Equals("costume", StringComparison.OrdinalIgnoreCase))
        {
            string? icon = payload.TryGetProperty("icon", out var iProp) ? iProp.GetString() : null;
            string? iconFile = payload.TryGetProperty("iconFile", out var ifProp) ? ifProp.GetString() : null;
            string? costumeName = payload.TryGetProperty("costumeName", out var cnProp) ? cnProp.GetString() : null;
            string? ownerHero = payload.TryGetProperty("ownerHero", out var ohProp) ? ohProp.GetString() : null;
            string? swfFile = payload.TryGetProperty("swfFile", out var sfProp) ? sfProp.GetString() : null;
            string? colorSchemeId = payload.TryGetProperty("colorSchemeId", out var csProp)
                ? (csProp.ValueKind == JsonValueKind.Number ? csProp.GetInt64().ToString() : csProp.GetString())
                : null;

            Dictionary<uint, uint>? colorMap = null;
            if (!string.IsNullOrEmpty(colorSchemeId) && !colorSchemeId.Equals("__NONE__", StringComparison.OrdinalIgnoreCase))
            {
                var scheme = _catalog.ColorSchemes.FirstOrDefault(cs =>
                    cs.SchemeId.ToString().Equals(colorSchemeId, StringComparison.OrdinalIgnoreCase) ||
                    cs.SchemeName.Equals(colorSchemeId, StringComparison.OrdinalIgnoreCase) ||
                    cs.DisplayName.Equals(colorSchemeId, StringComparison.OrdinalIgnoreCase));

                if (scheme != null && scheme.SwapColors.Count > 0)
                {
                    var cost = _catalog.Costumes.FirstOrDefault(c =>
                        (!string.IsNullOrEmpty(ownerHero) && c.OwnerHero.Equals(ownerHero, StringComparison.OrdinalIgnoreCase) && (c.CostumeName.Equals(costumeName, StringComparison.OrdinalIgnoreCase) || c.DisplayName.Equals(costumeName, StringComparison.OrdinalIgnoreCase))) ||
                        c.CostumeName.Equals(costumeName, StringComparison.OrdinalIgnoreCase) ||
                        c.DisplayName.Equals(costumeName, StringComparison.OrdinalIgnoreCase));

                    if (cost != null && cost.BaseColors.Count > 0)
                    {
                        colorMap = new Dictionary<uint, uint>();
                        foreach (var kvp in scheme.SwapColors)
                        {
                            if (cost.BaseColors.TryGetValue(kvp.Key, out uint baseVal))
                            {
                                colorMap[baseVal] = kvp.Value;
                            }
                        }
                    }
                }
            }

            // In Brawlhalla UI_Icons.swf, classic/default costumes can use either canonical roster pose or full classic skin icon.
            // If the costume is classic/default, first try resolving the canonical legend pose.
            if ((!string.IsNullOrEmpty(icon) && icon.EndsWith("_Classic", StringComparison.OrdinalIgnoreCase)) ||
                (!string.IsNullOrEmpty(costumeName) && costumeName.Equals("Classic", StringComparison.OrdinalIgnoreCase)))
            {
                var poseSvg = ResolveLegendPoseSvg(uiIcons, ownerHero ?? costumeName, ownerHero);
                if (poseSvg != null) return poseSvg;

                if (!string.IsNullOrEmpty(icon))
                {
                    var classicIconSvg = PreviewRenderer.RenderSymbolToSvg(uiIcons, icon, null, colorMap);
                    if (classicIconSvg != null) return classicIconSvg;
                }
            }

            string targetFile = !string.IsNullOrEmpty(iconFile) ? Path.Combine(_installation.GamePath, iconFile) : uiIcons;
            if (!File.Exists(targetFile)) targetFile = uiIcons;

            // Render non-classic icon if available
            if (!string.IsNullOrEmpty(icon) && !icon.EndsWith("_Classic", StringComparison.OrdinalIgnoreCase))
            {
                var svg = PreviewRenderer.RenderSymbolToSvg(targetFile, icon, null, colorMap);
                if (svg != null) return svg;
            }

            if (!string.IsNullOrEmpty(costumeName) && !costumeName.Equals("Classic", StringComparison.OrdinalIgnoreCase))
            {
                var svg = PreviewRenderer.RenderSymbolToSvg(uiIcons, $"a_SkinIcon_{ownerHero}_{costumeName}", null, colorMap)
                       ?? PreviewRenderer.RenderSymbolToSvg(uiIcons, $"a_SkinIcon_{costumeName}", null, colorMap);
                if (svg != null) return svg;
            }

            if (!string.IsNullOrEmpty(swfFile))
            {
                string fullSwf = Path.Combine(_installation.GamePath, swfFile);
                var headSvg = PreviewRenderer.RenderSymbolToSvg(fullSwf, $"a_Head_{costumeName}", null, colorMap)
                    ?? PreviewRenderer.RenderSymbolToSvg(fullSwf, $"a_Torso_{costumeName}", null, colorMap);
                if (headSvg != null) return headSvg;
            }

            // Fallback to roster pose if custom skin art is not found
            if (!string.IsNullOrEmpty(ownerHero))
            {
                var fallbackSvg = ResolveLegendPoseSvg(uiIcons, ownerHero, ownerHero);
                if (fallbackSvg != null) return fallbackSvg;
            }
        }
        else if (type.Equals("weapon", StringComparison.OrdinalIgnoreCase))
        {
            string? swf = payload.TryGetProperty("swfFile", out var swfProp) ? swfProp.GetString() : null;
            string? symbol = payload.TryGetProperty("symbol", out var symProp) ? symProp.GetString() : null;
            string? baseWeapon = payload.TryGetProperty("baseWeapon", out var bwProp) ? bwProp.GetString() : null;
            string? ownerHero = payload.TryGetProperty("ownerHero", out var ohProp) ? ohProp.GetString() : null;
            string? colorSchemeId = payload.TryGetProperty("colorSchemeId", out var csProp)
                ? (csProp.ValueKind == JsonValueKind.Number ? csProp.GetInt64().ToString() : csProp.GetString())
                : null;

            Dictionary<uint, uint>? colorMap = null;
            if (!string.IsNullOrEmpty(colorSchemeId) && !colorSchemeId.Equals("__NONE__", StringComparison.OrdinalIgnoreCase))
            {
                var scheme = _catalog.ColorSchemes.FirstOrDefault(cs =>
                    cs.SchemeId.ToString().Equals(colorSchemeId, StringComparison.OrdinalIgnoreCase) ||
                    cs.SchemeName.Equals(colorSchemeId, StringComparison.OrdinalIgnoreCase) ||
                    cs.DisplayName.Equals(colorSchemeId, StringComparison.OrdinalIgnoreCase));

                if (scheme != null && scheme.SwapColors.Count > 0)
                {
                    var wep = _catalog.WeaponSkins.FirstOrDefault(w =>
                        (!string.IsNullOrEmpty(symbol) && (w.SymbolSuffix.Equals(symbol, StringComparison.OrdinalIgnoreCase) || w.WeaponSkinName.Equals(symbol, StringComparison.OrdinalIgnoreCase))) ||
                        (!string.IsNullOrEmpty(ownerHero) && w.OwnerHero.Equals(ownerHero, StringComparison.OrdinalIgnoreCase) && !string.IsNullOrEmpty(baseWeapon) && w.BaseWeapon.Equals(baseWeapon, StringComparison.OrdinalIgnoreCase)) ||
                        (!string.IsNullOrEmpty(baseWeapon) && w.BaseWeapon.Equals(baseWeapon, StringComparison.OrdinalIgnoreCase)));

                    string heroToLookUp = !string.IsNullOrEmpty(ownerHero) && !ownerHero.Equals("--", StringComparison.OrdinalIgnoreCase)
                        ? ownerHero
                        : (wep?.OwnerHero ?? "");

                    var baseColorSources = new List<Dictionary<string, uint>>();
                    if (wep?.BaseColors != null && wep.BaseColors.Count > 0)
                        baseColorSources.Add(wep.BaseColors);

                    var heroesToCheck = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
                    if (!string.IsNullOrEmpty(heroToLookUp) && !heroToLookUp.Equals("--", StringComparison.OrdinalIgnoreCase))
                        heroesToCheck.Add(heroToLookUp);
                    if (!string.IsNullOrEmpty(wep?.InheritCostumeDefines))
                        heroesToCheck.Add(wep.InheritCostumeDefines);

                    foreach (var h in heroesToCheck)
                    {
                        var costumes = _catalog.Costumes
                            .Where(c => c.OwnerHero.Equals(h, StringComparison.OrdinalIgnoreCase) || c.CostumeName.Equals(h, StringComparison.OrdinalIgnoreCase))
                            .ToList();
                        foreach (var hc in costumes)
                        {
                            if (hc.BaseColors != null && hc.BaseColors.Count > 0)
                                baseColorSources.Add(hc.BaseColors);
                        }
                    }

                    if (baseColorSources.Count > 0)
                    {
                        colorMap = new Dictionary<uint, uint>();
                        foreach (var kvp in scheme.SwapColors)
                        {
                            foreach (var bcs in baseColorSources)
                            {
                                if (bcs.TryGetValue(kvp.Key, out uint baseVal))
                                {
                                    colorMap[baseVal] = kvp.Value;
                                }
                            }
                        }
                        if (colorMap.Count == 0)
                        {
                            colorMap = null;
                        }
                    }
                }
            }

            string bw = baseWeapon?.Trim() ?? "";
            bool isPairedType = bw.Equals("Katar", StringComparison.OrdinalIgnoreCase) ||
                               bw.Equals("Katars", StringComparison.OrdinalIgnoreCase) ||
                               bw.Equals("Boots", StringComparison.OrdinalIgnoreCase) ||
                               bw.Equals("BattleBoots", StringComparison.OrdinalIgnoreCase) ||
                               bw.Equals("Fists", StringComparison.OrdinalIgnoreCase) ||
                               bw.Equals("Gauntlet", StringComparison.OrdinalIgnoreCase) ||
                               bw.Equals("Gauntlets", StringComparison.OrdinalIgnoreCase);

            if (isPairedType)
            {
                var pairedSvg = PreviewRenderer.RenderPairedWeaponToSvg(_installation.GamePath, bw, symbol ?? "", swf, ownerHero, colorMap);
                if (pairedSvg != null) return pairedSvg;
            }

            if (!string.IsNullOrEmpty(symbol))
            {
                var swfList = new List<string>();
                if (!string.IsNullOrEmpty(swf))
                {
                    swfList.Add(Path.Combine(_installation.GamePath, swf));
                }
                if (!string.IsNullOrEmpty(ownerHero) && !ownerHero.Equals("--", StringComparison.OrdinalIgnoreCase))
                {
                    string heroSwf = Path.Combine(_installation.GamePath, $"Gfx_{ownerHero}.swf");
                    if (File.Exists(heroSwf) && !swfList.Contains(heroSwf, StringComparer.OrdinalIgnoreCase))
                    {
                        swfList.Add(heroSwf);
                    }
                }
                string[] fallbacks =
                [
                    Path.Combine(_installation.GamePath, "Gfx_Weapons.swf"),
                    Path.Combine(_installation.GamePath, "Gfx_Weapons_02.swf"),
                    Path.Combine(_installation.GamePath, "Gfx_Weapons_04.swf"),
                    Path.Combine(_installation.GamePath, "Gfx_Weapons_Chakram.swf"),
                    Path.Combine(_installation.GamePath, "Gfx_Weapons_OneOff.swf"),
                    Path.Combine(_installation.GamePath, "Gfx_Weapons_Sets.swf")
                ];
                foreach (var fb in fallbacks)
                {
                    if (!swfList.Contains(fb, StringComparer.OrdinalIgnoreCase) && File.Exists(fb))
                        swfList.Add(fb);
                }
                try
                {
                    var extraWeaponSwfs = Directory.GetFiles(_installation.GamePath, "Gfx_Weapons*.swf");
                    foreach (var ews in extraWeaponSwfs)
                    {
                        if (!swfList.Contains(ews, StringComparer.OrdinalIgnoreCase))
                            swfList.Add(ews);
                    }
                }
                catch { }

                var candidateSymbols = new List<string>();

                if (bw.Equals("Greatsword", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponGreat_{symbol}");
                    candidateSymbols.Add($"a_WeaponGreatsword_{symbol}");
                    candidateSymbols.Add("a_WeaponGreat");
                    candidateSymbols.Add("a_WeaponGreatsword");
                }
                else if (bw.Equals("Fists", StringComparison.OrdinalIgnoreCase) || bw.Equals("Gauntlet", StringComparison.OrdinalIgnoreCase) || bw.Equals("Gauntlets", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponFists01Large_{symbol}");
                    candidateSymbols.Add($"a_WeaponFists01_{symbol}");
                    candidateSymbols.Add($"a_WeaponFists_{symbol}");
                    candidateSymbols.Add($"a_WeaponFistsForearmLarge_{symbol}");
                    candidateSymbols.Add($"a_WeaponFists02_{symbol}");
                    candidateSymbols.Add($"a_WeaponGauntlet_{symbol}");
                    candidateSymbols.Add($"a_WeaponGauntlets_{symbol}");
                    candidateSymbols.Add("a_WeaponFists01Large");
                    candidateSymbols.Add("a_WeaponFists01");
                }
                else if (bw.Equals("Katar", StringComparison.OrdinalIgnoreCase) || bw.Equals("Katars", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponKatarTop_{symbol}");
                    candidateSymbols.Add($"a_WeaponKatar_{symbol}");
                    candidateSymbols.Add($"a_WeaponKatarBladeTop_{symbol}");
                    candidateSymbols.Add($"a_WeaponKatarBlade_{symbol}");
                    candidateSymbols.Add($"a_WeaponKatarUnder_{symbol}");
                    candidateSymbols.Add($"a_WeaponKatar01_{symbol}");
                    candidateSymbols.Add($"a_WeaponKatars_{symbol}");
                    candidateSymbols.Add("a_WeaponKatarTop");
                    candidateSymbols.Add("a_WeaponKatar");
                }
                else if (bw.Equals("Boots", StringComparison.OrdinalIgnoreCase) || bw.Equals("BattleBoots", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponBootsSide_{symbol}");
                    candidateSymbols.Add($"a_WeaponBootsFront_{symbol}");
                    candidateSymbols.Add($"a_WeaponBootsBack_{symbol}");
                    candidateSymbols.Add($"a_WeaponBootsSideTop_{symbol}");
                    candidateSymbols.Add($"a_WeaponBoots_{symbol}");
                    candidateSymbols.Add($"a_WeaponBoots01_{symbol}");
                    candidateSymbols.Add("a_WeaponBootsSide");
                    candidateSymbols.Add("a_WeaponBootsFront");
                }
                else if (bw.Equals("Chakram", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponChakramCombined_{symbol}");
                    candidateSymbols.Add($"a_WeaponChakramRight_{symbol}");
                    candidateSymbols.Add($"a_WeaponChakram_{symbol}");
                    candidateSymbols.Add($"a_WeaponChakramAngleAway_{symbol}");
                    candidateSymbols.Add("a_WeaponChakramCombined");
                }
                else if (bw.Equals("Hammer", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponHammer_{symbol}");
                    candidateSymbols.Add($"a_WeaponHammer01_{symbol}");
                    candidateSymbols.Add("a_WeaponHammer");
                }
                else if (bw.Equals("Sword", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponSword_{symbol}");
                    candidateSymbols.Add($"a_WeaponSword01_{symbol}");
                    candidateSymbols.Add("a_WeaponSword");
                }
                else if (bw.Equals("Pistol", StringComparison.OrdinalIgnoreCase) || bw.Equals("Blasters", StringComparison.OrdinalIgnoreCase) || bw.Equals("Blaster", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponPistol_{symbol}");
                    candidateSymbols.Add($"a_WeaponPistol01_{symbol}");
                    candidateSymbols.Add("a_WeaponPistol");
                }
                else if (bw.Equals("Spear", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponSpear_{symbol}");
                    candidateSymbols.Add($"a_WeaponSpear01_{symbol}");
                    candidateSymbols.Add("a_WeaponSpear");
                }
                else if (bw.Equals("RocketLance", StringComparison.OrdinalIgnoreCase) || bw.Equals("Lance", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponLance_{symbol}");
                    candidateSymbols.Add($"a_WeaponRocketLance_{symbol}");
                    candidateSymbols.Add("a_WeaponLance");
                }
                else if (bw.Equals("Axe", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponAxe_{symbol}");
                    candidateSymbols.Add($"a_WeaponAxe01_{symbol}");
                    candidateSymbols.Add("a_WeaponAxe");
                }
                else if (bw.Equals("Bow", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponBow_{symbol}");
                    candidateSymbols.Add($"a_WeaponBow01_{symbol}");
                    candidateSymbols.Add("a_WeaponBow");
                }
                else if (bw.Equals("Scythe", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponScythe_{symbol}");
                    candidateSymbols.Add($"a_WeaponScythe01_{symbol}");
                    candidateSymbols.Add("a_WeaponScythe");
                }
                else if (bw.Equals("Cannon", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponCannon_{symbol}");
                    candidateSymbols.Add($"a_WeaponCannon01_{symbol}");
                    candidateSymbols.Add("a_WeaponCannon");
                }
                else if (bw.Equals("Orb", StringComparison.OrdinalIgnoreCase))
                {
                    candidateSymbols.Add($"a_WeaponOrb_{symbol}");
                    candidateSymbols.Add($"a_WeaponOrb01_{symbol}");
                    candidateSymbols.Add("a_WeaponOrb");
                }
                else if (!string.IsNullOrEmpty(bw))
                {
                    candidateSymbols.Add($"a_Weapon{bw}_{symbol}");
                    candidateSymbols.Add($"a_Weapon{bw}");
                }

                // General fallback if base weapon specific did not match
                candidateSymbols.Add($"a_Weapon{symbol}");
                candidateSymbols.Add(symbol);

                foreach (var targetSwf in swfList)
                {
                    if (!File.Exists(targetSwf)) continue;
                    foreach (var sym in candidateSymbols)
                    {
                        var svg = PreviewRenderer.RenderSymbolToSvg(targetSwf, sym, null, colorMap);
                        if (svg != null) return svg;
                    }
                }
            }
        }
        else if (type.Equals("podium", StringComparison.OrdinalIgnoreCase))
        {
            string? animFile = payload.TryGetProperty("animFile", out var afProp) ? afProp.GetString() : null;
            string? name = payload.TryGetProperty("name", out var nProp) ? nProp.GetString() : null;
            if (!string.IsNullOrEmpty(animFile))
            {
                string targetSwf = Path.Combine(_installation.GamePath, animFile);
                if (File.Exists(targetSwf))
                {
                    var candidates = new List<string> { $"a_Podium_{name}", "a_Podium_Default", "a_Podium", name ?? "" };
                    foreach (var c in candidates)
                    {
                        var svg = PreviewRenderer.RenderSymbolToSvg(targetSwf, c);
                        if (svg != null) return svg;
                    }
                }
            }
        }
        else if (type.Equals("sidekick", StringComparison.OrdinalIgnoreCase))
        {
            string? animFile = payload.TryGetProperty("animFile", out var afProp) ? afProp.GetString() : null;
            string? name = payload.TryGetProperty("name", out var nProp) ? nProp.GetString() : null;
            if (!string.IsNullOrEmpty(animFile))
            {
                string targetSwf = Path.Combine(_installation.GamePath, animFile);
                if (File.Exists(targetSwf))
                {
                    var candidates = new List<string> { $"a_SpawnBot_{name}", "a_AnimationRobot", name ?? "" };
                    foreach (var c in candidates)
                    {
                        var svg = PreviewRenderer.RenderSymbolToSvg(targetSwf, c);
                        if (svg != null) return svg;
                    }
                }
            }
        }
        else if (type.Equals("ko", StringComparison.OrdinalIgnoreCase))
        {
            string? animFile = payload.TryGetProperty("animFile", out var afProp) ? afProp.GetString() : null;
            string? name = payload.TryGetProperty("name", out var nProp) ? nProp.GetString() : null;
            if (!string.IsNullOrEmpty(animFile))
            {
                string targetSwf = Path.Combine(_installation.GamePath, animFile);
                if (File.Exists(targetSwf))
                {
                    var candidates = new List<string> { $"a_KO_{name}", $"a_Explode_{name}", "a_Explode", name ?? "" };
                    foreach (var c in candidates)
                    {
                        var svg = PreviewRenderer.RenderSymbolToSvg(targetSwf, c);
                        if (svg != null) return svg;
                    }
                }
            }
        }

        return null;
    }

    private string[] GetVerificationSteps(JsonElement payload)
    {
        string legend = payload.TryGetProperty("legendName", out var lProp) ? lProp.GetString() ?? "" : "";
        string costume = payload.TryGetProperty("costumeName", out var cProp) ? cProp.GetString() ?? "" : "";
        string w1 = payload.TryGetProperty("weapon1", out var w1Prop) ? w1Prop.GetString() ?? "" : "";
        string w2 = payload.TryGetProperty("weapon2", out var w2Prop) ? w2Prop.GetString() ?? "" : "";
        return GameLauncher.GetVerificationChecklist(legend, costume, w1, w2);
    }

    private ModificationRequest DeserializeRequest(JsonElement payload)
    {
        if (payload.ValueKind != JsonValueKind.Object)
            throw new ArgumentException("Request payload must be an object.");

        if (!payload.TryGetProperty("targetLegend", out var tlProp) || tlProp.ValueKind != JsonValueKind.Object)
            throw new ArgumentException("Target Legend is required.");

        var targetLegend = JsonSerializer.Deserialize<LegendInfo>(tlProp.GetRawText(), _jsonOptions)
                           ?? throw new ArgumentException("Target Legend is required.");

        if (!payload.TryGetProperty("targetCostume", out var tcProp) || tcProp.ValueKind != JsonValueKind.Object)
            throw new ArgumentException("Target Costume is required.");

        var targetCostume = JsonSerializer.Deserialize<CostumeInfo>(tcProp.GetRawText(), _jsonOptions)
                            ?? throw new ArgumentException("Target Costume is required.");

        CostumeInfo? replacementCostume = null;
        if (payload.TryGetProperty("replacementCostume", out var rcProp) && rcProp.ValueKind == JsonValueKind.Object)
            replacementCostume = JsonSerializer.Deserialize<CostumeInfo>(rcProp.GetRawText(), _jsonOptions);

        WeaponSkinInfo? tw1 = null;
        if (payload.TryGetProperty("targetWeapon1", out var tw1Prop) && tw1Prop.ValueKind == JsonValueKind.Object)
            tw1 = JsonSerializer.Deserialize<WeaponSkinInfo>(tw1Prop.GetRawText(), _jsonOptions);

        WeaponSkinInfo? rw1 = null;
        if (payload.TryGetProperty("replacementWeapon1", out var rw1Prop) && rw1Prop.ValueKind == JsonValueKind.Object)
            rw1 = JsonSerializer.Deserialize<WeaponSkinInfo>(rw1Prop.GetRawText(), _jsonOptions);

        WeaponSkinInfo? tw2 = null;
        if (payload.TryGetProperty("targetWeapon2", out var tw2Prop) && tw2Prop.ValueKind == JsonValueKind.Object)
            tw2 = JsonSerializer.Deserialize<WeaponSkinInfo>(tw2Prop.GetRawText(), _jsonOptions);

        WeaponSkinInfo? rw2 = null;
        if (payload.TryGetProperty("replacementWeapon2", out var rw2Prop) && rw2Prop.ValueKind == JsonValueKind.Object)
            rw2 = JsonSerializer.Deserialize<WeaponSkinInfo>(rw2Prop.GetRawText(), _jsonOptions);

        ColorSchemeInfo? colorScheme = null;
        if (payload.TryGetProperty("selectedColorScheme", out var csProp) && csProp.ValueKind == JsonValueKind.Object)
            colorScheme = JsonSerializer.Deserialize<ColorSchemeInfo>(csProp.GetRawText(), _jsonOptions);

        return new ModificationRequest
        {
            TargetLegend = targetLegend,
            TargetCostume = targetCostume,
            ReplacementCostume = replacementCostume,
            TargetWeapon1 = tw1,
            ReplacementWeapon1 = rw1,
            TargetWeapon2 = tw2,
            ReplacementWeapon2 = rw2,
            SelectedColorScheme = colorScheme
        };
    }

    private object ClearAssetCache()
    {
        PreviewRenderer.InvalidateCache();
        App.SafeLog("bridge.log", "[MAINTENANCE] Cleared asset preview cache.");
        return new
        {
            success = true,
            message = "Asset preview cache cleared successfully."
        };
    }

    private object RepairGameInstallation()
    {
        App.SafeLog("bridge.log", $"[MAINTENANCE] Running diagnostic & repair at {DateTime.Now}...");

        PreviewRenderer.InvalidateCache();

        string? targetPath = _installation.IsValid ? _installation.GamePath : null;
        if (string.IsNullOrEmpty(targetPath))
        {
            try
            {
                string cfgDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BrawlhallaSkinStudio");
                string pathFile = Path.Combine(cfgDir, "game_path.txt");
                if (File.Exists(pathFile))
                {
                    targetPath = File.ReadAllText(pathFile).Trim();
                }
            }
            catch { }
        }

        var detected = GameDetector.DetectGameInstallation(targetPath);
        if (detected.IsValid)
        {
            _installation = detected;
            _backupManager = new BackupManager(_installation.GamePath);
            _catalog.LoadFromInstallation(_installation);
            _modEngine = new ModificationEngine(_installation.GamePath, _backupManager, _catalog);
            _presetManager = new PresetManager(Path.Combine(_installation.GamePath, ".studio_presets"));

            try
            {
                string cfgDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BrawlhallaSkinStudio");
                Directory.CreateDirectory(cfgDir);
                File.WriteAllText(Path.Combine(cfgDir, "game_path.txt"), detected.GamePath);
            }
            catch { }

            App.SafeLog("bridge.log", $"[MAINTENANCE] Repair successful. Path: {detected.GamePath}, Legends: {_catalog.Legends.Count}");

            return new
            {
                success = true,
                message = "Game installation validated and repaired successfully.",
                gamePath = _installation.GamePath,
                version = _installation.Version,
                swfCount = _installation.DiscoveredSwfCount,
                legendCount = _catalog.Legends.Count,
                costumeCount = _catalog.Costumes.Count,
                systemInfo = GetSystemInfo(),
                catalog = GetCatalog()
            };
        }
        else
        {
            App.SafeLog("bridge.log", $"[MAINTENANCE] Repair warning: {string.Join("; ", detected.ValidationMessages)}");
            return new
            {
                success = false,
                message = "Could not find a valid game installation. Please select Brawlhalla.exe manually.",
                validationMessages = detected.ValidationMessages
            };
        }
    }

    private const string DefaultGitHubRepo = "dahakabdellah12/BrawlCraft";

    private string GetGitHubRepo()
    {
        try
        {
            string cfgDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BrawlhallaSkinStudio");
            string repoFile = Path.Combine(cfgDir, "github_repo.txt");
            if (File.Exists(repoFile))
            {
                string custom = File.ReadAllText(repoFile).Trim();
                if (RepoPattern.IsMatch(custom))
                    return custom;
            }
        }
        catch { }
        return DefaultGitHubRepo;
    }

    private object SetGitHubRepo(JsonElement payload)
    {
        string? repo = payload.TryGetProperty("repo", out var rProp) ? rProp.GetString()?.Trim() : null;
        if (string.IsNullOrEmpty(repo) || !RepoPattern.IsMatch(repo))
        {
            return new
            {
                success = false,
                message = "Repository must use the owner/name format.",
                repo = GetGitHubRepo()
            };
        }

        try
        {
            string cfgDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BrawlhallaSkinStudio");
            Directory.CreateDirectory(cfgDir);
            File.WriteAllText(Path.Combine(cfgDir, "github_repo.txt"), repo);
        }
        catch { }
        return new { success = true, repo = GetGitHubRepo() };
    }

    private async Task<object> CheckForUpdatesAsync(JsonElement payload)
    {
        string requestedRepo = payload.TryGetProperty("repo", out var rProp) ? rProp.GetString()?.Trim() ?? "" : "";
        string targetRepo = RepoPattern.IsMatch(requestedRepo) ? requestedRepo : GetGitHubRepo();

        string repoUrl = $"https://github.com/{targetRepo}";
        string releasesUrl = $"{repoUrl}/releases";
        string apiUrl = $"https://api.github.com/repos/{targetRepo}/releases/latest";

        try
        {
            using var client = new HttpClient();
            client.Timeout = TimeSpan.FromSeconds(10);
            client.DefaultRequestHeaders.UserAgent.ParseAdd($"BrawlCraft-Updater/{CurrentVersion}");
            client.DefaultRequestHeaders.Accept.ParseAdd("application/vnd.github.v3+json");

            // Primary channel: GitHub Releases on the public repository (binaries only).
            // Fallback: downloads/latest.json, with a CDN mirror if GitHub is unreachable.
            try
            {
                var releaseResult = await TryReadGitHubReleaseAsync(client, apiUrl, targetRepo, repoUrl, releasesUrl);
                if (releaseResult != null)
                {
                    return releaseResult;
                }
            }
            catch (Exception releaseEx)
            {
                App.SafeLog("bridge.log", $"[UPDATE] GitHub Releases check failed ({apiUrl}): {releaseEx.Message}");
            }

            string primaryManifestUrl = $"https://raw.githubusercontent.com/{targetRepo}/master/downloads/latest.json?v={DateTime.UtcNow.Ticks}";
            string mirrorManifestUrl = $"https://cdn.jsdelivr.net/gh/{targetRepo}@master/downloads/latest.json";
            foreach (string manifestUrl in new[] { primaryManifestUrl, mirrorManifestUrl })
            {
                try
                {
                    var manifest = await TryReadUpdateManifestAsync(client, manifestUrl);
                    if (manifest != null)
                    {
                        return BuildManifestResult(manifest, targetRepo, repoUrl, releasesUrl);
                    }
                }
                catch (Exception manifestEx)
                {
                    App.SafeLog("bridge.log", $"[UPDATE] Manifest check failed ({manifestUrl}): {manifestEx.Message}");
                }
            }

            return new
            {
                success = true,
                hasUpdate = false,
                currentVersion = CurrentVersion,
                latestVersion = CurrentVersion,
                releaseTitle = $"BrawlCraft v{CurrentVersion} (Official)",
                releaseNotes = "You are running the latest stable release on GitHub.",
                htmlUrl = releasesUrl,
                downloadUrl = releasesUrl,
                repo = targetRepo,
                repoUrl,
                releasesUrl,
                checkedAt = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss")
            };
        }
        catch (Exception ex)
        {
            App.SafeLog("bridge.log", $"[UPDATE] GitHub check failed for {targetRepo}: {ex.Message}");
            return new
            {
                success = true,
                hasUpdate = false,
                currentVersion = CurrentVersion,
                latestVersion = CurrentVersion,
                networkError = true,
                errorMessage = ex.Message,
                repo = targetRepo,
                repoUrl,
                releasesUrl,
                checkedAt = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss")
            };
        }
    }

    private async Task<object?> TryReadGitHubReleaseAsync(
        HttpClient client,
        string apiUrl,
        string targetRepo,
        string repoUrl,
        string releasesUrl)
    {
        using var response = await client.GetAsync(apiUrl);
        if (!response.IsSuccessStatusCode) return null;

        var json = await response.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;
        string tagName = root.TryGetProperty("tag_name", out var tagProp) ? tagProp.GetString() ?? "" : "";
        string releaseName = root.TryGetProperty("name", out var nameProp) ? nameProp.GetString() ?? "" : "";
        string htmlUrl = root.TryGetProperty("html_url", out var urlProp) ? urlProp.GetString() ?? "" : releasesUrl;
        string releaseNotes = root.TryGetProperty("body", out var bProp) ? bProp.GetString() ?? "" : "";
        string publishedAt = root.TryGetProperty("published_at", out var pubProp) ? pubProp.GetString() ?? "" : "";

        string downloadUrl = htmlUrl;
        string? assetName = null;
        long assetSize = 0;
        string? sha256 = null;

        string? zipDownloadUrl = null;
        string? zipAssetName = null;
        long zipAssetSize = 0;
        string? zipDigest = null;

        string? exeDownloadUrl = null;
        string? exeAssetName = null;
        long exeAssetSize = 0;
        string? exeDigest = null;

        string? checksumsUrl = null;

        if (root.TryGetProperty("assets", out var assetsProp) && assetsProp.ValueKind == JsonValueKind.Array)
        {
            foreach (var asset in assetsProp.EnumerateArray())
            {
                string aName = asset.TryGetProperty("name", out var anProp) ? anProp.GetString() ?? "" : "";
                string bUrl = asset.TryGetProperty("browser_download_url", out var dProp) ? dProp.GetString() ?? "" : "";
                long sVal = asset.TryGetProperty("size", out var sProp) ? sProp.GetInt64() : 0;
                string? digest = ReadAssetDigest(asset);

                if (aName.Equals("SHA256SUMS.txt", StringComparison.OrdinalIgnoreCase))
                {
                    checksumsUrl = bUrl;
                }
                else if (aName.EndsWith(".zip", StringComparison.OrdinalIgnoreCase) && zipDownloadUrl == null)
                {
                    zipAssetName = aName;
                    zipDownloadUrl = bUrl;
                    zipAssetSize = sVal;
                    zipDigest = digest;
                }
                else if (aName.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) && exeDownloadUrl == null)
                {
                    exeAssetName = aName;
                    exeDownloadUrl = bUrl;
                    exeAssetSize = sVal;
                    exeDigest = digest;
                }
            }
        }

        if (string.IsNullOrEmpty(zipDownloadUrl) && string.IsNullOrEmpty(exeDownloadUrl))
        {
            return null;
        }

        if (!string.IsNullOrEmpty(zipDownloadUrl))
        {
            downloadUrl = zipDownloadUrl;
            assetName = zipAssetName;
            assetSize = zipAssetSize;
            sha256 = zipDigest;
        }
        else
        {
            downloadUrl = exeDownloadUrl!;
            assetName = exeAssetName;
            assetSize = exeAssetSize;
            sha256 = exeDigest;
        }

        if (sha256 == null && !string.IsNullOrEmpty(checksumsUrl) && !string.IsNullOrEmpty(assetName))
        {
            try
            {
                string? sumsText = await TryFetchTextAsync(client, checksumsUrl);
                if (sumsText != null)
                {
                    sha256 = FindChecksumForAsset(sumsText, assetName);
                }
            }
            catch (Exception ex)
            {
                App.SafeLog("bridge.log", $"[UPDATE] Checksum lookup failed: {ex.Message}");
            }
        }

        string latestVerClean = tagName.TrimStart('v', 'V');
        bool hasUpdate = false;
        if (Version.TryParse(latestVerClean, out var latestVer) && Version.TryParse(CurrentVersion, out var currVer))
        {
            hasUpdate = latestVer > currVer;
        }

        return new
        {
            success = true,
            hasUpdate,
            currentVersion = CurrentVersion,
            latestVersion = string.IsNullOrEmpty(latestVerClean) ? CurrentVersion : latestVerClean,
            releaseTitle = string.IsNullOrEmpty(releaseName) ? tagName : releaseName,
            releaseNotes,
            htmlUrl,
            downloadUrl,
            assetName,
            assetSizeFormatted = assetSize > 0 ? $"{assetSize / (1024.0 * 1024.0):F1} MB" : null,
            sha256,
            verified = sha256 != null,
            repo = targetRepo,
            repoUrl,
            releasesUrl,
            publishedAt,
            checkedAt = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss")
        };
    }

    private sealed class UpdateManifest
    {
        public string? Version { get; set; }
        public string? Title { get; set; }
        public string? Notes { get; set; }
        public string? AssetName { get; set; }
        public string? DownloadUrl { get; set; }
        public long Size { get; set; }
        public string? Sha256 { get; set; }
        public string? PublishedAt { get; set; }
    }

    private static async Task<UpdateManifest?> TryReadUpdateManifestAsync(HttpClient client, string url)
    {
        if (!IsAllowedGitHubDownloadUrl(url)) return null;

        using var response = await client.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
        if (!response.IsSuccessStatusCode) return null;
        if (response.Content.Headers.ContentLength is > 256 * 1024) return null;

        string json = await response.Content.ReadAsStringAsync();
        if (json.Length > 256 * 1024) return null;

        UpdateManifest? manifest;
        try
        {
            manifest = JsonSerializer.Deserialize<UpdateManifest>(json, _jsonOptions);
        }
        catch (JsonException)
        {
            return null;
        }

        if (manifest == null) return null;
        if (string.IsNullOrWhiteSpace(manifest.Version) || !VersionPattern.IsMatch(manifest.Version)) return null;
        if (string.IsNullOrWhiteSpace(manifest.DownloadUrl) || !IsAllowedGitHubDownloadUrl(manifest.DownloadUrl)) return null;
        if (string.IsNullOrWhiteSpace(manifest.Sha256) || !Sha256Pattern.IsMatch(manifest.Sha256)) return null;

        return manifest;
    }

    private object BuildManifestResult(UpdateManifest manifest, string targetRepo, string repoUrl, string releasesUrl)
    {
        bool hasUpdate = false;
        if (Version.TryParse(manifest.Version, out var latestVer) && Version.TryParse(CurrentVersion, out var currVer))
        {
            hasUpdate = latestVer > currVer;
        }

        string assetName = string.IsNullOrWhiteSpace(manifest.AssetName)
            ? Path.GetFileName(new Uri(manifest.DownloadUrl!).AbsolutePath)
            : manifest.AssetName!;

        return new
        {
            success = true,
            hasUpdate,
            currentVersion = CurrentVersion,
            latestVersion = manifest.Version,
            releaseTitle = string.IsNullOrWhiteSpace(manifest.Title) ? $"BrawlCraft v{manifest.Version}" : manifest.Title,
            releaseNotes = manifest.Notes ?? "",
            htmlUrl = $"{repoUrl}/releases/tag/v{manifest.Version}",
            downloadUrl = manifest.DownloadUrl,
            assetName,
            assetSizeFormatted = manifest.Size > 0 ? $"{manifest.Size / (1024.0 * 1024.0):F1} MB" : null,
            sha256 = manifest.Sha256!.ToLowerInvariant(),
            verified = true,
            repo = targetRepo,
            repoUrl,
            releasesUrl,
            publishedAt = manifest.PublishedAt,
            checkedAt = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss")
        };
    }

    private static string? ReadAssetDigest(JsonElement asset)
    {
        if (!asset.TryGetProperty("digest", out var digestProp) || digestProp.ValueKind != JsonValueKind.String)
            return null;

        string digest = digestProp.GetString() ?? "";
        const string prefix = "sha256:";
        if (!digest.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            return null;

        string hex = digest[prefix.Length..].Trim();
        return Sha256Pattern.IsMatch(hex) ? hex.ToLowerInvariant() : null;
    }

    private static string? FindChecksumForAsset(string sumsText, string assetName)
    {
        foreach (var raw in sumsText.Split('\n'))
        {
            string line = raw.Trim();
            if (line.Length == 0) continue;

            int separator = line.IndexOfAny([' ', '\t']);
            if (separator <= 0) continue;

            string hash = line[..separator].Trim();
            string name = line[(separator + 1)..].Trim().TrimStart('*').Trim();
            if (name.StartsWith("./", StringComparison.Ordinal)) name = name[2..];

            if (Sha256Pattern.IsMatch(hash) && string.Equals(name, assetName, StringComparison.OrdinalIgnoreCase))
            {
                return hash.ToLowerInvariant();
            }
        }
        return null;
    }

    private static async Task<string?> TryFetchTextAsync(HttpClient client, string url)
    {
        if (!IsAllowedGitHubDownloadUrl(url)) return null;

        using var response = await client.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
        if (!response.IsSuccessStatusCode) return null;
        if (response.Content.Headers.ContentLength is > 1024 * 1024) return null;

        string text = await response.Content.ReadAsStringAsync();
        return text.Length <= 1024 * 1024 ? text : null;
    }

    private static bool IsAllowedGitHubDownloadUrl(string? url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri)) return false;
        if (uri.Scheme != Uri.UriSchemeHttps) return false;

        return uri.Host.Equals("github.com", StringComparison.OrdinalIgnoreCase)
               || uri.Host.Equals("raw.githubusercontent.com", StringComparison.OrdinalIgnoreCase)
               || uri.Host.Equals("cdn.jsdelivr.net", StringComparison.OrdinalIgnoreCase)
               || uri.Host.Equals("objects.githubusercontent.com", StringComparison.OrdinalIgnoreCase)
               || uri.Host.Equals("github-releases.githubusercontent.com", StringComparison.OrdinalIgnoreCase);
    }

    private bool PauseUpdate()
    {
        _isUpdatePaused = true;
        _updatePauseEvent.Reset();
        SendEvent("updateProgress", new { status = "paused", isPaused = true });
        App.SafeLog("bridge.log", "[AUTO_UPDATE] Update download paused.");
        return true;
    }

    private bool ResumeUpdate()
    {
        _isUpdatePaused = false;
        _updatePauseEvent.Set();
        SendEvent("updateProgress", new { status = "downloading", isPaused = false });
        App.SafeLog("bridge.log", "[AUTO_UPDATE] Update download resumed.");
        return true;
    }

    private bool CancelUpdate()
    {
        _isUpdatePaused = false;
        _updatePauseEvent.Set();
        _updateCts?.Cancel();
        SendEvent("updateProgress", new { status = "cancelled", isPaused = false });
        App.SafeLog("bridge.log", "[AUTO_UPDATE] Update download cancelled.");
        return true;
    }

    private void SendEvent(string eventName, object? data)
    {
        try
        {
            var evt = new
            {
                type = "event",
                @event = eventName,
                data
            };
            string json = JsonSerializer.Serialize(evt, _jsonOptions);
            _webView.PostWebMessageAsJson(json);
        }
        catch (Exception ex)
        {
            App.SafeLog("bridge.log", $"[EVENT_ERROR] Failed to send event {eventName}: {ex.Message}");
        }
    }

    private async Task<object> InstallUpdateAsync(JsonElement payload)
    {
        string? downloadUrl = payload.TryGetProperty("downloadUrl", out var dProp) ? dProp.GetString() : null;
        string? requestedVersion = payload.TryGetProperty("version", out var vProp) ? vProp.GetString() : null;
        string? expectedSha256 = payload.TryGetProperty("sha256", out var sProp) ? sProp.GetString()?.Trim() : null;

        if (string.IsNullOrWhiteSpace(downloadUrl))
        {
            throw new ArgumentException("Download URL cannot be empty.");
        }

        if (!IsAllowedGitHubDownloadUrl(downloadUrl))
        {
            throw new ArgumentException("Update download URL must be an official HTTPS GitHub URL.");
        }

        if (string.IsNullOrWhiteSpace(expectedSha256) || !Sha256Pattern.IsMatch(expectedSha256))
        {
            throw new InvalidOperationException("Update verification failed: no valid SHA-256 checksum was provided. Refusing to install.");
        }

        expectedSha256 = expectedSha256.ToLowerInvariant();
        string targetVersion = !string.IsNullOrWhiteSpace(requestedVersion) && VersionPattern.IsMatch(requestedVersion)
            ? requestedVersion
            : "latest";

        App.SafeLog("bridge.log", $"[AUTO_UPDATE] Starting download for version {targetVersion} from: {downloadUrl}");

        _updateCts?.Dispose();
        _updateCts = new CancellationTokenSource();
        var ct = _updateCts.Token;

        _updatePauseEvent.Set();
        _isUpdatePaused = false;

        string tempFolder = Path.Combine(Path.GetTempPath(), "BrawlCraftUpdate_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(tempFolder);

        string fileName = Path.GetFileName(new Uri(downloadUrl).AbsolutePath);
        if (string.IsNullOrEmpty(fileName)
            || fileName is "." or ".."
            || fileName.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
        {
            fileName = "update.bin";
        }
        string downloadedFile = Path.Combine(tempFolder, fileName);

        SendEvent("updateProgress", new { status = "starting", progress = 0, isPaused = false, version = targetVersion });

        try
        {
            // Download asset from GitHub with stream progress tracking and pause support
            using (var client = new HttpClient())
            {
                client.Timeout = TimeSpan.FromMinutes(10);
                client.DefaultRequestHeaders.UserAgent.ParseAdd($"BrawlCraft-Updater/{CurrentVersion}");

                using var response = await client.GetAsync(downloadUrl, HttpCompletionOption.ResponseHeadersRead, ct);
                response.EnsureSuccessStatusCode();

                long totalBytes = response.Content.Headers.ContentLength ?? -1;
                if (totalBytes > MaxUpdateArchiveBytes)
                {
                    throw new InvalidOperationException("Update package is larger than the allowed limit.");
                }

                using var stream = await response.Content.ReadAsStreamAsync(ct);
                using var fs = new FileStream(downloadedFile, FileMode.Create, FileAccess.Write, FileShare.None);

                byte[] buffer = new byte[81920]; // 80 KB
                long totalRead = 0;
                int bytesRead;
                DateTime lastReportTime = DateTime.MinValue;

                while ((bytesRead = await stream.ReadAsync(buffer, 0, buffer.Length, ct)) > 0)
                {
                    // If paused, wait here until resumed or cancelled
                    while (_isUpdatePaused && !ct.IsCancellationRequested)
                    {
                        _updatePauseEvent.Wait(200);
                    }
                    ct.ThrowIfCancellationRequested();

                    await fs.WriteAsync(buffer, 0, bytesRead, ct);
                    totalRead += bytesRead;
                    if (totalRead > MaxUpdateArchiveBytes)
                    {
                        throw new InvalidOperationException("Update package is larger than the allowed limit.");
                    }

                    if ((DateTime.Now - lastReportTime).TotalMilliseconds > 120 || totalRead == totalBytes)
                    {
                        lastReportTime = DateTime.Now;
                        int percent = totalBytes > 0 ? (int)((totalRead * 100) / totalBytes) : 0;
                        SendEvent("updateProgress", new
                        {
                            status = "downloading",
                            progress = percent,
                            bytesDownloaded = totalRead,
                            totalBytes,
                            isPaused = _isUpdatePaused,
                            version = targetVersion
                        });
                    }
                }
            }

            // Refuse to execute anything that does not match the published checksum.
            string actualSha256 = BackupManager.ComputeSha256(downloadedFile);
            if (!string.Equals(actualSha256, expectedSha256, StringComparison.OrdinalIgnoreCase))
            {
                App.SafeLog("bridge.log", $"[AUTO_UPDATE] Checksum mismatch. expected={expectedSha256} actual={actualSha256}");
                throw new InvalidOperationException("Update verification failed: checksum mismatch. The download was discarded.");
            }

            App.SafeLog("bridge.log", $"[AUTO_UPDATE] Verified SHA-256 and downloaded update payload to {downloadedFile}");
            SendEvent("updateProgress", new { status = "installing", progress = 100, isPaused = false, version = targetVersion });

            string appDir = AppDomain.CurrentDomain.BaseDirectory.TrimEnd('\\', '/');
            string appExe = Process.GetCurrentProcess().MainModule?.FileName ?? Path.Combine(appDir, "BrawlCraft.exe");
            int currentPid = Environment.ProcessId;

            // If it's a Zip package, perform seamless in-place update using a helper batch script
            if (downloadedFile.EndsWith(".zip", StringComparison.OrdinalIgnoreCase))
            {
                string extractDir = Path.Combine(tempFolder, "extracted");
                Directory.CreateDirectory(extractDir);
                ExtractZipSafely(downloadedFile, extractDir);

                // Handle potential nested folder inside the zip
                string contentDir = extractDir;
                var subDirs = Directory.GetDirectories(extractDir);
                if (subDirs.Length == 1 && !File.Exists(Path.Combine(extractDir, "BrawlCraft.exe")) && File.Exists(Path.Combine(subDirs[0], "BrawlCraft.exe")))
                {
                    contentDir = subDirs[0];
                }

                if (!File.Exists(Path.Combine(contentDir, "BrawlCraft.exe")))
                {
                    throw new InvalidOperationException("Update package does not contain BrawlCraft.exe.");
                }

                string updaterScript = Path.Combine(tempFolder, "apply_update.bat");
                string scriptContent = $@"@echo off
chcp 65001 > nul
echo Updating BrawlCraft to version {targetVersion}...
echo Waiting for application (PID {currentPid}) to exit...
:wait_loop
tasklist /fi ""PID eq {currentPid}"" 2>nul | find ""{currentPid}"" >nul
if not errorlevel 1 (
    timeout /t 1 /nobreak >nul
    goto wait_loop
)

timeout /t 1 /nobreak >nul
echo Applying new files...
xcopy ""{contentDir}\*"" ""{appDir}\"" /E /H /Y /Q >nul

echo Relaunching BrawlCraft...
start """" ""{appExe}""

timeout /t 2 /nobreak >nul
(goto) 2>nul & rd /s /q ""{tempFolder}""
exit
";
                File.WriteAllText(updaterScript, scriptContent);

                App.SafeLog("bridge.log", "[AUTO_UPDATE] Launching silent in-place updater script and exiting current instance.");

                var psi = new ProcessStartInfo
                {
                    FileName = updaterScript,
                    CreateNoWindow = true,
                    UseShellExecute = false,
                    WindowStyle = ProcessWindowStyle.Hidden
                };
                Process.Start(psi);

                // Close the current application to free up locked binaries
                _onClose?.Invoke();

                return new { success = true, mode = "in-place", restarting = true };
            }
            else if (downloadedFile.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
            {
                // Execute setup installer silently
                App.SafeLog("bridge.log", "[AUTO_UPDATE] Launching setup installer silently...");
                var psi = new ProcessStartInfo
                {
                    FileName = downloadedFile,
                    Arguments = "/SILENT /CLOSEAPPLICATIONS /RESTARTAPPLICATIONS",
                    UseShellExecute = true
                };
                Process.Start(psi);

                _onClose?.Invoke();
                return new { success = true, mode = "installer", restarting = true };
            }

            throw new InvalidOperationException("Unsupported update archive format.");
        }
        catch
        {
            try
            {
                if (Directory.Exists(tempFolder)) Directory.Delete(tempFolder, recursive: true);
            }
            catch { }
            throw;
        }
    }

    /// <summary>
    /// Extracts a zip archive while blocking path traversal (Zip Slip) and zip bombs.
    /// </summary>
    private static void ExtractZipSafely(string archivePath, string destinationDir)
    {
        string destRoot = Path.GetFullPath(destinationDir);
        if (!destRoot.EndsWith(Path.DirectorySeparatorChar))
        {
            destRoot += Path.DirectorySeparatorChar;
        }

        using var archive = ZipFile.OpenRead(archivePath);
        if (archive.Entries.Count > MaxUpdateArchiveEntries)
        {
            throw new InvalidOperationException("Update archive contains too many entries.");
        }

        long totalExtracted = 0;
        foreach (var entry in archive.Entries)
        {
            totalExtracted += entry.Length;
            if (totalExtracted > MaxUpdateArchiveBytes)
            {
                throw new InvalidOperationException("Update archive expands beyond the allowed size.");
            }

            string targetPath = Path.GetFullPath(Path.Combine(destRoot, entry.FullName));
            if (!targetPath.StartsWith(destRoot, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidOperationException("Update archive contains an unsafe path.");
            }

            if (string.IsNullOrEmpty(entry.Name))
            {
                Directory.CreateDirectory(targetPath);
                continue;
            }

            string? targetDir = Path.GetDirectoryName(targetPath);
            if (!string.IsNullOrEmpty(targetDir))
            {
                Directory.CreateDirectory(targetDir);
            }
            entry.ExtractToFile(targetPath, overwrite: true);
        }
    }

    private bool OpenExternalUrl(JsonElement payload)
    {
        try
        {
            string? url = payload.TryGetProperty("url", out var uProp) ? uProp.GetString() : null;
            if (!string.IsNullOrEmpty(url) && (url.StartsWith("http://", StringComparison.OrdinalIgnoreCase) || url.StartsWith("https://", StringComparison.OrdinalIgnoreCase)))
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = url,
                    UseShellExecute = true
                });
                return true;
            }
        }
        catch (Exception ex)
        {
            App.SafeLog("bridge.log", $"[EXTERNAL_URL] Open failed: {ex.Message}");
        }
        return false;
    }

    private void SendResponse(string id, bool success, object? data, string? error)
    {
        var response = new
        {
            id,
            success,
            data,
            error
        };

        string json = JsonSerializer.Serialize(response, _jsonOptions);
        _webView.PostWebMessageAsJson(json);
    }
}
