using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using BrawlhallaSkinStudio.Core.Models;

namespace BrawlhallaSkinStudio.Core.Services;

public class PresetManager
{
    private readonly string _presetsFilePath;
    private readonly List<Preset> _presets = [];

    public PresetManager(string storageDirectory)
    {
        if (!Directory.Exists(storageDirectory))
        {
            Directory.CreateDirectory(storageDirectory);
        }

        _presetsFilePath = Path.Combine(storageDirectory, "presets.json");
        LoadPresets();
    }

    public IReadOnlyList<Preset> Presets => _presets.AsReadOnly();

    public void SavePreset(Preset preset)
    {
        int existingIdx = _presets.FindIndex(p => p.Id == preset.Id);
        if (existingIdx >= 0)
        {
            _presets[existingIdx] = preset;
        }
        else
        {
            _presets.Add(preset);
        }
        SaveToFile();
    }

    public bool DeletePreset(string presetId)
    {
        int removed = _presets.RemoveAll(p => p.Id == presetId);
        if (removed > 0)
        {
            SaveToFile();
            return true;
        }
        return false;
    }

    public Preset DuplicatePreset(string presetId, string newName)
    {
        var existing = _presets.Find(p => p.Id == presetId);
        if (existing == null)
            throw new KeyNotFoundException($"Preset with ID {presetId} not found.");

        var copy = new Preset
        {
            Id = Guid.NewGuid().ToString("N"),
            Name = newName,
            Description = existing.Description,
            CreatedAt = DateTime.UtcNow,
            LegendName = existing.LegendName,
            TargetCostumeName = existing.TargetCostumeName,
            ReplacementCostumeName = existing.ReplacementCostumeName,
            Weapon1Type = existing.Weapon1Type,
            Weapon1SkinName = existing.Weapon1SkinName,
            Weapon2Type = existing.Weapon2Type,
            Weapon2SkinName = existing.Weapon2SkinName,
            ColorSchemeName = existing.ColorSchemeName,
            CustomPalette = new Dictionary<string, uint>(existing.CustomPalette)
        };

        _presets.Add(copy);
        SaveToFile();
        return copy;
    }

    public string ExportPresetJson(string presetId)
    {
        var preset = _presets.Find(p => p.Id == presetId);
        if (preset == null)
            throw new KeyNotFoundException($"Preset with ID {presetId} not found.");

        return JsonSerializer.Serialize(preset, new JsonSerializerOptions { WriteIndented = true });
    }

    public Preset ImportPresetJson(string json)
    {
        var imported = JsonSerializer.Deserialize<Preset>(json);
        if (imported == null)
            throw new InvalidDataException("Invalid preset JSON.");

        imported.Id = Guid.NewGuid().ToString("N");
        imported.CreatedAt = DateTime.UtcNow;
        _presets.Add(imported);
        SaveToFile();
        return imported;
    }

    private void LoadPresets()
    {
        if (File.Exists(_presetsFilePath))
        {
            try
            {
                string json = File.ReadAllText(_presetsFilePath);
                var loaded = JsonSerializer.Deserialize<List<Preset>>(json);
                if (loaded != null)
                {
                    _presets.Clear();
                    _presets.AddRange(loaded);
                }
            }
            catch { }
        }
    }

    private void SaveToFile()
    {
        try
        {
            string json = JsonSerializer.Serialize(_presets, new JsonSerializerOptions { WriteIndented = true });
            File.WriteAllText(_presetsFilePath, json);
        }
        catch { }
    }
}
