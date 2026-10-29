using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using BrawlhallaSkinStudio.Core.Models;
using BrawlhallaSkinStudio.Core.Services;
using SwfLib;
using SwfLib.Data;
using SwfLib.Gradients;
using SwfLib.Shapes.FillStyles;
using SwfLib.Tags;
using SwfLib.Tags.ControlTags;
using SwfLib.Tags.DisplayListTags;
using SwfLib.Tags.ShapeTags;

namespace BrawlhallaSkinStudio.Core.Engine;

public class ModificationEngine
{
    private readonly string _gamePath;
    private readonly BackupManager _backupManager;
    private readonly CatalogService? _catalog;

    public ModificationEngine(string gamePath, BackupManager backupManager, CatalogService? catalog = null)
    {
        _gamePath = gamePath;
        _backupManager = backupManager;
        _catalog = catalog;
    }

    public CompatibilityReport ValidateModification(ModificationRequest request)
    {
        var report = new CompatibilityReport();

        if (request.TargetCostume == null)
        {
            report.IsCompatible = false;
            report.Level = CompatibilityLevel.Incompatible;
            report.Diagnostics.Add("Target costume is not selected.");
            return report;
        }

        string targetSwfPath = Path.Combine(_gamePath, request.TargetCostume.SwfFile);
        if (!File.Exists(targetSwfPath))
        {
            report.IsCompatible = false;
            report.Level = CompatibilityLevel.Incompatible;
            report.Diagnostics.Add($"Target SWF file '{request.TargetCostume.SwfFile}' not found in game directory.");
            return report;
        }

        // Costume Compatibility
        if (request.ReplacementCostume != null)
        {
            string replacementSwfPath = Path.Combine(_gamePath, request.ReplacementCostume.SwfFile);
            if (!File.Exists(replacementSwfPath))
            {
                report.IsCompatible = false;
                report.Level = CompatibilityLevel.Incompatible;
                report.Diagnostics.Add($"Replacement SWF file '{request.ReplacementCostume.SwfFile}' not found in game directory.");
                return report;
            }

            bool sameSwf = string.Equals(request.TargetCostume.SwfFile, request.ReplacementCostume.SwfFile, StringComparison.OrdinalIgnoreCase);
            if (sameSwf)
            {
                report.Level = CompatibilityLevel.Compatible;
                report.RequiresCrossSwfImport = false;
                report.Diagnostics.Add($"Native intra-SWF replacement ({request.TargetCostume.CostumeName} -> {request.ReplacementCostume.CostumeName}). Complete rig fidelity guaranteed.");
            }
            else
            {
                report.Level = CompatibilityLevel.CrossLegendExperimental;
                report.RequiresCrossSwfImport = true;
                report.Diagnostics.Add($"Cross-SWF porting ({request.TargetCostume.SwfFile} <- {request.ReplacementCostume.SwfFile}). Visual assets and vector definitions will be ported into target rig.");
            }
        }

        // Weapon Compatibility
        ValidateWeapon(request.TargetWeapon1, request.ReplacementWeapon1, "Weapon 1", report);
        ValidateWeapon(request.TargetWeapon2, request.ReplacementWeapon2, "Weapon 2", report);

        return report;
    }

    private void ValidateWeapon(WeaponSkinInfo? target, WeaponSkinInfo? replacement, string slotName, CompatibilityReport report)
    {
        if (target == null || replacement == null) return;

        if (!string.Equals(target.BaseWeapon, replacement.BaseWeapon, StringComparison.OrdinalIgnoreCase))
        {
            report.IsCompatible = false;
            report.Level = CompatibilityLevel.Incompatible;
            report.Diagnostics.Add($"{slotName} mismatch: Cannot replace a '{target.BaseWeapon}' with a '{replacement.BaseWeapon}'. Base weapon categories must match.");
            return;
        }

        string targetSwf = Path.Combine(_gamePath, target.SwfFile);
        string replSwf = Path.Combine(_gamePath, replacement.SwfFile);

        if (!File.Exists(targetSwf))
        {
            report.IsCompatible = false;
            report.Level = CompatibilityLevel.Incompatible;
            report.Diagnostics.Add($"{slotName} target SWF '{target.SwfFile}' not found in game directory.");
            return;
        }
        if (!File.Exists(replSwf))
        {
            report.IsCompatible = false;
            report.Level = CompatibilityLevel.Incompatible;
            report.Diagnostics.Add($"{slotName} replacement SWF '{replacement.SwfFile}' not found in game directory.");
            return;
        }
    }

