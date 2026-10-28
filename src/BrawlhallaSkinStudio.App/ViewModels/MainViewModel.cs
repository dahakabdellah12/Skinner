using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows;
using BrawlhallaSkinStudio.Core.Engine;
using BrawlhallaSkinStudio.Core.Models;
using BrawlhallaSkinStudio.Core.Services;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace BrawlhallaSkinStudio.App.ViewModels;

public partial class MainViewModel : ObservableObject
{
    private readonly CatalogService _catalog = new();
    private BackupManager? _backupManager;
    private ModificationEngine? _modEngine;
    private PresetManager? _presetManager;

    [ObservableProperty]
    private string _currentTab = "Home";

    [ObservableProperty]
    private bool _isBusy;

    [ObservableProperty]
    private string _busyStatus = string.Empty;

    [ObservableProperty]
    private GameInstallation _installation = new();

    [ObservableProperty]
    private int _activeModCount;

    [ObservableProperty]
    private string _statusMessage = "Ready";

    [ObservableProperty]
    private bool _isStatusSuccess;

    [ObservableProperty]
    private bool _hasStatusNotification;

    // Search and Filters
    [ObservableProperty]
    private string _legendSearchText = string.Empty;

    [ObservableProperty]
    private string _costumeSearchText = string.Empty;

    [ObservableProperty]
    private string _selectedCostumeLegendFilter = "All";

    [ObservableProperty]
    private string _weaponSearchText = string.Empty;

    [ObservableProperty]
    private string _selectedWeaponTypeFilter = "All";

    // Customization Studio State
    [ObservableProperty]
    private LegendInfo? _selectedLegend;

    [ObservableProperty]
    private CostumeInfo? _selectedTargetCostume;

    [ObservableProperty]
    private CostumeInfo? _selectedReplacementCostume;

    [ObservableProperty]
    private WeaponSkinInfo? _selectedTargetWeapon1;

    [ObservableProperty]
    private WeaponSkinInfo? _selectedReplacementWeapon1;

    [ObservableProperty]
    private WeaponSkinInfo? _selectedTargetWeapon2;

    [ObservableProperty]
    private WeaponSkinInfo? _selectedReplacementWeapon2;

    [ObservableProperty]
    private ColorSchemeInfo? _selectedColorScheme;

    [ObservableProperty]
    private CompatibilityReport? _compatibilityReport;

    [ObservableProperty]
    private string _previewSvgContent = string.Empty;

    [ObservableProperty]
    private DiagnosticsReport? _diagnostics;

    [ObservableProperty]
    private ModificationResult? _lastModificationResult;

    // Collections
    public ObservableCollection<LegendInfo> FilteredLegends { get; } = [];
    public ObservableCollection<CostumeInfo> FilteredCostumes { get; } = [];
    public ObservableCollection<WeaponSkinInfo> FilteredWeaponSkins { get; } = [];
    public ObservableCollection<ColorSchemeInfo> ColorSchemes { get; } = [];
    public ObservableCollection<Preset> Presets { get; } = [];
    public ObservableCollection<BackupRecord> BackupRecords { get; } = [];
    public ObservableCollection<string> LegendFilterNames { get; } = ["All"];
    public ObservableCollection<string> WeaponTypeFilterNames { get; } = ["All"];
    public ObservableCollection<string> VerificationSteps { get; } = [];

    public MainViewModel()
    {
        _ = InitializeAsync();
    }

    [RelayCommand]
    public async Task InitializeAsync()
    {
        IsBusy = true;
        BusyStatus = "Detecting Brawlhalla installation...";
        try
        {
            await Task.Run(() =>
            {
                Installation = GameDetector.DetectGameInstallation();
                if (Installation.IsValid)
                {
                    _backupManager = new BackupManager(Installation.GamePath);
                    _modEngine = new ModificationEngine(Installation.GamePath, _backupManager);
                    _presetManager = new PresetManager(Path.Combine(Installation.GamePath, ".studio_presets"));

                    _catalog.LoadFromInstallation(Installation);
                }
            });

            if (Installation.IsValid && _catalog.IsLoaded)
            {
                PopulateCollections();
                RefreshActiveMods();
                RefreshPresets();
                RefreshBackups();
                RunDiagnostics();

                // Select default legend (Viking / Bödvar)
                SelectedLegend = _catalog.Legends.FirstOrDefault(l => l.HeroName.Equals("Viking", StringComparison.OrdinalIgnoreCase))
                                ?? _catalog.Legends.FirstOrDefault();

                ShowNotification($"Game connected: {Installation.GamePath} (Build: {Installation.BuildId})", true);
            }
            else
            {
                ShowNotification("Failed to detect or load Brawlhalla installation. Check diagnostics.", false);
            }
        }
        catch (Exception ex)
        {
            ShowNotification($"Initialization error: {ex.Message}", false);
        }
        finally
        {
            IsBusy = false;
        }
    }

