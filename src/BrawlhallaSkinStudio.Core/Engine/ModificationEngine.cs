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

        string handsSwfPath = Path.Combine(_gamePath, "Gfx_Hands.swf");
        string targetHands = GetEffectiveHandsSuffix(request.TargetCostume);
        string replHands = GetEffectiveHandsSuffix(request.ReplacementCostume ?? request.TargetCostume);
        bool needsHands = File.Exists(handsSwfPath) &&
            (!string.Equals(targetHands, replHands, StringComparison.OrdinalIgnoreCase) || hasColorScheme);
        if (needsHands)
        {
            targetFilesToModify.Add(handsSwfPath);
        }

        var crossoverSfxFiles = GetCrossoverSfxFiles(request.TargetCostume, request.ReplacementCostume);
        foreach (var sfxFile in crossoverSfxFiles)
        {
            if (File.Exists(sfxFile))
            {
                targetFilesToModify.Add(sfxFile);
            }
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
            // Process Costume Modification (Full body, cape, head, accessories, blanking missing parts)
            if (request.ReplacementCostume != null || hasColorScheme)
            {
                ApplyCostumeModification(request, result);
            }

            // Process Hands Modification (Gfx_Hands.swf - all 107 hand poses & color swap)
            if (needsHands)
            {
                ApplyHandsModification(targetHands, replHands, BuildColorSwapMap(request), result);
            }

            // Process Crossover SFX & Attack Specials
            if (request.ReplacementCostume != null)
            {
                ApplyCrossoverVisualEffects(request.TargetCostume, request.ReplacementCostume, result);
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
                            result.ModifiedFiles.Add(targetBnk);
                        }
                    }
                    else
                    {
                        string baseBnkRel = Path.Combine("audio", "pc", $"WEP_{request.TargetWeapon1.BaseWeapon}.bnk");
                        if (_backupManager.IsFileModified(baseBnkRel))
                        {
                            _backupManager.RestoreFile(baseBnkRel);
                            result.OperationLog.Add($"Restored clean official audio for WEP_{request.TargetWeapon1.BaseWeapon}.bnk.");
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
                            result.ModifiedFiles.Add(targetBnk);
                        }
                    }
                    else
                    {
                        string baseBnkRel = Path.Combine("audio", "pc", $"WEP_{request.TargetWeapon2.BaseWeapon}.bnk");
                        if (_backupManager.IsFileModified(baseBnkRel))
                        {
                            _backupManager.RestoreFile(baseBnkRel);
                            result.OperationLog.Add($"Restored clean official audio for WEP_{request.TargetWeapon2.BaseWeapon}.bnk.");
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

                // Also include any All{Costume} or cape/head symbols in same SWF
                foreach (var r in targetSymClass.References)
                {
                    if (r.SymbolName.EndsWith("_All" + replSuffix, StringComparison.OrdinalIgnoreCase) ||
                        r.SymbolName.EndsWith("_All" + replCostume.CostumeName, StringComparison.OrdinalIgnoreCase))
                    {
                        string prefix = r.SymbolName[..r.SymbolName.LastIndexOf('_')];
                        replMap[prefix] = r.SymbolID;
                    }
                    if (!string.IsNullOrEmpty(replCostume.DefaultCape) && replCostume.DefaultCape.Contains('/'))
                    {
                        string capeSuf = replCostume.DefaultCape.Split('/')[1];
                        if (r.SymbolName.EndsWith("_" + capeSuf, StringComparison.OrdinalIgnoreCase))
                        {
                            string prefix = r.SymbolName[..r.SymbolName.LastIndexOf('_')];
                            replMap[prefix] = r.SymbolID;
                        }
                    }
                    if (!string.IsNullOrEmpty(replCostume.HeadGfxCustomArt) && replCostume.HeadGfxCustomArt.Contains('/'))
                    {
                        string headSuf = replCostume.HeadGfxCustomArt.Split('/')[1];
                        if (r.SymbolName.EndsWith("_" + headSuf, StringComparison.OrdinalIgnoreCase))
                        {
                            string prefix = r.SymbolName[..r.SymbolName.LastIndexOf('_')];
                            replMap[prefix] = r.SymbolID;
                        }
                    }
                }

                ushort emptySpriteId = GetOrCreateEmptySprite(targetSwf);
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
                    else if (AnimationFallbacks.TryGetValue(prefix, out var fallbacks))
                    {
                        foreach (var fb in fallbacks)
                        {
                            if (replMap.TryGetValue(fb, out ushort fallbackId))
                            {
                                tr.SymbolID = fallbackId;
                                remappedCount++;
                                break;
                            }
                        }
                    }
                    else if (IsCosmeticPartEligibleForBlanking(prefix))
                    {
                        // Blank out missing cosmetic part (old nose, old beard, old hair, old hat, old cape)
                        tr.SymbolID = emptySpriteId;
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

                // Ensure all animation/jump fallbacks exist for targetSuffix so the game engine does not fall back to Gfx_Player.swf
                foreach (var kvp in AnimationFallbacks)
                {
                    string expectedTargetSym = $"{kvp.Key}_{targetSuffix}";
                    if (!targetSymClass.References.Any(r => r.SymbolName.Equals(expectedTargetSym, StringComparison.OrdinalIgnoreCase)))
                    {
                        ushort symId = 0;
                        if (replMap.TryGetValue(kvp.Key, out ushort directId))
                        {
                            symId = directId;
                        }
                        else
                        {
                            foreach (var fb in kvp.Value)
                            {
                                if (replMap.TryGetValue(fb, out ushort fbId))
                                {
                                    symId = fbId;
                                    break;
                                }
                            }
                        }

                        if (symId != 0)
                        {
                            targetSymClass.References.Add(new SwfSymbolReference
                            {
                                SymbolID = symId,
                                SymbolName = expectedTargetSym
                            });
                            remappedCount++;
                        }
                    }
                }

                result.OperationLog.Add($"Remapped {remappedCount} symbols for costume '{targetCostume.CostumeName}' -> '{replCostume.CostumeName}' (full body replacement).");
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

            string capeSuffix = "";
            if (!string.IsNullOrEmpty(replCostume.DefaultCape) && replCostume.DefaultCape.Contains('/'))
            {
                capeSuffix = replCostume.DefaultCape.Split('/')[1];
            }
            string headSuffix = "";
            if (!string.IsNullOrEmpty(replCostume.HeadGfxCustomArt) && replCostume.HeadGfxCustomArt.Contains('/'))
            {
                headSuffix = replCostume.HeadGfxCustomArt.Split('/')[1];
            }

            var sourceRefs = replSymClass.References
                .Where(r => r.SymbolName.EndsWith("_" + replSuffix, StringComparison.OrdinalIgnoreCase) ||
                            r.SymbolName.EndsWith("_All" + replSuffix, StringComparison.OrdinalIgnoreCase) ||
                            r.SymbolName.EndsWith("_All" + replCostume.CostumeName, StringComparison.OrdinalIgnoreCase) ||
                            (!string.IsNullOrEmpty(capeSuffix) && r.SymbolName.EndsWith("_" + capeSuffix, StringComparison.OrdinalIgnoreCase)) ||
                            (!string.IsNullOrEmpty(headSuffix) && r.SymbolName.EndsWith("_" + headSuffix, StringComparison.OrdinalIgnoreCase)))
                .ToList();

            PortAndRemapSymbols(replSwf, targetSwf, sourceRefs, targetSuffix, colorMap, result);

            // Also check if DefaultCape is in a separate external SWF (e.g. Gfx_WarRolandBP9.swf)
            if (!string.IsNullOrEmpty(replCostume.DefaultCape) && replCostume.DefaultCape.Contains('/'))
            {
                string capeSwfFile = replCostume.DefaultCape.Split('/')[0];
                string capeSwfPath = Path.Combine(_gamePath, capeSwfFile);
                if (File.Exists(capeSwfPath) && !string.Equals(capeSwfPath, replSwfPath, StringComparison.OrdinalIgnoreCase))
                {
                    using var cfs = new FileStream(capeSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
                    var cSwf = SwfFile.ReadFrom(cfs);
                    var cSymClass = cSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
                    if (cSymClass != null)
                    {
                        var capeRefs = cSymClass.References.Where(r => r.SymbolName.EndsWith("_" + capeSuffix, StringComparison.OrdinalIgnoreCase)).ToList();
                        if (capeRefs.Count > 0)
                        {
                            PortAndRemapSymbols(cSwf, targetSwf, capeRefs, targetSuffix, colorMap, result);
                        }
                    }
                }
            }

            // Also check if HeadGfxCustomArt is in a separate external SWF
            if (!string.IsNullOrEmpty(replCostume.HeadGfxCustomArt) && replCostume.HeadGfxCustomArt.Contains('/'))
            {
                string headSwfFile = replCostume.HeadGfxCustomArt.Split('/')[0];
                string headSwfPath = Path.Combine(_gamePath, headSwfFile);
                if (File.Exists(headSwfPath) && !string.Equals(headSwfPath, replSwfPath, StringComparison.OrdinalIgnoreCase))
                {
                    using var hfs = new FileStream(headSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
                    var hSwf = SwfFile.ReadFrom(hfs);
                    var hSymClass = hSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
                    if (hSymClass != null)
                    {
                        var headRefs = hSymClass.References.Where(r => r.SymbolName.EndsWith("_" + headSuffix, StringComparison.OrdinalIgnoreCase)).ToList();
                        if (headRefs.Count > 0)
                        {
                            PortAndRemapSymbols(hSwf, targetSwf, headRefs, targetSuffix, colorMap, result);
                        }
                    }
                }
            }
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
        var portedPrefixMap = new Dictionary<string, ushort>(StringComparer.OrdinalIgnoreCase);
        foreach (var sr in sourceRefs)
        {
            string prefix = sr.SymbolName[..sr.SymbolName.LastIndexOf('_')];
            string targetSymName = $"{prefix}_{targetSuffix}";
            ushort newSpriteId = idMap[sr.SymbolID];
            portedPrefixMap[prefix] = newSpriteId;

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

            // Also register original symbol name if it's a cape, head, or special accessory
            if (!targetSymClass.References.Any(r => r.SymbolName.Equals(sr.SymbolName, StringComparison.OrdinalIgnoreCase)))
            {
                targetSymClass.References.Add(new SwfSymbolReference
                {
                    SymbolID = newSpriteId,
                    SymbolName = sr.SymbolName
                });
            }

            portedCount++;
        }

        // Remap target symbols that source was missing (e.g. jump/animation bones or missing cosmetic parts to blank)
        ushort emptySpriteId = GetOrCreateEmptySprite(targetSwf);
        var targetRefs = targetSymClass.References
            .Where(r => r.SymbolName.EndsWith("_" + targetSuffix, StringComparison.OrdinalIgnoreCase))
            .ToList();

        foreach (var tr in targetRefs)
        {
            string prefix = tr.SymbolName[..tr.SymbolName.LastIndexOf('_')];
            if (!portedPrefixMap.ContainsKey(prefix))
            {
                if (AnimationFallbacks.TryGetValue(prefix, out var fallbacks))
                {
                    foreach (var fb in fallbacks)
                    {
                        if (portedPrefixMap.TryGetValue(fb, out ushort fbId))
                        {
                            tr.SymbolID = fbId;
                            portedCount++;
                            break;
                        }
                    }
                }
                else if (IsCosmeticPartEligibleForBlanking(prefix))
                {
                    // Blank out missing cosmetic part (old nose, old beard, old hair, old hat, old cape)
                    tr.SymbolID = emptySpriteId;
                    portedCount++;
                }
            }
        }

        // Ensure all animation/jump fallbacks exist for targetSuffix so the game does not fall back to Gfx_Player.swf
        foreach (var kvp in AnimationFallbacks)
        {
            string expectedTargetSym = $"{kvp.Key}_{targetSuffix}";
            if (!targetSymClass.References.Any(r => r.SymbolName.Equals(expectedTargetSym, StringComparison.OrdinalIgnoreCase)))
            {
                ushort symId = 0;
                if (portedPrefixMap.TryGetValue(kvp.Key, out ushort directId))
                {
                    symId = directId;
                }
                else
                {
                    foreach (var fb in kvp.Value)
                    {
                        if (portedPrefixMap.TryGetValue(fb, out ushort fbId))
                        {
                            symId = fbId;
                            break;
                        }
                    }
                }

                if (symId != 0)
                {
                    targetSymClass.References.Add(new SwfSymbolReference
                    {
                        SymbolID = symId,
                        SymbolName = expectedTargetSym
                    });
                    portedCount++;
                }
            }
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

    public static readonly Dictionary<string, string[]> AnimationFallbacks = new(StringComparer.OrdinalIgnoreCase)
    {
        ["a_Foot1Bent"] = new[] { "a_Foot1", "a_Foot1Side" },
        ["a_Foot1Side"] = new[] { "a_Foot1", "a_Foot1Bent" },
        ["a_Foot1"] = new[] { "a_Foot1Side", "a_Foot1Bent" },
        ["a_ShinSideBend"] = new[] { "a_ShinSide", "a_Shin", "a_ShinSideStraight", "a_ShinFrontAngle" },
        ["a_ShinFrontAngle"] = new[] { "a_Shin", "a_ShinSide", "a_ShinSideStraight", "a_ShinSideBend" },
        ["a_ShinSideStraight"] = new[] { "a_ShinSide", "a_Shin", "a_ShinSideBend", "a_ShinFrontAngle" },
        ["a_ShinSide"] = new[] { "a_Shin", "a_ShinSideStraight", "a_ShinSideBend" },
        ["a_ShinBack"] = new[] { "a_Shin", "a_ShinSide" },
        ["a_Shin"] = new[] { "a_ShinSide", "a_ShinSideStraight" },
        ["a_Torso1Back"] = new[] { "a_Torso1", "a_Torso2Back", "a_Torso2" },
        ["a_Torso2Back"] = new[] { "a_Torso2", "a_Torso1Back", "a_Torso1" },
        ["a_Torso1"] = new[] { "a_Torso1Back", "a_Torso2" },
        ["a_Torso2"] = new[] { "a_Torso2Back", "a_Torso1" },
        ["a_Waist1Back"] = new[] { "a_Waist1" },
        ["a_Waist1"] = new[] { "a_Waist1Back" },
        ["a_Leg1Back"] = new[] { "a_Leg1" },
        ["a_Leg1"] = new[] { "a_Leg1Back" },
    };

    public static bool HasDedicatedSoundBank(WeaponSkinInfo? weapon)
    {
        if (weapon == null) return false;
        if (string.IsNullOrWhiteSpace(weapon.SoundBank)) return false;
        if (BaseWeaponSoundBanks.Contains(weapon.SoundBank)) return false;
        if (weapon.SoundBank.Equals($"WEP_{weapon.BaseWeapon}", StringComparison.OrdinalIgnoreCase)) return false;
        return true;
    }

    private static readonly HashSet<string> AllowedVaderTargetSwords = new(StringComparer.OrdinalIgnoreCase)
    {
        "SwordEzio",        // Brotherhood Sword (ID 1446)
        "SwordAhsoka",      // Sword Ahsoka (ID 2028)
        "SwordAnakin",      // Sword Anakin (ID 2026)
        "SwordAsgardSaber", // Sword Asgard Saber (ID 2046)
        "SwordObiWan"       // Sword Obi Wan (ID 2021)
    };

    public static bool IsAllowedVaderTargetSword(WeaponSkinInfo target)
    {
        if (AllowedVaderTargetSwords.Contains(target.WeaponSkinName)) return true;
        if (!string.IsNullOrWhiteSpace(target.DisplayName))
        {
            string d = target.DisplayName.ToLowerInvariant();
            if (d.Contains("brotherhood") || d.Contains("ahsoka") || d.Contains("anakin") || d.Contains("obi wan") || d.Contains("obiwan") || d.Contains("asgard saber"))
                return true;
        }
        return false;
    }

    public static bool IsSaberWeapon(WeaponSkinInfo weapon)
    {
        if (weapon.WeaponSkinName.Contains("DarthVader", StringComparison.OrdinalIgnoreCase) ||
            weapon.WeaponSkinName.Contains("Saber", StringComparison.OrdinalIgnoreCase))
            return true;
        if (!string.IsNullOrWhiteSpace(weapon.SoundBank) && weapon.SoundBank.StartsWith("WEP_Saber_", StringComparison.OrdinalIgnoreCase))
            return true;
        return false;
    }

    private static bool ShouldApplyWeaponAudio(WeaponSkinInfo? replacement, WeaponSkinInfo? target)
    {
        if (replacement == null || target == null) return false;
        if (!HasCustomWeaponSound(replacement)) return false;

        // If replacement is a lightsaber / saber weapon (like SwordDarthVader):
        // Authentic routing: Darth Vader lightsaber sound effects apply to:
        // Brotherhood sword (SwordEzio), Sword Ahsoka, Sword Anakin, Sword Asgard Saber, Sword Obi Wan,
        // or any weapon that has its own dedicated saber sound bank.
        // Other standard swords (Bödvar, Hattori, Sir Roland, Jhala, etc.) keep their natural sound!
        if (IsSaberWeapon(replacement))
        {
            return IsAllowedVaderTargetSword(target) || HasDedicatedSoundBank(target);
        }

        // For all other crossover / special weapons (Megaman blasters, Shrek cannon, Fiona boots/fists,
        // Mythic Minotaur, Mythic Wu Shang, Mythic Nix, etc.):
        // Allow transferring audio so their unique sound effects work in game!
        return true;
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

    public string GetEffectiveHandsSuffix(CostumeInfo? costume)
    {
        if (costume == null) return "BaseHand";
        if (!string.IsNullOrEmpty(costume.HandsSuffix)) return costume.HandsSuffix;
        if (_catalog != null && !string.IsNullOrEmpty(costume.OwnerHero))
        {
            var def = _catalog.Costumes.FirstOrDefault(c => c.OwnerHero.Equals(costume.OwnerHero, StringComparison.OrdinalIgnoreCase) && c.IsDefault);
            if (def != null && !string.IsNullOrEmpty(def.HandsSuffix))
                return def.HandsSuffix;
        }
        return "BaseHand";
    }

    private static readonly string[] CosmeticPrefixesToBlank =
    [
        "a_nose", "a_beard", "a_mustache", "a_hair", "a_hairback", "a_hat",
        "a_helmet", "a_accent", "a_eyebrows", "a_belt", "a_cape", "a_ears",
        "a_horn", "a_horns", "a_tail", "a_glasses", "a_mask", "a_waistback",
        "a_skirt", "a_collar", "a_hood"
    ];

    private static bool IsCosmeticPartEligibleForBlanking(string prefix)
    {
        string pLower = prefix.ToLowerInvariant();
        return CosmeticPrefixesToBlank.Any(cp => pLower.StartsWith(cp));
    }

    private static ushort GetOrCreateEmptySprite(SwfFile swf)
    {
        var symClass = swf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
        var existingEmpty = symClass?.References.FirstOrDefault(r => r.SymbolName == "bc_EmptySprite");
        if (existingEmpty != null) return existingEmpty.SymbolID;

        ushort maxId = 0;
        foreach (var tag in swf.Tags)
        {
            if (tag is DefineSpriteTag s) maxId = Math.Max(maxId, s.SpriteID);
            else if (tag is ShapeBaseTag sh) maxId = Math.Max(maxId, sh.ShapeID);
        }
        ushort emptyId = (ushort)(maxId + 1);

        var emptySprite = new DefineSpriteTag
        {
            SpriteID = emptyId,
            FramesCount = 1
        };
        emptySprite.Tags.Add(new ShowFrameTag());
        emptySprite.Tags.Add(new EndTag());

        int insertIdx = symClass != null ? swf.Tags.IndexOf(symClass) : swf.Tags.Count;
        if (insertIdx < 0) insertIdx = swf.Tags.Count;
        swf.Tags.Insert(insertIdx, emptySprite);

        if (symClass != null)
        {
            symClass.References.Add(new SwfSymbolReference
            {
                SymbolID = emptyId,
                SymbolName = "bc_EmptySprite"
            });
        }

        return emptyId;
    }

    private List<string> GetCrossoverSfxFiles(CostumeInfo? targetCostume, CostumeInfo? replCostume)
    {
        var list = new List<string>();
        if (replCostume == null) return list;

        // 1. SwapCustomArts
        foreach (var swap in replCostume.SwapCustomArts)
        {
            if (swap.StartsWith("SFX_", StringComparison.OrdinalIgnoreCase) && swap.Contains('/'))
            {
                string sfxName = swap.Split('/')[0];
                string sfxPath = Path.Combine(_gamePath, sfxName);
                if (File.Exists(sfxPath) && !list.Contains(sfxPath, StringComparer.OrdinalIgnoreCase))
                {
                    list.Add(sfxPath);
                }
            }
        }

        // 2. Dedicated Crossover SFX (e.g. SFX_DarthVader.swf, SFX_MasterChief.swf)
        string dedicatedSfx = Path.Combine(_gamePath, $"SFX_{replCostume.CostumeName}.swf");
        if (File.Exists(dedicatedSfx))
        {
            var candidateWeaponSfx = new[]
            {
                "SFX_Sword.swf", "SFX_Hammer.swf", "SFX_Pistols.swf", "SFX_Spear.swf",
                "SFX_Axe.swf", "SFX_Katar.swf", "SFX_Lance.swf", "SFX_Bow.swf",
                "SFX_Fists.swf", "SFX_Scythe.swf", "SFX_Cannon.swf", "SFX_Orb.swf",
                "SFX_GreatSword.swf", "SFX_HitReacts.swf"
            };

            foreach (var wsfx in candidateWeaponSfx)
            {
                string p = Path.Combine(_gamePath, wsfx);
                if (File.Exists(p) && !list.Contains(p, StringComparer.OrdinalIgnoreCase))
                {
                    list.Add(p);
                }
            }
        }

        return list;
    }

    private static bool IsSymbolRelevantToSfx(string symbolName, string sfxFileName)
    {
        string symLower = symbolName.ToLowerInvariant();
        string fileLower = sfxFileName.ToLowerInvariant();

        if (fileLower.Contains("hitreact"))
            return symLower.Contains("hitreact") || symLower.Contains("stun");

        if (fileLower.Contains("sword"))
            return symLower.Contains("sword") || symLower.Contains("swoosh") || symLower.Contains("blade");

        if (fileLower.Contains("hammer"))
            return symLower.Contains("hammer") || symLower.Contains("slam") || symLower.Contains("smash");

        if (fileLower.Contains("pistols"))
            return symLower.Contains("pistol") || symLower.Contains("blaster") || symLower.Contains("gun") || symLower.Contains("shot");

        if (fileLower.Contains("spear"))
            return symLower.Contains("spear") || symLower.Contains("lance") || symLower.Contains("thrust");

        if (fileLower.Contains("axe"))
            return symLower.Contains("axe") || symLower.Contains("chop");

        if (fileLower.Contains("katar"))
            return symLower.Contains("katar") || symLower.Contains("claw") || symLower.Contains("stab");

        if (fileLower.Contains("bow"))
            return symLower.Contains("bow") || symLower.Contains("arrow");

        if (fileLower.Contains("fists"))
            return symLower.Contains("fist") || symLower.Contains("gauntlet") || symLower.Contains("punch");

        if (fileLower.Contains("scythe"))
            return symLower.Contains("scythe") || symLower.Contains("reap");

        if (fileLower.Contains("orb"))
            return symLower.Contains("orb") || symLower.Contains("magic") || symLower.Contains("blast") || symLower.Contains("diw");

        if (fileLower.Contains("cannon"))
            return symLower.Contains("cannon") || symLower.Contains("boom");

        if (fileLower.Contains("greatsword"))
            return symLower.Contains("greatsword") || symLower.Contains("gsword");

        return symLower.Contains("attackspecial") || symLower.Contains("iwfx") || symLower.Contains("ofx");
    }

    private void ApplyHandsModification(string targetHands, string replHands, Dictionary<uint, uint> colorMap, ModificationResult result)
    {
        string handsSwfPath = Path.Combine(_gamePath, "Gfx_Hands.swf");
        if (!File.Exists(handsSwfPath)) return;

        SwfFile handsSwf;
        using (var fs = new FileStream(handsSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            handsSwf = SwfFile.ReadFrom(fs);
        }

        var symClass = handsSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
        if (symClass == null) return;

        int remappedCount = 0;
        if (!string.Equals(targetHands, replHands, StringComparison.OrdinalIgnoreCase))
        {
            var replMap = new Dictionary<string, ushort>(StringComparer.OrdinalIgnoreCase);
            foreach (var r in symClass.References)
            {
                if (r.SymbolName.EndsWith("_" + replHands, StringComparison.OrdinalIgnoreCase))
                {
                    string prefix = r.SymbolName[..r.SymbolName.LastIndexOf('_')];
                    replMap[prefix] = r.SymbolID;
                }
            }

            foreach (var r in symClass.References)
            {
                if (r.SymbolName.EndsWith("_" + targetHands, StringComparison.OrdinalIgnoreCase))
                {
                    string prefix = r.SymbolName[..r.SymbolName.LastIndexOf('_')];
                    if (replMap.TryGetValue(prefix, out ushort replId))
                    {
                        r.SymbolID = replId;
                        remappedCount++;
                    }
                }
            }
        }

        if (colorMap.Count > 0)
        {
            ApplyColorMapToSwf(handsSwf, colorMap, result);
        }

        SaveSwfAtomically(handsSwf, handsSwfPath);
        result.OperationLog.Add($"Applied hands modification in Gfx_Hands.swf ({targetHands} -> {replHands}, {remappedCount} hand poses remapped).");
    }

    private void ApplyCrossoverVisualEffects(CostumeInfo? targetCostume, CostumeInfo? replCostume, ModificationResult result)
    {
        if (replCostume == null) return;

        // 1. Process SwapCustomArts (e.g. SFX_Sword.swf/SwapCyberSam)
        foreach (var swap in replCostume.SwapCustomArts)
        {
            if (swap.StartsWith("SFX_", StringComparison.OrdinalIgnoreCase) && swap.Contains('/'))
            {
                var parts = swap.Split('/');
                string sfxFileName = parts[0];
                string swapSuffix = parts[1];
                string sfxPath = Path.Combine(_gamePath, sfxFileName);
                if (!File.Exists(sfxPath)) continue;

                SwfFile sfxSwf;
                using (var fs = new FileStream(sfxPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                {
                    sfxSwf = SwfFile.ReadFrom(fs);
                }

                var symClass = sfxSwf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
                if (symClass == null) continue;

                var swapSyms = symClass.References
                    .Where(r => r.SymbolName.EndsWith("_" + swapSuffix, StringComparison.OrdinalIgnoreCase))
                    .ToList();

                if (swapSyms.Count > 0)
                {
                    int remapped = 0;
                    foreach (var ss in swapSyms)
                    {
                        string basePrefix = ss.SymbolName[..ss.SymbolName.LastIndexOf('_')];
                        var baseSym = symClass.References.FirstOrDefault(r => r.SymbolName.Equals(basePrefix, StringComparison.OrdinalIgnoreCase));
                        if (baseSym != null)
                        {
                            baseSym.SymbolID = ss.SymbolID;
                            remapped++;
                        }

                        // Also remap target hero's signature symbol if cross-legend
                        if (targetCostume != null && !string.IsNullOrEmpty(targetCostume.OwnerHero) && !string.IsNullOrEmpty(replCostume.OwnerHero) &&
                            !string.Equals(targetCostume.OwnerHero, replCostume.OwnerHero, StringComparison.OrdinalIgnoreCase))
                        {
                            string targetSymName = basePrefix.Replace(replCostume.OwnerHero, targetCostume.OwnerHero, StringComparison.OrdinalIgnoreCase);
                            var targetSym = symClass.References.FirstOrDefault(r => r.SymbolName.Equals(targetSymName, StringComparison.OrdinalIgnoreCase));
                            if (targetSym != null)
                            {
                                targetSym.SymbolID = ss.SymbolID;
                                remapped++;
                            }
                        }
                    }

                    SaveSwfAtomically(sfxSwf, sfxPath);
                    result.OperationLog.Add($"Applied crossover SFX swaps in '{sfxFileName}' ({remapped} effects active).");
                }
            }
        }

        // 2. Process Dedicated SFX (e.g. SFX_DarthVader.swf, SFX_MasterChief.swf)
        string dedicatedSfxPath = Path.Combine(_gamePath, $"SFX_{replCostume.CostumeName}.swf");
        if (File.Exists(dedicatedSfxPath))
        {
            SwfFile srcSfx;
            using (var fs = new FileStream(dedicatedSfxPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            {
                srcSfx = SwfFile.ReadFrom(fs);
            }

            var srcSymClass = srcSfx.Tags.OfType<SymbolClassTag>().FirstOrDefault();
            if (srcSymClass != null && srcSymClass.References.Count > 0)
            {
                var relevantSfxFiles = GetCrossoverSfxFiles(targetCostume, replCostume);
                foreach (var targetSfxPath in relevantSfxFiles)
                {
                    if (!File.Exists(targetSfxPath) || string.Equals(targetSfxPath, dedicatedSfxPath, StringComparison.OrdinalIgnoreCase)) continue;

                    SwfFile dstSfx;
                    using (var fs = new FileStream(targetSfxPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                    {
                        dstSfx = SwfFile.ReadFrom(fs);
                    }

                    var dstSymClass = dstSfx.Tags.OfType<SymbolClassTag>().FirstOrDefault();
                    if (dstSymClass == null) continue;

                    string targetFileName = Path.GetFileName(targetSfxPath);
                    var matchingRefs = srcSymClass.References.Where(r => IsSymbolRelevantToSfx(r.SymbolName, targetFileName)).ToList();
                    if (matchingRefs.Count > 0)
                    {
                        PortAndRemapCrossoverSfx(srcSfx, dstSfx, matchingRefs, targetCostume, replCostume, result);
                        SaveSwfAtomically(dstSfx, targetSfxPath);
                        result.OperationLog.Add($"Ported {matchingRefs.Count} signature effects from '{Path.GetFileName(dedicatedSfxPath)}' into '{targetFileName}'.");
                    }
                }
            }
        }
    }

    private void PortAndRemapCrossoverSfx(
        SwfFile sourceSwf,
        SwfFile targetSwf,
        List<SwfSymbolReference> sourceRefs,
        CostumeInfo? targetCostume,
        CostumeInfo? replCostume,
        ModificationResult result)
    {
        var neededIds = new HashSet<ushort>();
        foreach (var r in sourceRefs)
        {
            CollectNeededCharacters(r.SymbolID, sourceSwf, neededIds);
        }

        ushort maxId = 0;
        foreach (var tag in targetSwf.Tags)
        {
            if (tag is DefineSpriteTag s) maxId = Math.Max(maxId, s.SpriteID);
            else if (tag is ShapeBaseTag sh) maxId = Math.Max(maxId, sh.ShapeID);
        }

        var idMap = new Dictionary<ushort, ushort>();
        foreach (var oldId in neededIds)
        {
            maxId++;
            idMap[oldId] = maxId;
        }

        var newShapes = new List<SwfTagBase>();
        foreach (var sh in sourceSwf.Tags.OfType<ShapeBaseTag>())
        {
            if (neededIds.Contains(sh.ShapeID))
            {
                newShapes.Add(CloneShapeWithNewId(sh, idMap[sh.ShapeID], []));
            }
        }

        var newSprites = new List<SwfTagBase>();
        foreach (var sp in sourceSwf.Tags.OfType<DefineSpriteTag>())
        {
            if (neededIds.Contains(sp.SpriteID))
            {
                newSprites.Add(CloneSpriteWithNewId(sp, idMap[sp.SpriteID], idMap));
            }
        }

        var targetSymClass = targetSwf.Tags.OfType<SymbolClassTag>().First();
        int insertIdx = targetSwf.Tags.IndexOf(targetSymClass);
        if (insertIdx < 0) insertIdx = targetSwf.Tags.Count;

        foreach (var sh in newShapes) targetSwf.Tags.Insert(insertIdx++, sh);
        foreach (var sp in newSprites) targetSwf.Tags.Insert(insertIdx++, sp);

        // Remap references
        foreach (var sr in sourceRefs)
        {
            ushort newSpriteId = idMap[sr.SymbolID];

            var existing = targetSymClass.References.FirstOrDefault(r => r.SymbolName.Equals(sr.SymbolName, StringComparison.OrdinalIgnoreCase));
            if (existing != null) existing.SymbolID = newSpriteId;
            else targetSymClass.References.Add(new SwfSymbolReference { SymbolID = newSpriteId, SymbolName = sr.SymbolName });

            int lastUnderscore = sr.SymbolName.LastIndexOf('_');
            if (lastUnderscore > 0)
            {
                string baseSymName = sr.SymbolName[..lastUnderscore];
                var baseRef = targetSymClass.References.FirstOrDefault(r => r.SymbolName.Equals(baseSymName, StringComparison.OrdinalIgnoreCase));
                if (baseRef != null) baseRef.SymbolID = newSpriteId;

                if (targetCostume != null && replCostume != null &&
                    !string.IsNullOrEmpty(targetCostume.OwnerHero) && !string.IsNullOrEmpty(replCostume.OwnerHero) &&
                    !string.Equals(targetCostume.OwnerHero, replCostume.OwnerHero, StringComparison.OrdinalIgnoreCase))
                {
                    string targetHeroSym = baseSymName.Replace(replCostume.OwnerHero, targetCostume.OwnerHero, StringComparison.OrdinalIgnoreCase);
                    var targetHeroRef = targetSymClass.References.FirstOrDefault(r => r.SymbolName.Equals(targetHeroSym, StringComparison.OrdinalIgnoreCase));
                    if (targetHeroRef != null) targetHeroRef.SymbolID = newSpriteId;
                }
            }
        }
    }
}
