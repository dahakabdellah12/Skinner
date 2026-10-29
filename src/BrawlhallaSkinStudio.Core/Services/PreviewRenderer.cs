using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Xml.Linq;
using SwfLib;
using SwfLib.Data;
using SwfLib.Tags;
using SwfLib.Tags.ControlTags;
using SwfLib.Tags.DisplayListTags;
using SwfLib.Tags.ShapeTags;
using SwiffCheese.Exporting.Svg;
using SwiffCheese.Shapes;
using SwiffCheese.Wrappers;

namespace BrawlhallaSkinStudio.Core.Services;

public static class PreviewRenderer
{
    private static readonly Dictionary<string, string> _svgCache = new(StringComparer.OrdinalIgnoreCase);
    private static readonly Dictionary<string, SwfFile> _swfFileCache = new(StringComparer.OrdinalIgnoreCase);

    public static void InvalidateCache(string? swfPath = null)
    {
        lock (_swfFileCache)
        {
            if (string.IsNullOrEmpty(swfPath))
            {
                _swfFileCache.Clear();
                _svgCache.Clear();
            }
            else
            {
                _swfFileCache.Remove(swfPath);
                string fileName = Path.GetFileName(swfPath);
                var keysToRemove = _svgCache.Keys
                    .Where(k => k.StartsWith(fileName + "#", StringComparison.OrdinalIgnoreCase))
                    .ToList();
                foreach (var k in keysToRemove)
                {
                    _svgCache.Remove(k);
                }
            }
        }
    }