    [RelayCommand]
    public void SwitchTab(string tabName)
    {
        CurrentTab = tabName;
        if (tabName == "Diagnostics")
        {
            RunDiagnostics();
        }
        else if (tabName == "ModManager")
        {
            RefreshActiveMods();
            RefreshBackups();
        }
        else if (tabName == "Presets")
        {
            RefreshPresets();
        }
        else if (tabName == "Verification")
        {
            UpdateVerificationSteps();
        }
    }

    [RelayCommand]
    public void SelectLegendForStudio(LegendInfo legend)
    {
        SelectedLegend = legend;
        SwitchTab("Studio");
    }

    [RelayCommand]
    public void SelectCostumeForStudio(CostumeInfo costume)
    {
        var owner = _catalog.Legends.FirstOrDefault(l => l.HeroName.Equals(costume.OwnerHero, StringComparison.OrdinalIgnoreCase));
        if (owner != null)
        {
            SelectedLegend = owner;
            SelectedReplacementCostume = costume;
        }
        SwitchTab("Studio");
    }

    [RelayCommand]
    public void SelectWeaponForStudio(WeaponSkinInfo weapon)
    {
        if (SelectedLegend != null)
        {
            if (string.Equals(weapon.BaseWeapon, SelectedLegend.Weapon1, StringComparison.OrdinalIgnoreCase))
            {
                SelectedReplacementWeapon1 = weapon;
            }
            else if (string.Equals(weapon.BaseWeapon, SelectedLegend.Weapon2, StringComparison.OrdinalIgnoreCase))
            {
                SelectedReplacementWeapon2 = weapon;
            }
        }
        SwitchTab("Studio");
    }

    [RelayCommand]
    public async Task ApplyCurrentModificationAsync()
    {
        if (_modEngine == null || SelectedLegend == null || SelectedTargetCostume == null)
        {
            ShowNotification("Cannot apply modification: Incomplete selection.", false);
            return;
        }

        var request = new ModificationRequest
        {
            TargetLegend = SelectedLegend,
            TargetCostume = SelectedTargetCostume,
            ReplacementCostume = SelectedReplacementCostume,
            TargetWeapon1 = SelectedTargetWeapon1,
            ReplacementWeapon1 = SelectedReplacementWeapon1,
            TargetWeapon2 = SelectedTargetWeapon2,
            ReplacementWeapon2 = SelectedReplacementWeapon2,
            SelectedColorScheme = SelectedColorScheme
        };

        IsBusy = true;
        BusyStatus = "Applying cosmetic modification...";
        try
        {
            ModificationResult? result = null;
            await Task.Run(() =>
            {
                result = _modEngine.ApplyModification(request);
            });

            LastModificationResult = result;
            if (result != null && result.Success)
            {
                ShowNotification(result.Message, true);
                RefreshActiveMods();
                RefreshBackups();
                UpdateVerificationSteps();
            }
            else
            {
                ShowNotification(result?.Message ?? "Modification failed.", false);
            }
        }
        catch (Exception ex)
        {
            ShowNotification($"Failed to apply modification: {ex.Message}", false);
        }
        finally
        {
            IsBusy = false;
        }
    }

    [RelayCommand]
    public void RestoreTargetCostume()
    {
        if (_backupManager == null || SelectedTargetCostume == null) return;

        bool restored = _backupManager.RestoreFile(SelectedTargetCostume.SwfFile);
        if (restored)
        {
            ShowNotification($"Restored original '{SelectedTargetCostume.SwfFile}'.", true);
            RefreshActiveMods();
            RefreshBackups();
        }
        else
        {
            ShowNotification($"No backup found to restore for '{SelectedTargetCostume.SwfFile}'.", false);
        }
    }