    public ModificationResult ApplyModification(ModificationRequest request)
    {
        var result = new ModificationResult();
        var report = ValidateModification(request);

        if (!report.IsCompatible)
        {
            result.Success = false;
            result.Message = "Modification validation failed: " + string.Join("; ", report.Diagnostics);
            result.OperationLog.AddRange(report.Diagnostics);
            return result;
        }

        bool hasColorScheme = (request.SelectedColorScheme != null && request.SelectedColorScheme.SwapColors.Count > 0) ||
                              (request.CustomPalette != null && request.CustomPalette.Count > 0);

        // Self-heal: ensure no shared base sound banks remain modified from legacy versions
        foreach (var baseBank in BaseWeaponSoundBanks)
        {
            string relBnk = Path.Combine("audio", "pc", $"{baseBank}.bnk");
            if (_backupManager.IsFileModified(relBnk))
            {
                _backupManager.RestoreFile(relBnk);
            }
        }

        // Identify all target SWF files that will be modified
        var targetFilesToModify = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        if (request.TargetCostume != null && !string.IsNullOrWhiteSpace(request.TargetCostume.SwfFile))
        {
            string targetCostumeSwf = Path.Combine(_gamePath, request.TargetCostume.SwfFile);
            targetFilesToModify.Add(targetCostumeSwf);
        }

        var replW1 = ResolveFullWeaponSkin(request.ReplacementWeapon1);
        if (request.TargetWeapon1 != null && !string.IsNullOrWhiteSpace(request.TargetWeapon1.SwfFile))
        {
            bool needsW1 = hasColorScheme || (replW1 != null &&
                !string.Equals(request.TargetWeapon1.WeaponSkinName, replW1.WeaponSkinName, StringComparison.OrdinalIgnoreCase));
            if (needsW1)
            {
                targetFilesToModify.Add(Path.Combine(_gamePath, request.TargetWeapon1.SwfFile));

                if (replW1 != null && ShouldApplyWeaponAudio(replW1, request.TargetWeapon1))
                {
                    string targetBnk = GetWeaponSoundBankPath(request.TargetWeapon1);
                    string replBnk = GetWeaponSoundBankPath(replW1);
                    if (File.Exists(targetBnk) && File.Exists(replBnk))
                    {
                        targetFilesToModify.Add(targetBnk);
                    }
                }
            }
        }
        var replW2 = ResolveFullWeaponSkin(request.ReplacementWeapon2);
        if (request.TargetWeapon2 != null && !string.IsNullOrWhiteSpace(request.TargetWeapon2.SwfFile))
        {
            bool needsW2 = hasColorScheme || (replW2 != null &&
                !string.Equals(request.TargetWeapon2.WeaponSkinName, replW2.WeaponSkinName, StringComparison.OrdinalIgnoreCase));
            if (needsW2)
            {
                targetFilesToModify.Add(Path.Combine(_gamePath, request.TargetWeapon2.SwfFile));

                if (replW2 != null && ShouldApplyWeaponAudio(replW2, request.TargetWeapon2))
                {
                    string targetBnk = GetWeaponSoundBankPath(request.TargetWeapon2);
                    string replBnk = GetWeaponSoundBankPath(replW2);
                    if (File.Exists(targetBnk) && File.Exists(replBnk))
                    {
                        targetFilesToModify.Add(targetBnk);
                    }
                }
            }
        }

        // Record pre-modification hashes
        foreach (var file in targetFilesToModify)
        {
            if (File.Exists(file))
            {
                result.HashesBefore[Path.GetFileName(file)] = BackupManager.ComputeSha256(file);
            }
        }

        // Create backup
        result.BackupId = _backupManager.CreateBackup(targetFilesToModify, $"Apply: {request.TargetLegend.HeroName} -> {request.ReplacementCostume?.CostumeName ?? "Default"}");
        result.OperationLog.Add($"Created backup {result.BackupId} for {targetFilesToModify.Count} files.");

        try
        {
            // Process Costume Modification
            if (request.ReplacementCostume != null || hasColorScheme)
            {
                ApplyCostumeModification(request, result);
            }

            // Process Weapon 1 Modification
            if (request.TargetWeapon1 != null)
            {
                bool needsW1 = hasColorScheme || (replW1 != null &&
                    !string.Equals(request.TargetWeapon1.WeaponSkinName, replW1.WeaponSkinName, StringComparison.OrdinalIgnoreCase));
                if (needsW1)
                {
                    ApplyWeaponModification(request.TargetWeapon1, replW1 ?? request.TargetWeapon1, request, result);

                    if (replW1 != null && ShouldApplyWeaponAudio(replW1, request.TargetWeapon1))
                    {
                        string targetBnk = GetWeaponSoundBankPath(request.TargetWeapon1);
                        string replBnk = GetWeaponSoundBankPath(replW1);
                        if (File.Exists(targetBnk) && File.Exists(replBnk))
                        {
                            ApplyWeaponAudioModification(request.TargetWeapon1, replW1, targetBnk, replBnk, result);
                        }
                    }
                }
            }

            // Process Weapon 2 Modification
            if (request.TargetWeapon2 != null)
            {
                bool needsW2 = hasColorScheme || (replW2 != null &&
                    !string.Equals(request.TargetWeapon2.WeaponSkinName, replW2.WeaponSkinName, StringComparison.OrdinalIgnoreCase));
                if (needsW2)
                {
                    ApplyWeaponModification(request.TargetWeapon2, replW2 ?? request.TargetWeapon2, request, result);

                    if (replW2 != null && ShouldApplyWeaponAudio(replW2, request.TargetWeapon2))
                    {
                        string targetBnk = GetWeaponSoundBankPath(request.TargetWeapon2);
                        string replBnk = GetWeaponSoundBankPath(replW2);
                        if (File.Exists(targetBnk) && File.Exists(replBnk))
                        {
                            ApplyWeaponAudioModification(request.TargetWeapon2, replW2, targetBnk, replBnk, result);
                        }
                    }
                }
            }

            // Verify written files
            foreach (var file in targetFilesToModify)
            {
                if (File.Exists(file))
                {
                    if (file.EndsWith(".swf", StringComparison.OrdinalIgnoreCase))
                    {
                        using (var fs = new FileStream(file, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                        {
                            var verifiedSwf = SwfFile.ReadFrom(fs);
                            if (verifiedSwf == null || verifiedSwf.Tags.Count == 0)
                            {
                                throw new InvalidDataException($"Integrity verification failed for '{Path.GetFileName(file)}'.");
                            }
                        }
                    }
                    else if (file.EndsWith(".bnk", StringComparison.OrdinalIgnoreCase))
                    {
                        byte[] header = new byte[4];
                        using (var fs = new FileStream(file, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                        {
                            if (fs.Read(header, 0, 4) < 4 || System.Text.Encoding.ASCII.GetString(header) != "BKHD")
                            {
                                throw new InvalidDataException($"Integrity verification failed for sound bank '{Path.GetFileName(file)}'.");
                            }
                        }
                    }

                    string newHash = BackupManager.ComputeSha256(file);
                    result.HashesAfter[Path.GetFileName(file)] = newHash;
                    result.ModifiedFiles.Add(Path.GetRelativePath(_gamePath, file));
                    result.OperationLog.Add($"Verified '{Path.GetFileName(file)}' (SHA-256: {newHash[..12]}...)");
                }
            }

            result.Success = true;
            result.Message = $"Successfully applied cosmetic modifications to {result.ModifiedFiles.Count} game asset(s).";
            return result;
        }
        catch (Exception ex)
        {
            result.Success = false;
            result.Message = $"Modification failed during execution: {ex.Message}";
            result.OperationLog.Add($"ERROR: {ex}");

            // Attempt automatic restoration on failure
            result.OperationLog.Add("Rolling back modified files from backup...");
            foreach (var file in targetFilesToModify)
            {
                string rel = Path.GetRelativePath(_gamePath, file);
                _backupManager.RestoreFile(rel);
            }

            return result;
        }
    }

    private void ApplyCostumeModification(ModificationRequest request, ModificationResult result)
    {
        var targetCostume = request.TargetCostume;
        var replCostume = request.ReplacementCostume ?? targetCostume;

        string targetSwfPath = Path.Combine(_gamePath, targetCostume.SwfFile);
        string replSwfPath = Path.Combine(_gamePath, replCostume.SwfFile);

        if (!File.Exists(targetSwfPath))
            throw new FileNotFoundException($"Cannot find target game asset '{targetCostume.SwfFile}'. Please verify your game installation.", targetSwfPath);
        if (!File.Exists(replSwfPath))
            throw new FileNotFoundException($"Cannot find replacement game asset '{replCostume.SwfFile}'. Please verify your game installation.", replSwfPath);

        bool sameSwf = string.Equals(targetCostume.SwfFile, replCostume.SwfFile, StringComparison.OrdinalIgnoreCase);

        SwfFile targetSwf;
        using (var fs = new FileStream(targetSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            targetSwf = SwfFile.ReadFrom(fs);
        }

        var targetSymClass = targetSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
        if (targetSymClass == null)
            throw new InvalidDataException($"SymbolClassTag not found in '{targetCostume.SwfFile}'.");

        string targetSuffix = targetCostume.SymbolSuffix;
        string replSuffix = replCostume.SymbolSuffix;

        // Build Palette Swap Map
        var colorMap = BuildColorSwapMap(request);

        if (sameSwf)
        {
            if (!string.Equals(targetSuffix, replSuffix, StringComparison.OrdinalIgnoreCase))
            {
                // Same SWF: Remap symbol references
                var replRefs = targetSymClass.References
                    .Where(r => r.SymbolName.EndsWith("_" + replSuffix, StringComparison.OrdinalIgnoreCase))
                    .ToList();

                var replMap = new Dictionary<string, ushort>(StringComparer.OrdinalIgnoreCase);
                foreach (var r in replRefs)
                {
                    string prefix = r.SymbolName[..r.SymbolName.LastIndexOf('_')];
                    replMap[prefix] = r.SymbolID;
                }

                int remappedCount = 0;
                var targetRefs = targetSymClass.References
                    .Where(r => r.SymbolName.EndsWith("_" + targetSuffix, StringComparison.OrdinalIgnoreCase))
                    .ToList();

                foreach (var tr in targetRefs)
                {
                    string prefix = tr.SymbolName[..tr.SymbolName.LastIndexOf('_')];
                    if (replMap.TryGetValue(prefix, out ushort replacementId))
                    {
                        tr.SymbolID = replacementId;
                        remappedCount++;
                    }
                }

                // Also add any replacement symbols that target was missing
                foreach (var kvp in replMap)
                {
                    string expectedTargetSym = $"{kvp.Key}_{targetSuffix}";
                    if (!targetSymClass.References.Any(r => r.SymbolName.Equals(expectedTargetSym, StringComparison.OrdinalIgnoreCase)))
                    {
                        targetSymClass.References.Add(new SwfSymbolReference
                        {
                            SymbolID = kvp.Value,
                            SymbolName = expectedTargetSym
                        });
                        remappedCount++;
                    }
                }

                result.OperationLog.Add($"Remapped {remappedCount} symbols for costume '{targetCostume.CostumeName}' -> '{replCostume.CostumeName}'.");
            }
        }
        else
        {
            // Cross-SWF: Port sprites and shapes from replSwf into targetSwf
            SwfFile replSwf;
            using (var fs = new FileStream(replSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            {
                replSwf = SwfFile.ReadFrom(fs);
            }

            var replSymClass = replSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
            if (replSymClass == null)
                throw new InvalidDataException($"SymbolClassTag not found in source '{replCostume.SwfFile}'.");

            var sourceRefs = replSymClass.References
                .Where(r => r.SymbolName.EndsWith("_" + replSuffix, StringComparison.OrdinalIgnoreCase))
                .ToList();

            PortAndRemapSymbols(replSwf, targetSwf, sourceRefs, targetSuffix, colorMap, result);
        }

        // Apply Color Scheme palette modifications to shapes if requested
        if (colorMap.Count > 0)
        {
            ApplyColorMapToSwf(targetSwf, colorMap, result);
        }

        // Save atomically
        SaveSwfAtomically(targetSwf, targetSwfPath);
    }

    private void ApplyWeaponModification(WeaponSkinInfo target, WeaponSkinInfo replacement, ModificationRequest request, ModificationResult result)
    {
        string targetSwfPath = Path.Combine(_gamePath, target.SwfFile);
        string replSwfPath = Path.Combine(_gamePath, replacement.SwfFile);

        if (!File.Exists(targetSwfPath))
            throw new FileNotFoundException($"Cannot find target weapon asset '{target.SwfFile}'.", targetSwfPath);
        if (!File.Exists(replSwfPath))
            throw new FileNotFoundException($"Cannot find replacement weapon asset '{replacement.SwfFile}'.", replSwfPath);

        bool sameSwf = string.Equals(target.SwfFile, replacement.SwfFile, StringComparison.OrdinalIgnoreCase);

        SwfFile targetSwf;
        using (var fs = new FileStream(targetSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            targetSwf = SwfFile.ReadFrom(fs);
        }

        var targetSymClass = targetSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
        if (targetSymClass == null)
            throw new InvalidDataException($"SymbolClassTag not found in '{target.SwfFile}'.");

        string targetSuffix = target.SymbolSuffix;
        string replSuffix = replacement.SymbolSuffix;

        var colorMap = BuildWeaponColorSwapMap(target, replacement, request);

        if (sameSwf)
        {
            if (!string.Equals(targetSuffix, replSuffix, StringComparison.OrdinalIgnoreCase))
            {
                var replRefs = targetSymClass.References
                    .Where(r => r.SymbolName.EndsWith("_" + replSuffix, StringComparison.OrdinalIgnoreCase) &&
                                IsSymbolForBaseWeapon(r.SymbolName, target.BaseWeapon))
                    .ToList();

                var replMap = new Dictionary<string, ushort>(StringComparer.OrdinalIgnoreCase);
                foreach (var r in replRefs)
                {
                    string prefix = r.SymbolName[..r.SymbolName.LastIndexOf('_')];
                    replMap[prefix] = r.SymbolID;
                }

                int remapped = 0;
                var targetRefs = targetSymClass.References
                    .Where(r => r.SymbolName.EndsWith("_" + targetSuffix, StringComparison.OrdinalIgnoreCase) &&
                                IsSymbolForBaseWeapon(r.SymbolName, target.BaseWeapon))
                    .ToList();

                foreach (var tr in targetRefs)
                {
                    string prefix = tr.SymbolName[..tr.SymbolName.LastIndexOf('_')];
                    if (replMap.TryGetValue(prefix, out ushort replId))
                    {
                        tr.SymbolID = replId;
                        remapped++;
                    }
                }

                foreach (var kvp in replMap)
                {
                    string expectedName = $"{kvp.Key}_{targetSuffix}";
                    if (!targetSymClass.References.Any(r => r.SymbolName.Equals(expectedName, StringComparison.OrdinalIgnoreCase)))
                    {
                        targetSymClass.References.Add(new SwfSymbolReference
                        {
                            SymbolID = kvp.Value,
                            SymbolName = expectedName
                        });
                        remapped++;
                    }
                }

                result.OperationLog.Add($"Remapped {remapped} weapon symbols for '{target.DisplayName}' -> '{replacement.DisplayName}'.");
            }
        }
        else
        {
            SwfFile replSwf;
            using (var fs = new FileStream(replSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            {
                replSwf = SwfFile.ReadFrom(fs);
            }

            var replSymClass = replSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
            if (replSymClass == null)
                throw new InvalidDataException($"SymbolClassTag not found in weapon source '{replacement.SwfFile}'.");

            var sourceRefs = replSymClass.References
                .Where(r => r.SymbolName.EndsWith("_" + replSuffix, StringComparison.OrdinalIgnoreCase) &&
                            IsSymbolForBaseWeapon(r.SymbolName, target.BaseWeapon))
                .ToList();

            PortAndRemapSymbols(replSwf, targetSwf, sourceRefs, targetSuffix, colorMap, result);
        }

        if (sameSwf && colorMap.Count > 0)
        {
            ApplyColorMapToSwf(targetSwf, colorMap, result);
        }

        SaveSwfAtomically(targetSwf, targetSwfPath);
    }

    private void PortAndRemapSymbols(
        SwfFile sourceSwf,
        SwfFile targetSwf,
        List<SwfSymbolReference> sourceRefs,
        string targetSuffix,
        Dictionary<uint, uint> colorMap,
        ModificationResult result)
    {
        // Recursively collect all character IDs needed
        var neededIds = new HashSet<ushort>();
        foreach (var r in sourceRefs)
        {
            CollectNeededCharacters(r.SymbolID, sourceSwf, neededIds);
        }

        // Determine max ID in target SWF
        ushort maxId = 0;
        foreach (var tag in targetSwf.Tags)
        {
            if (tag is DefineSpriteTag s) maxId = Math.Max(maxId, s.SpriteID);
            else if (tag is ShapeBaseTag sh) maxId = Math.Max(maxId, sh.ShapeID);
        }

        // Create re-indexing map
        var idMap = new Dictionary<ushort, ushort>();
        foreach (var oldId in neededIds)
        {
            maxId++;
            idMap[oldId] = maxId;
        }

        // Clone and import shapes
        var newShapeTags = new List<SwfTagBase>();
        foreach (var sh in sourceSwf.Tags.OfType<ShapeBaseTag>())
        {
            if (neededIds.Contains(sh.ShapeID))
            {
                var clonedShape = CloneShapeWithNewId(sh, idMap[sh.ShapeID], colorMap);
                newShapeTags.Add(clonedShape);
            }
        }

        // Clone and import sprites
        var newSpriteTags = new List<SwfTagBase>();
        foreach (var sp in sourceSwf.Tags.OfType<DefineSpriteTag>())
        {
            if (neededIds.Contains(sp.SpriteID))
            {
                var clonedSprite = CloneSpriteWithNewId(sp, idMap[sp.SpriteID], idMap);
                newSpriteTags.Add(clonedSprite);
            }
        }

        // Insert new tags before SymbolClassTag
        var targetSymClass = targetSwf.Tags.OfType<SymbolClassTag>().First();
        int insertIdx = targetSwf.Tags.IndexOf(targetSymClass);
        if (insertIdx < 0) insertIdx = targetSwf.Tags.Count;

        foreach (var sh in newShapeTags)
        {
            targetSwf.Tags.Insert(insertIdx++, sh);
        }
        foreach (var sp in newSpriteTags)
        {
            targetSwf.Tags.Insert(insertIdx++, sp);
        }

        // Map symbols
        int portedCount = 0;
        foreach (var sr in sourceRefs)
        {
            string prefix = sr.SymbolName[..sr.SymbolName.LastIndexOf('_')];
            string targetSymName = $"{prefix}_{targetSuffix}";
            ushort newSpriteId = idMap[sr.SymbolID];

            var existingTarget = targetSymClass.References.FirstOrDefault(r => r.SymbolName.Equals(targetSymName, StringComparison.OrdinalIgnoreCase));
            if (existingTarget != null)
            {
                existingTarget.SymbolID = newSpriteId;
            }
            else
            {
                targetSymClass.References.Add(new SwfSymbolReference
                {
                    SymbolID = newSpriteId,
                    SymbolName = targetSymName
                });
            }
            portedCount++;
        }

        result.OperationLog.Add($"Ported {newShapeTags.Count} shapes and {newSpriteTags.Count} sprites ({portedCount} symbol linkages).");
    }

    public static void CollectNeededCharacters(ushort charId, SwfFile swf, HashSet<ushort> collected)
    {
        if (!collected.Add(charId)) return;

        var sprite = swf.Tags.OfType<DefineSpriteTag>().FirstOrDefault(s => s.SpriteID == charId);
        if (sprite != null)
        {
            foreach (var t in sprite.Tags)
            {
                if (t is PlaceObject2Tag p2)
                {
                    CollectNeededCharacters(p2.CharacterID, swf, collected);
                }
                else if (t is PlaceObjectBaseTag pb)
                {
                    CollectNeededCharacters(pb.CharacterID, swf, collected);
                }
            }
        }
    }

    private static ShapeBaseTag CloneShapeWithNewId(ShapeBaseTag source, ushort newId, Dictionary<uint, uint> colorMap)
    {
        using var ms = new MemoryStream();
        // Serialize and re-read via SwfLib to clone
        var tempSwf = new SwfFile();
        tempSwf.FileInfo.Format = SwfFormat.FWS;
        tempSwf.FileInfo.Version = 15;
        tempSwf.Tags.Add(source);
        tempSwf.WriteTo(ms);

        ms.Position = 0;
        var deserialized = SwfFile.ReadFrom(ms);
        var cloned = deserialized.Tags.OfType<ShapeBaseTag>().First();
        cloned.ShapeID = newId;

        if (colorMap.Count > 0)
        {
            ApplyColorMapToShape(cloned, colorMap);
        }

        return cloned;
    }

    private static DefineSpriteTag CloneSpriteWithNewId(DefineSpriteTag source, ushort newId, Dictionary<ushort, ushort> idMap)
    {
        using var ms = new MemoryStream();
        var tempSwf = new SwfFile();
        tempSwf.FileInfo.Format = SwfFormat.FWS;
        tempSwf.FileInfo.Version = 15;
        tempSwf.Tags.Add(source);
        tempSwf.WriteTo(ms);

        ms.Position = 0;
        var deserialized = SwfFile.ReadFrom(ms);
        var cloned = deserialized.Tags.OfType<DefineSpriteTag>().First();
        cloned.SpriteID = newId;

        foreach (var t in cloned.Tags)
        {
            if (t is PlaceObject2Tag p2 && idMap.TryGetValue(p2.CharacterID, out ushort remappedId))
            {
                p2.CharacterID = remappedId;
            }
            else if (t is PlaceObjectBaseTag pb && idMap.TryGetValue(pb.CharacterID, out ushort remappedIdBase))
            {
                pb.CharacterID = remappedIdBase;
            }
        }

        return cloned;
    }

    private static Dictionary<uint, uint> BuildColorSwapMap(ModificationRequest request)
    {
        var map = new Dictionary<uint, uint>();
        var activeCostume = request.ReplacementCostume ?? request.TargetCostume;
        if (activeCostume == null) return map;

        // Custom palette overrides take highest precedence
        if (request.CustomPalette != null && request.CustomPalette.Count > 0)
        {
            foreach (var kvp in request.CustomPalette)
            {
                if (activeCostume.BaseColors.TryGetValue(kvp.Key, out uint baseColor))
                {
                    map[baseColor] = kvp.Value;
                }
                if (request.TargetCostume != null && request.TargetCostume.BaseColors.TryGetValue(kvp.Key, out uint targetBase))
                {
                    map[targetBase] = kvp.Value;
                }
            }
        }
        else if (request.SelectedColorScheme != null)
        {
            foreach (var kvp in request.SelectedColorScheme.SwapColors)
            {
                if (activeCostume.BaseColors.TryGetValue(kvp.Key, out uint baseColor))
                {
                    map[baseColor] = kvp.Value;
                }
                if (request.TargetCostume != null && request.TargetCostume.BaseColors.TryGetValue(kvp.Key, out uint targetBase))
                {
                    map[targetBase] = kvp.Value;
                }
            }
        }

        return map;
    }

    private Dictionary<uint, uint> BuildWeaponColorSwapMap(WeaponSkinInfo target, WeaponSkinInfo? replacement, ModificationRequest request)
    {
        var map = new Dictionary<uint, uint>();
        var activeWeapon = replacement ?? target;

        IReadOnlyDictionary<string, uint>? swapSource = null;
        if (request.CustomPalette != null && request.CustomPalette.Count > 0)
        {
            swapSource = request.CustomPalette;
        }
        else if (request.SelectedColorScheme != null && request.SelectedColorScheme.SwapColors.Count > 0)
        {
            swapSource = request.SelectedColorScheme.SwapColors;
        }

        if (swapSource == null)
            return map;

        var baseColorSources = new List<Dictionary<string, uint>>();
        if (activeWeapon?.BaseColors != null && activeWeapon.BaseColors.Count > 0)
            baseColorSources.Add(activeWeapon.BaseColors);
        if (target?.BaseColors != null && target.BaseColors.Count > 0)
            baseColorSources.Add(target.BaseColors);

        if (_catalog != null)
        {
            var heroesToCheck = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            if (!string.IsNullOrEmpty(activeWeapon?.OwnerHero) && !activeWeapon.OwnerHero.Equals("--", StringComparison.OrdinalIgnoreCase))
                heroesToCheck.Add(activeWeapon.OwnerHero);
            if (!string.IsNullOrEmpty(target?.OwnerHero) && !target.OwnerHero.Equals("--", StringComparison.OrdinalIgnoreCase))
                heroesToCheck.Add(target.OwnerHero);

            // InheritCostumeDefines check
            if (!string.IsNullOrEmpty(activeWeapon?.InheritCostumeDefines))
                heroesToCheck.Add(activeWeapon.InheritCostumeDefines);
            if (!string.IsNullOrEmpty(target?.InheritCostumeDefines))
                heroesToCheck.Add(target.InheritCostumeDefines);

            foreach (var h in heroesToCheck)
            {
                var costumes = _catalog.Costumes
                    .Where(c => c.OwnerHero.Equals(h, StringComparison.OrdinalIgnoreCase) || c.CostumeName.Equals(h, StringComparison.OrdinalIgnoreCase))
                    .ToList();
                foreach (var oc in costumes)
                {
                    if (oc.BaseColors != null && oc.BaseColors.Count > 0)
                        baseColorSources.Add(oc.BaseColors);
                }
            }
        }

        if (request.ReplacementCostume?.BaseColors != null && request.ReplacementCostume.BaseColors.Count > 0)
            baseColorSources.Add(request.ReplacementCostume.BaseColors);
        if (request.TargetCostume?.BaseColors != null && request.TargetCostume.BaseColors.Count > 0)
            baseColorSources.Add(request.TargetCostume.BaseColors);

        foreach (var kvp in swapSource)
        {
            foreach (var bcs in baseColorSources)
            {
                if (bcs.TryGetValue(kvp.Key, out uint baseColor))
                {
                    map[baseColor] = kvp.Value;
                }
            }
        }

        return map;
    }

    public static bool IsSymbolForBaseWeapon(string symbolName, string baseWeapon)
    {
        if (string.IsNullOrWhiteSpace(symbolName) || string.IsNullOrWhiteSpace(baseWeapon))
            return false;

        string[] forbiddenBodyPrefixes =
        [
            "a_head", "a_torso", "a_eyes", "a_waist", "a_leg", "a_foot",
            "a_shin", "a_shoulder", "a_jaw", "a_hair", "a_mouth", "a_ear",
            "a_neck", "a_tail", "a_horns", "a_cape", "a_collar", "a_wrist",
            "a_bicep", "a_arm", "a_elbow", "a_knee", "a_ankle", "a_pelvis",
            "a_chest", "a_belt", "a_skirt", "a_hood", "a_hat", "a_helmet",
            "a_beard", "a_nose", "a_brow", "a_pupil"
        ];

        string symLower = symbolName.ToLowerInvariant();
        foreach (var bp in forbiddenBodyPrefixes)
        {
            if (symLower.StartsWith(bp))
                return false;
        }

        string bw = baseWeapon.Trim().ToLowerInvariant();

        return bw switch
        {
            "fists" or "gauntlet" or "gauntlets" => symLower.Contains("fist") || symLower.Contains("gauntlet"),
            "scythe" => symLower.Contains("scythe"),
            "pistol" or "blaster" or "blasters" => symLower.Contains("pistol") || symLower.Contains("blaster"),
            "spear" => symLower.Contains("spear"),
            "katar" or "katars" => symLower.Contains("katar"),
            "sword" => symLower.Contains("sword") && !symLower.Contains("great"),
            "hammer" => symLower.Contains("hammer"),
            "axe" => symLower.Contains("axe"),
            "bow" => symLower.Contains("bow") && !symLower.Contains("elbow"),
            "lance" or "rocketlance" => symLower.Contains("lance"),
            "cannon" => symLower.Contains("cannon"),
            "orb" => symLower.Contains("orb"),
            "greatsword" => symLower.Contains("great"),
            "boots" or "battleboots" => symLower.Contains("boots"),
            "chakram" => symLower.Contains("chakram"),
            _ => symLower.Contains(bw) || symLower.StartsWith("a_weapon")
        };
    }

    private static void ApplyColorMapToSwf(SwfFile swf, Dictionary<uint, uint> colorMap, ModificationResult result)
    {
        int colorsChanged = 0;
        foreach (var shape in swf.Tags.OfType<ShapeBaseTag>())
        {
            colorsChanged += ApplyColorMapToShape(shape, colorMap);
        }
        result.OperationLog.Add($"Applied palette color transformations ({colorsChanged} fill colors updated).");
    }

    private static int ApplyColorMapToShape(ShapeBaseTag shape, Dictionary<uint, uint> colorMap)
    {
        int count = 0;
        if (shape is DefineShape3Tag s3)
        {
            count += ApplyToFillStylesRGBA(s3.FillStyles, colorMap);
        }
        else if (shape is DefineShape4Tag s4)
        {
            count += ApplyToFillStylesRGBA(s4.FillStyles, colorMap);
        }
        else if (shape is DefineShape2Tag s2)
        {
            count += ApplyToFillStylesRGB(s2.FillStyles, colorMap);
        }
        else if (shape is DefineShapeTag s1)
        {
            count += ApplyToFillStylesRGB(s1.FillStyles, colorMap);
        }
        return count;
    }

    private static int ApplyToFillStylesRGBA(IList<FillStyleRGBA>? fillStyles, Dictionary<uint, uint> colorMap)
    {
        int count = 0;
        if (fillStyles == null) return count;

        foreach (var fs in fillStyles)
        {
            if (fs is SolidFillStyleRGBA solid)
            {
                uint rgb = ((uint)solid.Color.Red << 16) | ((uint)solid.Color.Green << 8) | solid.Color.Blue;
                if (colorMap.TryGetValue(rgb, out uint swapRgb))
                {
                    solid.Color = new SwfRGBA
                    {
                        Red = (byte)((swapRgb >> 16) & 0xFF),
                        Green = (byte)((swapRgb >> 8) & 0xFF),
                        Blue = (byte)(swapRgb & 0xFF),
                        Alpha = solid.Color.Alpha
                    };
                    count++;
                }
            }
            else if (fs is LinearGradientFillStyleRGBA lin)
            {
                count += ApplyToGradient(lin.Gradient, colorMap);
            }
            else if (fs is RadialGradientFillStyleRGBA rad)
            {
                count += ApplyToGradient(rad.Gradient, colorMap);
            }
            else if (fs is FocalGradientFillStyleRGBA foc)
            {
                count += ApplyToGradient(foc.Gradient, colorMap);
            }
        }
        return count;
    }

    private static int ApplyToFillStylesRGB(IList<FillStyleRGB>? fillStyles, Dictionary<uint, uint> colorMap)
    {
        int count = 0;
        if (fillStyles == null) return count;

        foreach (var fs in fillStyles)
        {
            if (fs is SolidFillStyleRGB solid)
            {
                uint rgb = ((uint)solid.Color.Red << 16) | ((uint)solid.Color.Green << 8) | solid.Color.Blue;
                if (colorMap.TryGetValue(rgb, out uint swapRgb))
                {
                    solid.Color = new SwfRGB
                    {
                        Red = (byte)((swapRgb >> 16) & 0xFF),
                        Green = (byte)((swapRgb >> 8) & 0xFF),
                        Blue = (byte)(swapRgb & 0xFF)
                    };
                    count++;
                }
            }
            else if (fs is LinearGradientFillStyleRGB lin)
            {
                count += ApplyToGradientRGB(lin.Gradient, colorMap);
            }
            else if (fs is RadialGradientFillStyleRGB rad)
            {
                count += ApplyToGradientRGB(rad.Gradient, colorMap);
            }
        }
        return count;
    }

    private static int ApplyToGradientRGB(BaseGradientRGB gradient, Dictionary<uint, uint> colorMap)
    {
        int count = 0;
        if (gradient?.GradientRecords == null) return count;

        for (int i = 0; i < gradient.GradientRecords.Count; i++)
        {
            var rec = gradient.GradientRecords[i];
            uint rgb = ((uint)rec.Color.Red << 16) | ((uint)rec.Color.Green << 8) | rec.Color.Blue;
            if (colorMap.TryGetValue(rgb, out uint swapRgb))
            {
                rec.Color = new SwfRGB
                {
                    Red = (byte)((swapRgb >> 16) & 0xFF),
                    Green = (byte)((swapRgb >> 8) & 0xFF),
                    Blue = (byte)(swapRgb & 0xFF)
                };
                count++;
            }
        }
        return count;
    }

    private static int ApplyToGradient(BaseGradientRGBA gradient, Dictionary<uint, uint> colorMap)
    {
        int count = 0;
        if (gradient?.GradientRecords == null) return count;

        for (int i = 0; i < gradient.GradientRecords.Count; i++)
        {
            var rec = gradient.GradientRecords[i];
            uint rgb = ((uint)rec.Color.Red << 16) | ((uint)rec.Color.Green << 8) | rec.Color.Blue;
            if (colorMap.TryGetValue(rgb, out uint swapRgb))
            {
                rec.Color = new SwfRGBA
                {
                    Red = (byte)((swapRgb >> 16) & 0xFF),
                    Green = (byte)((swapRgb >> 8) & 0xFF),
                    Blue = (byte)(swapRgb & 0xFF),
                    Alpha = rec.Color.Alpha
                };
                count++;
            }
        }
        return count;
    }

    private static void SaveSwfAtomically(SwfFile swf, string destinationPath)
    {
        string tmpPath = destinationPath + ".tmp";
        using (var fs = new FileStream(tmpPath, FileMode.Create, FileAccess.Write, FileShare.None))
        {
            swf.WriteTo(fs);
            fs.Flush(true);
        }

        // Verify temp file by reading back with shared read access
        using (var fs = new FileStream(tmpPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            _ = SwfFile.ReadFrom(fs);
        }

        try
        {
            File.Move(tmpPath, destinationPath, overwrite: true);
        }
        catch (Exception ex) when (ex is IOException || ex is UnauthorizedAccessException)
        {
            // If File.Move fails due to concurrent access while the game is running,
            // overwrite via a shared stream write
            using (var src = new FileStream(tmpPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            using (var dst = new FileStream(destinationPath, FileMode.Create, FileAccess.Write, FileShare.ReadWrite))
            {
                src.CopyTo(dst);
                dst.Flush(true);
            }
            try { File.Delete(tmpPath); } catch { /* ignore temp cleanup */ }
        }
    }

    public AdvancedModificationResult ApplyAdvancedModification(AdvancedModificationRequest request)
    {
        var result = new AdvancedModificationResult();
        var filesToModify = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        // 1. Podium
        string? targetPodiumPath = null;
        string? replPodiumPath = null;
        if (request.TargetPodium != null && request.ReplacementPodium != null &&
            !string.Equals(request.TargetPodium.Name, request.ReplacementPodium.Name, StringComparison.OrdinalIgnoreCase))
        {
            targetPodiumPath = Path.Combine(_gamePath, request.TargetPodium.AnimFile);
            replPodiumPath = Path.Combine(_gamePath, request.ReplacementPodium.AnimFile);
            if (File.Exists(targetPodiumPath) && File.Exists(replPodiumPath) &&
                !string.Equals(targetPodiumPath, replPodiumPath, StringComparison.OrdinalIgnoreCase))
            {
                filesToModify.Add(targetPodiumPath);
            }
        }

        // 2. Sidekick
        string? targetSidekickPath = null;
        string? replSidekickPath = null;
        if (request.TargetSidekick != null && request.ReplacementSidekick != null &&
            !string.Equals(request.TargetSidekick.Name, request.ReplacementSidekick.Name, StringComparison.OrdinalIgnoreCase))
        {
            targetSidekickPath = Path.Combine(_gamePath, request.TargetSidekick.AnimFile);
            replSidekickPath = Path.Combine(_gamePath, request.ReplacementSidekick.AnimFile);
            if (File.Exists(targetSidekickPath) && File.Exists(replSidekickPath) &&
                !string.Equals(targetSidekickPath, replSidekickPath, StringComparison.OrdinalIgnoreCase))
            {
                filesToModify.Add(targetSidekickPath);
            }
        }

        // 3. KO Effect & Audio
        string? targetKoPath = null;
        string? replKoPath = null;
        if (request.TargetKoEffect != null && request.ReplacementKoEffect != null &&
            !string.Equals(request.TargetKoEffect.Name, request.ReplacementKoEffect.Name, StringComparison.OrdinalIgnoreCase))
        {
            targetKoPath = Path.Combine(_gamePath, request.TargetKoEffect.AnimFile);
            replKoPath = Path.Combine(_gamePath, request.ReplacementKoEffect.AnimFile);
            if (File.Exists(targetKoPath) && File.Exists(replKoPath) &&
                !string.Equals(targetKoPath, replKoPath, StringComparison.OrdinalIgnoreCase))
            {
                filesToModify.Add(targetKoPath);
            }
        }

        // 4. Crossover SFX
        string? srcCrossoverSfxPath = null;
        string? dstSfxPath = null;
        if (request.SyncCrossoverSfx && !string.IsNullOrWhiteSpace(request.SelectedCrossoverSfx))
        {
            srcCrossoverSfxPath = Path.Combine(_gamePath, request.SelectedCrossoverSfx);
            dstSfxPath = Path.Combine(_gamePath, "SFX_1.swf");
            if (File.Exists(srcCrossoverSfxPath) && File.Exists(dstSfxPath))
            {
                filesToModify.Add(dstSfxPath);
            }
        }

        if (filesToModify.Count == 0)
        {
            result.Success = true;
            result.Message = "لم يتم تحديد أي تعديلات جديدة لتطبيقها.";
            return result;
        }

        // Create backup
        result.BackupId = _backupManager.CreateBackup(filesToModify, "Apply Advanced Customizations");
        result.OperationLog.Add($"Created backup {result.BackupId} for {filesToModify.Count} files.");

        try
        {
            // Apply Podium
            if (targetPodiumPath != null && replPodiumPath != null && filesToModify.Contains(targetPodiumPath))
            {
                CopySwfSafely(replPodiumPath, targetPodiumPath);
                result.OperationLog.Add($"Applied Podium '{request.ReplacementPodium?.DisplayName}' onto '{request.TargetPodium?.DisplayName}'.");
            }

            // Apply Sidekick
            if (targetSidekickPath != null && replSidekickPath != null && filesToModify.Contains(targetSidekickPath))
            {
                CopySwfSafely(replSidekickPath, targetSidekickPath);
                result.OperationLog.Add($"Applied Sidekick '{request.ReplacementSidekick?.DisplayName}' onto '{request.TargetSidekick?.DisplayName}'.");
            }

            // Apply KO Effect & Audio
            if (targetKoPath != null && replKoPath != null && filesToModify.Contains(targetKoPath))
            {
                CopySwfSafely(replKoPath, targetKoPath);
                result.OperationLog.Add($"Applied KO Effect & Sound '{request.ReplacementKoEffect?.DisplayName}' onto '{request.TargetKoEffect?.DisplayName}'.");
            }

            // Apply Crossover SFX
            if (srcCrossoverSfxPath != null && dstSfxPath != null && filesToModify.Contains(dstSfxPath))
            {
                MergeSoundEffects(srcCrossoverSfxPath, dstSfxPath);
                result.OperationLog.Add($"Synced Crossover SFX bank '{request.SelectedCrossoverSfx}' into '{Path.GetFileName(dstSfxPath)}'.");
            }

            foreach (var f in filesToModify)
            {
                result.ModifiedFiles.Add(Path.GetRelativePath(_gamePath, f));
            }

            result.Success = true;
            result.Message = $"تم تطبيق التعديلات المتقدمة بنجاح على {result.ModifiedFiles.Count} ملف(ات).";
            return result;
        }
        catch (Exception ex)
        {
            result.Success = false;
            result.Message = $"فشل تطبيق التعديل المتقدم: {ex.Message}";
            result.OperationLog.Add($"ERROR: {ex}");

            // Rollback
            foreach (var f in filesToModify)
            {
                _backupManager.RestoreFile(Path.GetRelativePath(_gamePath, f));
            }
            return result;
        }
    }

    private static void CopySwfSafely(string sourcePath, string destinationPath)
    {
        byte[] bytes = File.ReadAllBytes(sourcePath);
        string tmpPath = destinationPath + ".tmp";
        File.WriteAllBytes(tmpPath, bytes);

        try
        {
            File.Move(tmpPath, destinationPath, overwrite: true);
        }
        catch
        {
            using (var src = new FileStream(tmpPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            using (var dst = new FileStream(destinationPath, FileMode.Create, FileAccess.Write, FileShare.ReadWrite))
            {
                src.CopyTo(dst);
                dst.Flush(true);
            }
            try { File.Delete(tmpPath); } catch { }
        }
    }

    private static void MergeSoundEffects(string sourceSfxPath, string targetSfxPath)
    {
        SwfFile srcSwf;
        using (var fs = new FileStream(sourceSfxPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            srcSwf = SwfFile.ReadFrom(fs);
        }

        SwfFile dstSwf;
        using (var fs = new FileStream(targetSfxPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            dstSwf = SwfFile.ReadFrom(fs);
        }

        // Add sound symbols from srcSwf to dstSwf
        var srcSymClass = srcSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
        var dstSymClass = dstSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();

        if (srcSymClass != null && dstSymClass != null)
        {
            foreach (var r in srcSymClass.References)
            {
                if (!dstSymClass.References.Any(dr => dr.SymbolName.Equals(r.SymbolName, StringComparison.OrdinalIgnoreCase)))
                {
                    dstSymClass.References.Add(new SwfSymbolReference
                    {
                        SymbolID = r.SymbolID,
                        SymbolName = r.SymbolName
                    });
                }
            }
        }

        SaveSwfAtomically(dstSwf, targetSfxPath);
    }

    private WeaponSkinInfo? ResolveFullWeaponSkin(WeaponSkinInfo? weapon)
    {
        if (weapon == null) return null;
        if (weapon.AttackSoundOverrides.Count > 0 && !string.IsNullOrWhiteSpace(weapon.SoundBank))
        {
            return weapon;
        }
        if (_catalog != null)
        {
            var found = _catalog.WeaponSkins.FirstOrDefault(w =>
                w.WeaponSkinId == weapon.WeaponSkinId ||
                w.WeaponSkinName.Equals(weapon.WeaponSkinName, StringComparison.OrdinalIgnoreCase));
            if (found != null) return found;
        }
        return weapon;
    }

    public static readonly HashSet<string> BaseWeaponSoundBanks = new(StringComparer.OrdinalIgnoreCase)
    {
        "WEP_Sword",
        "WEP_Hammer",
        "WEP_Spear",
        "WEP_RocketLance",
        "WEP_Pistol",
        "WEP_Katar",
        "WEP_Bow",
        "WEP_Axe",
        "WEP_Scythe",
        "WEP_Cannon",
        "WEP_Orb",
        "WEP_Greatsword",
        "WEP_Boots",
        "WEP_Chakrams",
        "WEP_Fists",
        "WEP_Unarmed",
        "WEP_Explosion"
    };

    public static bool HasDedicatedSoundBank(WeaponSkinInfo? weapon)
    {
        if (weapon == null) return false;
        if (string.IsNullOrWhiteSpace(weapon.SoundBank)) return false;
        if (BaseWeaponSoundBanks.Contains(weapon.SoundBank)) return false;
        if (weapon.SoundBank.Equals($"WEP_{weapon.BaseWeapon}", StringComparison.OrdinalIgnoreCase)) return false;
        return true;
    }

    private static bool ShouldApplyWeaponAudio(WeaponSkinInfo? replacement, WeaponSkinInfo? target)
    {
        if (replacement == null || target == null) return false;
        if (!HasCustomWeaponSound(replacement)) return false;

        // CRITICAL FIX: Only apply custom sound effects if the target weapon has its OWN
        // dedicated custom sound bank (e.g. WEP_Saber_07, WEP_Saber_01, WEP_Pistol_02, etc.).
        // We NEVER overwrite shared base sound banks (WEP_Sword.bnk, WEP_Hammer.bnk, etc.)
        // because those banks are globally shared by all regular/normal weapons of that type across all legends.
        // Overwriting a base sound bank causes all normal weapons in the game to play the special sound effects.
        return HasDedicatedSoundBank(target);
    }

    private static bool HasCustomWeaponSound(WeaponSkinInfo weapon)
    {
        return !string.IsNullOrWhiteSpace(weapon.SoundBank) && weapon.AttackSoundOverrides.Count > 0;
    }

    private string GetWeaponSoundBankPath(WeaponSkinInfo weapon)
    {
        string bankName = !string.IsNullOrWhiteSpace(weapon.SoundBank)
            ? weapon.SoundBank
            : $"WEP_{weapon.BaseWeapon}";
        return Path.Combine(_gamePath, "audio", "pc", $"{bankName}.bnk");
    }

    public static uint FnvHash(string name)
    {
        byte[] bytes = System.Text.Encoding.ASCII.GetBytes(name.ToLowerInvariant());
        uint hash = 2166136261;
        foreach (byte b in bytes)
        {
            hash *= 16777619;
            hash ^= b;
        }
        return hash;
    }

    private static void ApplyWeaponAudioModification(
        WeaponSkinInfo target,
        WeaponSkinInfo replacement,
        string targetBnkPath,
        string replBnkPath,
        ModificationResult result)
    {
        if (!File.Exists(replBnkPath))
        {
            result.OperationLog.Add($"Warning: Source sound bank '{Path.GetFileName(replBnkPath)}' not found. Audio SFX was not applied.");
            return;
        }

        byte[] bytes = File.ReadAllBytes(replBnkPath);
        if (bytes.Length < 16)
        {
            throw new InvalidDataException($"Source sound bank '{replBnkPath}' is invalid or corrupted.");
        }

        string targetBankName = !string.IsNullOrWhiteSpace(target.SoundBank)
            ? target.SoundBank
            : $"WEP_{target.BaseWeapon}";
        uint targetBankId = FnvHash(targetBankName);
        Array.Copy(BitConverter.GetBytes(targetBankId), 0, bytes, 12, 4);

        var eventMap = new Dictionary<uint, uint>();
        if (target.AttackSoundOverrides.Count > 0)
        {
            var targetEventByBase = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var tovr in target.AttackSoundOverrides)
            {
                var tparts = tovr.Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
                if (tparts.Length == 2)
                {
                    targetEventByBase[tparts[0]] = tparts[1];
                }
            }

            foreach (var ovr in replacement.AttackSoundOverrides)
            {
                var parts = ovr.Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
                if (parts.Length == 2)
                {
                    string targetEventName = targetEventByBase.TryGetValue(parts[0], out var mappedTarget)
                        ? mappedTarget
                        : parts[0];
                    uint targetId = FnvHash(targetEventName);
                    uint replId = FnvHash(parts[1]);
                    eventMap[replId] = targetId;
                }
            }
        }
        else
        {
            foreach (var ovr in replacement.AttackSoundOverrides)
            {
                var parts = ovr.Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
                if (parts.Length == 2)
                {
                    uint origId = FnvHash(parts[0]);
                    uint replId = FnvHash(parts[1]);
                    eventMap[replId] = origId;
                }
            }
        }

        int remappedCount = 0;
        int pos = 0;
        while (pos < bytes.Length - 8)
        {
            string tag = System.Text.Encoding.ASCII.GetString(bytes, pos, 4);
            uint size = BitConverter.ToUInt32(bytes, pos + 4);
            pos += 8;

            if (tag == "HIRC")
            {
                int hircEnd = pos + (int)size;
                uint count = BitConverter.ToUInt32(bytes, pos);
                int objPos = pos + 4;
                for (int i = 0; i < count && objPos < hircEnd; i++)
                {
                    byte type = bytes[objPos];
                    uint objSize = BitConverter.ToUInt32(bytes, objPos + 1);
                    if (type == 4) // Event
                    {
                        uint eventId = BitConverter.ToUInt32(bytes, objPos + 5);
                        if (eventMap.TryGetValue(eventId, out uint mappedId))
                        {
                            Array.Copy(BitConverter.GetBytes(mappedId), 0, bytes, objPos + 5, 4);
                            remappedCount++;
                        }
                    }
                    objPos += 5 + (int)objSize;
                }
                break;
            }
            pos += (int)size;
        }

        string tmpPath = targetBnkPath + ".tmp";
        File.WriteAllBytes(tmpPath, bytes);
        try
        {
            File.Move(tmpPath, targetBnkPath, overwrite: true);
        }
        catch
        {
            using (var src = new FileStream(tmpPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            using (var dst = new FileStream(targetBnkPath, FileMode.Create, FileAccess.Write, FileShare.ReadWrite))
            {
                src.CopyTo(dst);
                dst.Flush(true);
            }
            try { File.Delete(tmpPath); } catch { }
        }

        result.OperationLog.Add($"Applied custom weapon SFX sound bank '{replacement.SoundBank}' onto '{Path.GetFileName(targetBnkPath)}' with {remappedCount} remapped events.");
    }
}