    public static SwfFile? GetOrLoadSwf(string swfPath)
    {
        if (!File.Exists(swfPath)) return null;
        lock (_swfFileCache)
        {
            if (!_swfFileCache.TryGetValue(swfPath, out var swf))
            {
                using var fs = new FileStream(swfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
                swf = SwfFile.ReadFrom(fs);
                _swfFileCache[swfPath] = swf;
            }
            return swf;
        }
    }

    public static string? RenderSymbolToSvg(string swfPath, string symbolName, string? cacheFolder = null, Dictionary<uint, uint>? colorMap = null)
    {
        if (!File.Exists(swfPath)) return null;

        string colorKey = colorMap != null && colorMap.Count > 0
            ? "#c" + string.Join("_", colorMap.OrderBy(k => k.Key).Select(k => $"{k.Key:X6}:{k.Value:X6}"))
            : "";
        string cacheKey = $"{Path.GetFileName(swfPath)}#{symbolName}{colorKey}";
        if (_svgCache.TryGetValue(cacheKey, out var cached))
            return cached;

        try
        {
            var swf = GetOrLoadSwf(swfPath);
            if (swf == null) return null;

            var symClass = swf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
            if (symClass == null) return null;

            // Find matching reference with multiple fallback heuristics
            var symRef = symClass.References.FirstOrDefault(r => r.SymbolName.Equals(symbolName, StringComparison.OrdinalIgnoreCase))
                         ?? symClass.References.FirstOrDefault(r => r.SymbolName.EndsWith("_" + symbolName, StringComparison.OrdinalIgnoreCase))
                         ?? symClass.References.FirstOrDefault(r => r.SymbolName.Contains(symbolName, StringComparison.OrdinalIgnoreCase));

            if (symRef == null) return null;

            var placedShapes = new List<(ShapeBaseTag Shape, double a, double b, double c, double d, double tx, double ty)>();
            TraverseSprite(swf, symRef.SymbolID, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, placedShapes, new HashSet<ushort>());

            string? svgString = BuildSvgFromPlacedShapes(placedShapes, colorMap);
            if (svgString != null)
            {
                _svgCache[cacheKey] = svgString;

                if (!string.IsNullOrEmpty(cacheFolder))
                {
                    try
                    {
                        Directory.CreateDirectory(cacheFolder);
                        string outFile = Path.Combine(cacheFolder, $"{cacheKey.Replace("#", "_").Replace(".swf", "")}.svg");
                        File.WriteAllText(outFile, svgString);
                    }
                    catch { }
                }
            }

            return svgString;
        }
        catch
        {
            return null;
        }
    }

    public static string? RenderPairedWeaponToSvg(
        string gamePath,
        string baseWeapon,
        string symbol,
        string? swfFile,
        string? ownerHero,
        Dictionary<uint, uint>? colorMap = null)
    {
        string colorKey = colorMap != null && colorMap.Count > 0
            ? "#c" + string.Join("_", colorMap.OrderBy(k => k.Key).Select(k => $"{k.Key:X6}:{k.Value:X6}"))
            : "";
        string cacheKey = $"paired#{baseWeapon}#{symbol}#{swfFile}#{ownerHero}{colorKey}";
        if (_svgCache.TryGetValue(cacheKey, out var cached))
            return cached;

        string bw = baseWeapon?.Trim() ?? "";
        bool isKatar = bw.Equals("Katar", StringComparison.OrdinalIgnoreCase) || bw.Equals("Katars", StringComparison.OrdinalIgnoreCase);
        bool isBoots = bw.Equals("Boots", StringComparison.OrdinalIgnoreCase) || bw.Equals("BattleBoots", StringComparison.OrdinalIgnoreCase);
        bool isFists = bw.Equals("Fists", StringComparison.OrdinalIgnoreCase) || bw.Equals("Gauntlet", StringComparison.OrdinalIgnoreCase) || bw.Equals("Gauntlets", StringComparison.OrdinalIgnoreCase);

        if (!isKatar && !isBoots && !isFists)
            return null;

        var candidateSwfs = new List<string>();
        if (!string.IsNullOrEmpty(swfFile))
            candidateSwfs.Add(Path.Combine(gamePath, swfFile));
        if (!string.IsNullOrEmpty(ownerHero) && !ownerHero.Equals("--", StringComparison.OrdinalIgnoreCase))
        {
            string heroSwf = Path.Combine(gamePath, $"Gfx_{ownerHero}.swf");
            if (!candidateSwfs.Contains(heroSwf, StringComparer.OrdinalIgnoreCase))
                candidateSwfs.Add(heroSwf);
        }
        string[] standardWeapons =
        [
            Path.Combine(gamePath, "Gfx_Weapons.swf"),
            Path.Combine(gamePath, "Gfx_Weapons_02.swf"),
            Path.Combine(gamePath, "Gfx_Weapons_Sets.swf"),
            Path.Combine(gamePath, "Gfx_Weapons_OneOff.swf")
        ];
        foreach (var sw in standardWeapons)
        {
            if (!candidateSwfs.Contains(sw, StringComparer.OrdinalIgnoreCase))
                candidateSwfs.Add(sw);
        }

        try
        {
            var extra = Directory.GetFiles(gamePath, "Gfx_Weapons*.swf");
            foreach (var e in extra)
            {
                if (!candidateSwfs.Contains(e, StringComparer.OrdinalIgnoreCase))
                    candidateSwfs.Add(e);
            }
        }
        catch { }

        SwfFile? targetSwf = null;
        ushort mainId = 0;
        ushort secId = 0;

        if (isKatar)
        {
            string[] katarCandidates =
            [
                $"a_WeaponKatarTop_{symbol}",
                $"a_WeaponKatarBladeTop_{symbol}",
                $"a_WeaponKatar_{symbol}",
                $"a_WeaponKatarBlade_{symbol}",
                "a_WeaponKatarTop",
                "a_WeaponKatarBladeTop",
                "a_WeaponKatar"
            ];

            foreach (var file in candidateSwfs)
            {
                var swf = GetOrLoadSwf(file);
                if (swf == null) continue;
                var symClass = swf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
                if (symClass == null) continue;

                ushort foundId = FindSymbolId(symClass, katarCandidates);
                if (foundId > 0)
                {
                    mainId = foundId;
                    targetSwf = swf;
                    break;
                }
            }

            if (targetSwf != null && mainId > 0)
            {
                var placedShapes = new List<(ShapeBaseTag Shape, double a, double b, double c, double d, double tx, double ty)>();
                // Crossed pair: Left blade and Right blade
                TraverseSprite(targetSwf, mainId, -0.776107788, 0.630584717, 0.630584717, 0.776107788, 7.75 * 20.0, 0, placedShapes, new HashSet<ushort>());
                TraverseSprite(targetSwf, mainId, 0.776107788, 0.630584717, -0.630584717, 0.776107788, -7.75 * 20.0, 0, placedShapes, new HashSet<ushort>());

                string? svg = BuildSvgFromPlacedShapes(placedShapes, colorMap);
                if (svg != null)
                {
                    _svgCache[cacheKey] = svg;
                    return svg;
                }
            }

            string tooltipSwf = Path.Combine(gamePath, "UI_TooltipAnimations.swf");
            var fb = RenderSymbolToSvg(tooltipSwf, "a__TooltipAnimation_KatarPose", null, colorMap);
            if (fb != null) { _svgCache[cacheKey] = fb; return fb; }
        }
        else if (isBoots)
        {
            string[] frontCandidates =
            [
                $"a_WeaponBootsFront_{symbol}",
                $"a_WeaponBootsSide_{symbol}",
                "a_WeaponBootsFront",
                "a_WeaponBootsSide"
            ];
            string[] toeCandidates =
            [
                $"a_WeaponBootsToeFront_{symbol}",
                $"a_WeaponBootsToeSide_{symbol}",
                "a_WeaponBootsToeFront",
                "a_WeaponBootsToeSide"
            ];

            foreach (var file in candidateSwfs)
            {
                var swf = GetOrLoadSwf(file);
                if (swf == null) continue;
                var symClass = swf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
                if (symClass == null) continue;

                ushort foundFront = FindSymbolId(symClass, frontCandidates);
                if (foundFront > 0)
                {
                    mainId = foundFront;
                    secId = FindSymbolId(symClass, toeCandidates);
                    targetSwf = swf;
                    break;
                }
            }

            if (targetSwf != null && mainId > 0)
            {
                var placedShapes = new List<(ShapeBaseTag Shape, double a, double b, double c, double d, double tx, double ty)>();
                // Boot 1 (Left)
                TraverseSprite(targetSwf, mainId, -0.9076538, -0.5606995, -0.5606995, 0.9076538, 35.85 * 20.0, -21.55 * 20.0, placedShapes, new HashSet<ushort>());
                if (secId > 0)
                {
                    TraverseSprite(targetSwf, secId, -0.9076538, -0.5606995, -0.5606995, 0.9076538, 16.4 * 20.0, 19.7 * 20.0, placedShapes, new HashSet<ushort>());
                }

                // Boot 2 (Right)
                TraverseSprite(targetSwf, mainId, -0.9337769, -0.5160828, -0.5160828, 0.9337769, -23.05 * 20.0, -26.5 * 20.0, placedShapes, new HashSet<ushort>());
                if (secId > 0)
                {
                    TraverseSprite(targetSwf, secId, -0.9337769, -0.5160828, -0.5160828, 0.9337769, -40.4 * 20.0, 15.15 * 20.0, placedShapes, new HashSet<ushort>());
                }

                string? svg = BuildSvgFromPlacedShapes(placedShapes, colorMap);
                if (svg != null)
                {
                    _svgCache[cacheKey] = svg;
                    return svg;
                }
            }

            string tooltipSwf = Path.Combine(gamePath, "UI_TooltipAnimations.swf");
            var fb = RenderSymbolToSvg(tooltipSwf, "a__TooltipAnimation_BootsPose", null, colorMap);
            if (fb != null) { _svgCache[cacheKey] = fb; return fb; }
        }
        else if (isFists)
        {
            string[] fistCandidates =
            [
                $"a_WeaponFists01Large_{symbol}",
                $"a_WeaponFists01_{symbol}",
                $"a_WeaponFists03_{symbol}",
                $"a_WeaponFists02_{symbol}",
                $"a_WeaponFists07_{symbol}",
                $"a_WeaponFists_{symbol}",
                $"a_WeaponGauntlet_{symbol}",
                "a_WeaponFists01Large",
                "a_WeaponFists01"
            ];
            string[] forearmCandidates =
            [
                $"a_WeaponFistsForearmLarge_{symbol}",
                $"a_WeaponFistsForearm_{symbol}",
                $"a_WeaponFistsForearm2_{symbol}",
                "a_WeaponFistsForearmLarge",
                "a_WeaponFistsForearm"
            ];

            foreach (var file in candidateSwfs)
            {
                var swf = GetOrLoadSwf(file);
                if (swf == null) continue;
                var symClass = swf.Tags.OfType<SymbolClassTag>().FirstOrDefault();
                if (symClass == null) continue;

                ushort foundFist = FindSymbolId(symClass, fistCandidates);
                if (foundFist > 0)
                {
                    mainId = foundFist;
                    secId = FindSymbolId(symClass, forearmCandidates);
                    targetSwf = swf;
                    break;
                }
            }

            if (targetSwf != null && mainId > 0)
            {
                var placedShapes = new List<(ShapeBaseTag Shape, double a, double b, double c, double d, double tx, double ty)>();
                // Left Gauntlet
                if (secId > 0)
                {
                    TraverseSprite(targetSwf, secId, -0.36386108, 0.72775269, 0.73371887, 0.39523315, 35.05 * 20.0, -11.6 * 20.0, placedShapes, new HashSet<ushort>());
                }
                TraverseSprite(targetSwf, mainId, -0.48268127, 0.67320251, 0.66517639, 0.50666809, 34.5 * 20.0, -27.55 * 20.0, placedShapes, new HashSet<ushort>());

                // Right Gauntlet
                if (secId > 0)
                {
                    TraverseSprite(targetSwf, secId, -0.51438904, 0.59861755, 0.64282227, 0.58735657, -33.3 * 20.0, 6.75 * 20.0, placedShapes, new HashSet<ushort>());
                }
                TraverseSprite(targetSwf, mainId, -0.73551941, 0.46618652, -0.44372559, -0.74929810, -30.75 * 20.0, -6.55 * 20.0, placedShapes, new HashSet<ushort>());

                string? svg = BuildSvgFromPlacedShapes(placedShapes, colorMap);
                if (svg != null)
                {
                    _svgCache[cacheKey] = svg;
                    return svg;
                }
            }

            string tooltipSwf = Path.Combine(gamePath, "UI_TooltipAnimations.swf");
            var fb = RenderSymbolToSvg(tooltipSwf, "a__TooltipAnimation_FistsPose", null, colorMap);
            if (fb != null) { _svgCache[cacheKey] = fb; return fb; }
        }

        return null;
    }

    private static ushort FindSymbolId(SymbolClassTag symClass, IEnumerable<string> candidates)
    {
        foreach (var cand in candidates)
        {
            foreach (var r in symClass.References)
            {
                if (r.SymbolName == null) continue;
                if (r.SymbolName.Equals(cand, StringComparison.OrdinalIgnoreCase) ||
                    r.SymbolName.EndsWith("_" + cand, StringComparison.OrdinalIgnoreCase))
                {
                    return r.SymbolID;
                }
            }
        }
        return 0;
    }

    private static void TraverseSprite(
        SwfFile swf,
        ushort charId,
        double a, double b, double c, double d, double tx, double ty,
        List<(ShapeBaseTag Shape, double a, double b, double c, double d, double tx, double ty)> placedShapes,
        HashSet<ushort> visitedSprites)
    {
        var sprite = swf.Tags.OfType<DefineSpriteTag>().FirstOrDefault(s => s.SpriteID == charId);
        if (sprite != null)
        {
            if (!visitedSprites.Add(charId)) return;

            foreach (var t in sprite.Tags)
            {
                if (t is PlaceObject2Tag p2)
                {
                    double ca = p2.HasMatrix ? p2.Matrix.ScaleX : 1.0;
                    double cb = p2.HasMatrix ? p2.Matrix.RotateSkew0 : 0.0;
                    double cc = p2.HasMatrix ? p2.Matrix.RotateSkew1 : 0.0;
                    double cd = p2.HasMatrix ? p2.Matrix.ScaleY : 1.0;
                    double ctx = p2.HasMatrix ? p2.Matrix.TranslateX : 0.0;
                    double cty = p2.HasMatrix ? p2.Matrix.TranslateY : 0.0;

                    double combA = a * ca + c * cb;
                    double combB = b * ca + d * cb;
                    double combC = a * cc + c * cd;
                    double combD = b * cc + d * cd;
                    double combTx = a * ctx + c * cty + tx;
                    double combTy = b * ctx + d * cty + ty;

                    TraverseSprite(swf, p2.CharacterID, combA, combB, combC, combD, combTx, combTy, placedShapes, visitedSprites);
                }
                else if (t is PlaceObjectBaseTag pb)
                {
                    TraverseSprite(swf, pb.CharacterID, a, b, c, d, tx, ty, placedShapes, visitedSprites);
                }
            }
        }
        else
        {
            var sh = swf.Tags.OfType<ShapeBaseTag>().FirstOrDefault(s => s.ShapeID == charId);
            if (sh != null)
            {
                placedShapes.Add((sh, a, b, c, d, tx, ty));
            }
        }
    }

    private static string? BuildSvgFromPlacedShapes(
        List<(ShapeBaseTag Shape, double a, double b, double c, double d, double tx, double ty)> placedShapes,
        Dictionary<uint, uint>? colorMap = null)
    {
        if (placedShapes.Count == 0) return null;

        double minX = double.MaxValue, minY = double.MaxValue;
        double maxX = double.MinValue, maxY = double.MinValue;

        foreach (var (sh, a, b, c, d, tx, ty) in placedShapes)
        {
            double[] xs = [sh.ShapeBounds.XMin, sh.ShapeBounds.XMax, sh.ShapeBounds.XMin, sh.ShapeBounds.XMax];
            double[] ys = [sh.ShapeBounds.YMin, sh.ShapeBounds.YMin, sh.ShapeBounds.YMax, sh.ShapeBounds.YMax];

            for (int i = 0; i < 4; i++)
            {
                double px = (a * xs[i] + c * ys[i] + tx) / 20.0;
                double py = (b * xs[i] + d * ys[i] + ty) / 20.0;
                minX = Math.Min(minX, px);
                minY = Math.Min(minY, py);
                maxX = Math.Max(maxX, px);
                maxY = Math.Max(maxY, py);
            }
        }

        double margin = 2.0;
        double vbX = Math.Round(minX - margin, 2);
        double vbY = Math.Round(minY - margin, 2);
        double vbW = Math.Round(Math.Max(1.0, maxX - minX + margin * 2), 2);
        double vbH = Math.Round(Math.Max(1.0, maxY - minY + margin * 2), 2);

        var rootSvg = new XElement(XName.Get("svg", "http://www.w3.org/2000/svg"),
            new XAttribute("xmlns", "http://www.w3.org/2000/svg"),
            new XAttribute("viewBox", $"{vbX} {vbY} {vbW} {vbH}"),
            new XAttribute("width", "100%"),
            new XAttribute("height", "100%"),
            new XAttribute("preserveAspectRatio", "xMidYMid meet"),
            new XAttribute("shape-rendering", "geometricPrecision"),
            new XAttribute("text-rendering", "geometricPrecision"),
            new XAttribute("image-rendering", "crisp-edges"));

        foreach (var (sh, a, b, c, d, tx, ty) in placedShapes)
        {
            var shapeTag = new DefineShapeXTag(sh);
            var size = new SvgSize((int)vbW, (int)vbH);
            var mat = new SvgMatrix(a, b, c, d, tx / 20.0, ty / 20.0);
            var compiled = new SwfShape(shapeTag);
            var exporter = new SvgShapeExporter(size, mat, new SvgColorTransform());
            compiled.Export(exporter);

            if (exporter.Document.Root != null)
            {
                foreach (var child in exporter.Document.Root.Elements())
                {
                    rootSvg.Add(child);
                }
            }
        }

        if (colorMap != null && colorMap.Count > 0)
        {
            ApplyColorMapToSvg(rootSvg, colorMap);
        }

        return rootSvg.ToString();
    }

    public static void ApplyColorMapToSvg(XElement rootSvg, Dictionary<uint, uint> colorMap)
    {
        if (rootSvg == null || colorMap == null || colorMap.Count == 0) return;

        var hexMap = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var kvp in colorMap)
        {
            hexMap[$"#{kvp.Key:X6}"] = $"#{kvp.Value:X6}";
        }

        foreach (var el in rootSvg.DescendantsAndSelf())
        {
            var fillAttr = el.Attribute("fill");
            if (fillAttr != null && hexMap.TryGetValue(fillAttr.Value, out var newFill))
            {
                fillAttr.Value = newFill;
            }

            var strokeAttr = el.Attribute("stroke");
            if (strokeAttr != null && hexMap.TryGetValue(strokeAttr.Value, out var newStroke))
            {
                strokeAttr.Value = newStroke;
            }

            var stopAttr = el.Attribute("stop-color");
            if (stopAttr != null && hexMap.TryGetValue(stopAttr.Value, out var newStop))
            {
                stopAttr.Value = newStop;
            }
        }
    }
}