    [RelayCommand]
    public void RestoreAllFiles()
    {
        if (_backupManager == null) return;

        int restored = _backupManager.RestoreAll();
        ShowNotification($"Restored {restored} game asset file(s) to pristine condition.", true);
        RefreshActiveMods();
        RefreshBackups();
    }

    [RelayCommand]
    public void LaunchGame()
    {
        if (!Installation.IsValid) return;

        bool launched = GameLauncher.LaunchGame(Installation.ExePath, preferSteam: true);
        if (launched)
        {
            ShowNotification("Brawlhalla launched! Check appearance in training mode.", true);
            SwitchTab("Verification");
        }
        else
        {
            ShowNotification("Failed to launch Brawlhalla.", false);
        }
    }

    [RelayCommand]
    public void SaveCurrentAsPreset()
    {
        if (_presetManager == null || SelectedLegend == null) return;

        string name = $"{SelectedLegend.DisplayName} - {SelectedReplacementCostume?.DisplayName ?? "Default"}";
        var preset = new Preset
        {
            Name = name,
            Description = $"Custom preset for {SelectedLegend.DisplayName}",
            LegendName = SelectedLegend.HeroName,
            TargetCostumeName = SelectedTargetCostume?.CostumeName ?? SelectedLegend.HeroName,
            ReplacementCostumeName = SelectedReplacementCostume?.CostumeName ?? "",
            Weapon1Type = SelectedLegend.Weapon1,
            Weapon1SkinName = SelectedReplacementWeapon1?.WeaponSkinName ?? "",
            Weapon2Type = SelectedLegend.Weapon2,
            Weapon2SkinName = SelectedReplacementWeapon2?.WeaponSkinName ?? "",
            ColorSchemeName = SelectedColorScheme?.SchemeName ?? ""
        };

        _presetManager.SavePreset(preset);
        RefreshPresets();
        ShowNotification($"Preset '{name}' saved successfully.", true);
    }

    [RelayCommand]
    public async Task ApplyPresetAsync(Preset preset)
    {
        var legend = _catalog.Legends.FirstOrDefault(l => l.HeroName.Equals(preset.LegendName, StringComparison.OrdinalIgnoreCase));
        if (legend == null)
        {
            ShowNotification($"Legend '{preset.LegendName}' not found in catalog.", false);
            return;
        }

        SelectedLegend = legend;
        SelectedReplacementCostume = legend.Costumes.FirstOrDefault(c => c.CostumeName.Equals(preset.ReplacementCostumeName, StringComparison.OrdinalIgnoreCase));

        if (!string.IsNullOrEmpty(preset.Weapon1SkinName))
        {
            SelectedReplacementWeapon1 = _catalog.WeaponSkins.FirstOrDefault(w => w.WeaponSkinName.Equals(preset.Weapon1SkinName, StringComparison.OrdinalIgnoreCase));
        }

        if (!string.IsNullOrEmpty(preset.Weapon2SkinName))
        {
            SelectedReplacementWeapon2 = _catalog.WeaponSkins.FirstOrDefault(w => w.WeaponSkinName.Equals(preset.Weapon2SkinName, StringComparison.OrdinalIgnoreCase));
        }

        if (!string.IsNullOrEmpty(preset.ColorSchemeName))
        {
            SelectedColorScheme = _catalog.ColorSchemes.FirstOrDefault(c => c.SchemeName.Equals(preset.ColorSchemeName, StringComparison.OrdinalIgnoreCase));
        }

        await ApplyCurrentModificationAsync();
    }

    [RelayCommand]
    public void DeletePreset(string presetId)
    {
        if (_presetManager?.DeletePreset(presetId) == true)
        {
            RefreshPresets();
            ShowNotification("Preset deleted.", true);
        }
    }

    private void PopulateCollections()
    {
        FilteredLegends.Clear();
        foreach (var l in _catalog.Legends) FilteredLegends.Add(l);

        FilteredCostumes.Clear();
        foreach (var c in _catalog.Costumes) FilteredCostumes.Add(c);

        FilteredWeaponSkins.Clear();
        foreach (var w in _catalog.WeaponSkins) FilteredWeaponSkins.Add(w);

        ColorSchemes.Clear();
        foreach (var cs in _catalog.ColorSchemes) ColorSchemes.Add(cs);

        LegendFilterNames.Clear();
        LegendFilterNames.Add("All");
        foreach (var l in _catalog.Legends) LegendFilterNames.Add(l.DisplayName);

        WeaponTypeFilterNames.Clear();
        WeaponTypeFilterNames.Add("All");
        foreach (var wt in _catalog.WeaponSkinsByType.Keys.OrderBy(k => k))
            WeaponTypeFilterNames.Add(wt);
    }

    partial void OnLegendSearchTextChanged(string value) => FilterLegends();
    partial void OnCostumeSearchTextChanged(string value) => FilterCostumes();
    partial void OnSelectedCostumeLegendFilterChanged(string value) => FilterCostumes();
    partial void OnWeaponSearchTextChanged(string value) => FilterWeaponSkins();
    partial void OnSelectedWeaponTypeFilterChanged(string value) => FilterWeaponSkins();

    partial void OnSelectedLegendChanged(LegendInfo? value)
    {
        if (value == null) return;

        // Default target costume to default skin
        SelectedTargetCostume = value.Costumes.FirstOrDefault(c => c.IsDefault) ?? value.Costumes.FirstOrDefault();
        SelectedReplacementCostume = SelectedTargetCostume;

        // Default weapons
        SelectedTargetWeapon1 = _catalog.WeaponSkins.FirstOrDefault(w => w.OwnerHero.Equals(value.HeroName, StringComparison.OrdinalIgnoreCase) && w.BaseWeapon.Equals(value.Weapon1, StringComparison.OrdinalIgnoreCase))
                                ?? _catalog.WeaponSkins.FirstOrDefault(w => w.BaseWeapon.Equals(value.Weapon1, StringComparison.OrdinalIgnoreCase));

        SelectedReplacementWeapon1 = SelectedTargetWeapon1;

        SelectedTargetWeapon2 = _catalog.WeaponSkins.FirstOrDefault(w => w.OwnerHero.Equals(value.HeroName, StringComparison.OrdinalIgnoreCase) && w.BaseWeapon.Equals(value.Weapon2, StringComparison.OrdinalIgnoreCase))
                                ?? _catalog.WeaponSkins.FirstOrDefault(w => w.BaseWeapon.Equals(value.Weapon2, StringComparison.OrdinalIgnoreCase));

        SelectedReplacementWeapon2 = SelectedTargetWeapon2;

        UpdateCompatibility();
    }

    partial void OnSelectedReplacementCostumeChanged(CostumeInfo? value) => UpdateCompatibility();
    partial void OnSelectedReplacementWeapon1Changed(WeaponSkinInfo? value) => UpdateCompatibility();
    partial void OnSelectedReplacementWeapon2Changed(WeaponSkinInfo? value) => UpdateCompatibility();
    partial void OnSelectedColorSchemeChanged(ColorSchemeInfo? value) => UpdateCompatibility();

    private void UpdateCompatibility()
    {
        if (_modEngine == null || SelectedLegend == null || SelectedTargetCostume == null)
        {
            CompatibilityReport = null;
            return;
        }

        var req = new ModificationRequest
        {
            TargetLegend = SelectedLegend,
            TargetCostume = SelectedTargetCostume,
            ReplacementCostume = SelectedReplacementCostume,
            TargetWeapon1 = SelectedTargetWeapon1,
            ReplacementWeapon1 = SelectedReplacementWeapon1,
            TargetWeapon2 = SelectedTargetWeapon2,
            ReplacementWeapon2 = SelectedReplacementWeapon2,
            SelectedColorScheme = SelectedColorScheme
        };

        CompatibilityReport = _modEngine.ValidateModification(req);
    }

    private void FilterLegends()
    {
        FilteredLegends.Clear();
        var q = _catalog.Legends.AsEnumerable();
        if (!string.IsNullOrWhiteSpace(LegendSearchText))
        {
            q = q.Where(l => l.DisplayName.Contains(LegendSearchText, StringComparison.OrdinalIgnoreCase)
                          || l.HeroName.Contains(LegendSearchText, StringComparison.OrdinalIgnoreCase)
                          || l.Weapon1.Contains(LegendSearchText, StringComparison.OrdinalIgnoreCase)
                          || l.Weapon2.Contains(LegendSearchText, StringComparison.OrdinalIgnoreCase));
        }
        foreach (var item in q) FilteredLegends.Add(item);
    }

    private void FilterCostumes()
    {
        FilteredCostumes.Clear();
        var q = _catalog.Costumes.AsEnumerable();
        if (!string.IsNullOrWhiteSpace(CostumeSearchText))
        {
            q = q.Where(c => c.DisplayName.Contains(CostumeSearchText, StringComparison.OrdinalIgnoreCase)
                          || c.CostumeName.Contains(CostumeSearchText, StringComparison.OrdinalIgnoreCase)
                          || c.OwnerHero.Contains(CostumeSearchText, StringComparison.OrdinalIgnoreCase));
        }
        if (SelectedCostumeLegendFilter != "All")
        {
            var matchLegend = _catalog.Legends.FirstOrDefault(l => l.DisplayName.Equals(SelectedCostumeLegendFilter, StringComparison.OrdinalIgnoreCase));
            if (matchLegend != null)
            {
                q = q.Where(c => c.OwnerHero.Equals(matchLegend.HeroName, StringComparison.OrdinalIgnoreCase));
            }
        }
        foreach (var item in q) FilteredCostumes.Add(item);
    }

    private void FilterWeaponSkins()
    {
        FilteredWeaponSkins.Clear();
        var q = _catalog.WeaponSkins.AsEnumerable();
        if (!string.IsNullOrWhiteSpace(WeaponSearchText))
        {
            q = q.Where(w => w.DisplayName.Contains(WeaponSearchText, StringComparison.OrdinalIgnoreCase)
                          || w.WeaponSkinName.Contains(WeaponSearchText, StringComparison.OrdinalIgnoreCase)
                          || w.BaseWeapon.Contains(WeaponSearchText, StringComparison.OrdinalIgnoreCase));
        }
        if (SelectedWeaponTypeFilter != "All")
        {
            q = q.Where(w => w.BaseWeapon.Equals(SelectedWeaponTypeFilter, StringComparison.OrdinalIgnoreCase));
        }
        foreach (var item in q) FilteredWeaponSkins.Add(item);
    }

    private void RefreshActiveMods()
    {
        if (_backupManager == null)
        {
            ActiveModCount = 0;
            return;
        }

        int count = 0;
        foreach (var record in _backupManager.Records)
        {
            foreach (var key in record.FileOriginalHashes.Keys)
            {
                if (_backupManager.IsFileModified(key)) count++;
            }
        }
        ActiveModCount = count;
    }

    private void RefreshPresets()
    {
        Presets.Clear();
        if (_presetManager != null)
        {
            foreach (var p in _presetManager.Presets) Presets.Add(p);
        }
    }

    private void RefreshBackups()
    {
        BackupRecords.Clear();
        if (_backupManager != null)
        {
            foreach (var r in _backupManager.Records) BackupRecords.Add(r);
        }
    }

    private void RunDiagnostics()
    {
        if (_backupManager != null)
        {
            Diagnostics = DiagnosticsService.RunDiagnostics(Installation, _catalog, _backupManager);
        }
    }

    private void UpdateVerificationSteps()
    {
        VerificationSteps.Clear();
        string legend = SelectedLegend?.DisplayName ?? "Selected Legend";
        string costume = SelectedReplacementCostume?.DisplayName ?? "Selected Costume";
        string w1 = SelectedReplacementWeapon1?.DisplayName ?? "Weapon 1";
        string w2 = SelectedReplacementWeapon2?.DisplayName ?? "Weapon 2";

        var steps = GameLauncher.GetVerificationChecklist(legend, costume, w1, w2);
        foreach (var step in steps) VerificationSteps.Add(step);
    }

    private void ShowNotification(string msg, bool isSuccess)
    {
        StatusMessage = msg;
        IsStatusSuccess = isSuccess;
        HasStatusNotification = true;
    }
}
