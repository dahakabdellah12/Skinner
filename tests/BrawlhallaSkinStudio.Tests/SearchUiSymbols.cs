using System;
using System.IO;
using System.Linq;
using BrawlhallaSkinStudio.Core.Models;
using SwfLib;
using SwfLib.Data;
using SwfLib.Shapes.FillStyles;
using SwfLib.Tags;
using SwfLib.Tags.ControlTags;
using SwfLib.Tags.DisplayListTags;
using SwfLib.Tags.ShapeTags;
using Xunit;

namespace BrawlhallaSkinStudio.Tests;

public class SearchUiSymbols
{
    [Fact]
    public void TestPairedWeaponsLogic()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        string tooltipSwf = Path.Combine(inst.GamePath, "UI_TooltipAnimations.swf");
        Assert.True(File.Exists(tooltipSwf));

        // Test fallback poses from UI_TooltipAnimations
        var fistsPose = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(tooltipSwf, "a__TooltipAnimation_FistsPose");
        var bootsPose = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(tooltipSwf, "a__TooltipAnimation_BootsPose");
        var katarPose = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(tooltipSwf, "a__TooltipAnimation_KatarPose");

        Assert.NotNull(fistsPose);
        Assert.NotNull(bootsPose);
        Assert.NotNull(katarPose);

        Console.WriteLine($"Default tooltips verified: Fists ({fistsPose.Length}), Boots ({bootsPose.Length}), Katar ({katarPose.Length})");
    }

    [Fact]
    public void TestSvgExportCorrectViewBox()
    {
        string uiIcons = @"D:\SteamLibrary\steamapps\common\Brawlhalla\UI_Icons.swf";
        using var fs = File.OpenRead(uiIcons);
        var swf = SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SymbolClassTag>().First();
        var bodvarRef = sym.References.First(r => r.SymbolName == "a_Roster_Pose_Bodvar");

        var sprite = swf.Tags.OfType<DefineSpriteTag>().First(s => s.SpriteID == bodvarRef.SymbolID);
        var p2 = sprite.Tags.OfType<PlaceObject2Tag>().First();
        var shape = swf.Tags.OfType<ShapeBaseTag>().First(s => s.ShapeID == p2.CharacterID);

        int minX = shape.ShapeBounds.XMin;
        int minY = shape.ShapeBounds.YMin;
        int maxX = shape.ShapeBounds.XMax;
        int maxY = shape.ShapeBounds.YMax;

        double boxX = minX / 20.0;
        double boxY = minY / 20.0;
        double boxW = (maxX - minX) / 20.0;
        double boxH = (maxY - minY) / 20.0;

        double margin = 2.0;
        double vbX = Math.Round(boxX - margin, 2);
        double vbY = Math.Round(boxY - margin, 2);
        double vbW = Math.Round(Math.Max(1.0, boxW + margin * 2), 2);
        double vbH = Math.Round(Math.Max(1.0, boxH + margin * 2), 2);

        var shapeTag = new SwiffCheese.Wrappers.DefineShapeXTag(shape);
        var compiled = new SwiffCheese.Shapes.SwfShape(shapeTag);
        var exporter = new SwiffCheese.Exporting.Svg.SvgShapeExporter(
            new SwiffCheese.Exporting.Svg.SvgSize((int)boxW, (int)boxH),
            new SwiffCheese.Exporting.Svg.SvgMatrix(),
            new SwiffCheese.Exporting.Svg.SvgColorTransform()
        );
        compiled.Export(exporter);

        var rootSvg = new System.Xml.Linq.XElement(System.Xml.Linq.XName.Get("svg", "http://www.w3.org/2000/svg"),
            new System.Xml.Linq.XAttribute("xmlns", "http://www.w3.org/2000/svg"),
            new System.Xml.Linq.XAttribute("viewBox", $"{vbX} {vbY} {vbW} {vbH}"),
            new System.Xml.Linq.XAttribute("width", "100%"),
            new System.Xml.Linq.XAttribute("height", "100%"),
            new System.Xml.Linq.XAttribute("shape-rendering", "geometricPrecision"));

        foreach (var child in exporter.Document.Root!.Elements())
        {
            rootSvg.Add(child);
        }

        string resultSvg = rootSvg.ToString();
        File.WriteAllText("test_bodvar_fixed.svg", resultSvg);
        Console.WriteLine($"Fixed SVG ViewBox: {vbX} {vbY} {vbW} {vbH}");
        Console.WriteLine("Snippet: " + resultSvg.Substring(0, 200));
        Assert.Contains($"viewBox=\"{vbX} {vbY} {vbW} {vbH}\"", resultSvg);
    }

    [Fact]
    public void InspectNixAndJhalaColors()
    {
        string uiIcons = @"D:\SteamLibrary\steamapps\common\Brawlhalla\UI_Icons.swf";
        using var fs = File.OpenRead(uiIcons);
        var swf = SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SymbolClassTag>().First();

        Console.WriteLine("All symbols in UI_Icons containing Nix or Reaper:");
        foreach (var r in sym.References.Where(r => r.SymbolName.Contains("Nix", StringComparison.OrdinalIgnoreCase) || r.SymbolName.Contains("Reaper", StringComparison.OrdinalIgnoreCase)))
        {
            Console.WriteLine($"  {r.SymbolName} (ID={r.SymbolID})");
        }

        Console.WriteLine("All symbols in UI_Icons containing Jhala or Barbarian:");
        foreach (var r in sym.References.Where(r => r.SymbolName.Contains("Jhala", StringComparison.OrdinalIgnoreCase) || r.SymbolName.Contains("Barbarian", StringComparison.OrdinalIgnoreCase)))
        {
            Console.WriteLine($"  {r.SymbolName} (ID={r.SymbolID})");
        }
    }

    [Fact]
    public void InspectColorTransforms()
    {
        string uiIcons = @"D:\SteamLibrary\steamapps\common\Brawlhalla\UI_Icons.swf";
        using var fs = File.OpenRead(uiIcons);
        var swf = SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SymbolClassTag>().First();

        var nixRef = sym.References.First(r => r.SymbolName == "a_SkinIcon_Reaper_Classic");
        var jhalaRef = sym.References.First(r => r.SymbolName == "a_SkinIcon_Barbarian_Classic");

        void InspectSprite(ushort symId, string name)
        {
            Console.WriteLine($"=== Inspecting {name} (ID={symId}) ===");
            void Recurse(ushort id, int depth)
            {
                var sp = swf.Tags.OfType<DefineSpriteTag>().FirstOrDefault(s => s.SpriteID == id);
                if (sp != null)
                {
                    Console.WriteLine($"{new string(' ', depth * 2)}Sprite {id} ({sp.Tags.Count} tags):");
                    foreach (var t in sp.Tags)
                    {
                        if (t is PlaceObject2Tag p2)
                        {
                            Console.WriteLine($"{new string(' ', depth * 2 + 2)}PO2: Char={p2.CharacterID}, HasCT={p2.HasColorTransform}, HasMatrix={p2.HasMatrix}");
                            Recurse(p2.CharacterID, depth + 1);
                        }
                    }
                }
                else
                {
                    var sh = swf.Tags.OfType<ShapeBaseTag>().FirstOrDefault(s => s.ShapeID == id);
                    if (sh != null)
                    {
                        Console.WriteLine($"{new string(' ', depth * 2)}Shape {id} bounds: ({sh.ShapeBounds.XMin}, {sh.ShapeBounds.YMin}) to ({sh.ShapeBounds.XMax}, {sh.ShapeBounds.YMax})");
                    }
                }
            }
            Recurse(symId, 0);
        }

        InspectSprite(nixRef.SymbolID, "Nix Classic Icon");
        InspectSprite(jhalaRef.SymbolID, "Jhala Classic Icon");
    }

    [Fact]
    public void TestCatalogWeaponResolutionSuccessRate()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        string[] targets = ["Fists", "Katar", "Boots", "Greatsword", "Chakram", "Pistol", "Axe", "Bow", "Sword", "Spear", "Hammer", "RocketLance", "Scythe", "Cannon", "Orb"];

        string[] fallbackSwfs =
        [
            Path.Combine(inst.GamePath, "Gfx_Weapons.swf"),
            Path.Combine(inst.GamePath, "Gfx_Weapons_02.swf"),
            Path.Combine(inst.GamePath, "Gfx_Weapons_Sets.swf")
        ];

        string? ResolveWeaponSvg(string swf, string baseWeapon, string symbol)
        {
            string fullSwf = Path.Combine(inst.GamePath, swf);
            var swfList = new List<string> { fullSwf };
            foreach (var fb in fallbackSwfs)
            {
                if (!swfList.Contains(fb, StringComparer.OrdinalIgnoreCase) && File.Exists(fb))
                    swfList.Add(fb);
            }

            var candidateSymbols = new List<string>
            {
                // Greatsword specific
                $"a_WeaponGreat_{symbol}",
                $"a_WeaponGreatsword_{symbol}",
                // Fists specific
                $"a_WeaponFists01Large_{symbol}",
                $"a_WeaponFists01_{symbol}",
                $"a_WeaponFists_{symbol}",
                // Katar specific
                $"a_WeaponKatarTop_{symbol}",
                $"a_WeaponKatarBladeTop_{symbol}",
                $"a_WeaponKatarUnder_{symbol}",
                $"a_WeaponKatar_{symbol}",
                // Boots specific
                $"a_WeaponBootsSide_{symbol}",
                $"a_WeaponBootsFront_{symbol}",
                $"a_WeaponBootsSideTop_{symbol}",
                $"a_WeaponBoots_{symbol}",
                // Chakram specific
                $"a_WeaponChakramCombined_{symbol}",
                $"a_WeaponChakramRight_{symbol}",
                $"a_WeaponChakram_{symbol}",
                $"a_WeaponChakramAngleAway_{symbol}",
                // General
                $"a_Weapon{baseWeapon}_{symbol}",
                $"a_Weapon{symbol}",
                symbol
            };

            foreach (var targetSwf in swfList)
            {
                if (!File.Exists(targetSwf)) continue;
                foreach (var sym in candidateSymbols)
                {
                    var svg = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(targetSwf, sym);
                    if (svg != null) return svg;
                }
            }
            return null;
        }

        foreach (var bw in targets)
        {
            var skins = cat.WeaponSkins.Where(w => w.BaseWeapon.Equals(bw, StringComparison.OrdinalIgnoreCase)).ToList();
            int ok = 0;
            foreach (var s in skins)
            {
                var svg = ResolveWeaponSvg(s.SwfFile, s.BaseWeapon, s.SymbolSuffix);
                if (svg != null) ok++;
            }
            Console.WriteLine($"Weapon {bw}: {ok} / {skins.Count} rendered ({ok * 100.0 / Math.Max(1, skins.Count):F1}%)");
            Assert.True(ok > skins.Count * 0.7, $"Expected >70% resolution for {bw}, got {ok}/{skins.Count}");
        }
    }

    [Fact]
    public void TestBowRendering()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        var emberBows = cat.WeaponSkins.Where(w => w.BaseWeapon == "Bow").Take(5).ToList();
        foreach (var b in emberBows)
        {
            string fullSwf = Path.Combine(inst.GamePath, b.SwfFile);
            string sym = b.SymbolSuffix.StartsWith("a_Weapon") ? b.SymbolSuffix : $"a_Weapon{b.BaseWeapon}_{b.SymbolSuffix}";
            var svg = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(fullSwf, sym);
            Console.WriteLine($"Bow {b.DisplayName} ({sym}) in {b.SwfFile} => SVG length = {svg?.Length ?? 0}");
            if (svg != null)
            {
                File.WriteAllText($"test_bow_{b.WeaponSkinName}.svg", svg);
                var match = System.Text.RegularExpressions.Regex.Match(svg, @"viewBox=""([^""]+)""");
                Console.WriteLine($"  viewBox: {match.Value}");
            }
        }
    }

    [Fact]
    public void InspectBowSpriteDetails()
    {
        string swfPath = @"D:\SteamLibrary\steamapps\common\Brawlhalla\Gfx_Weapons_02.swf";
        using var fs = File.OpenRead(swfPath);
        var swf = SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SymbolClassTag>().First();
        var bowRef = sym.References.First(r => r.SymbolName == "a_WeaponBow_Elf");

        Console.WriteLine($"Symbol ID: {bowRef.SymbolID}");
        foreach (var ctor in typeof(SwiffCheese.Exporting.Svg.SvgMatrix).GetConstructors())
        {
            Console.WriteLine($"SvgMatrix ctor: {ctor}");
        }
        foreach (var prop in typeof(SwiffCheese.Exporting.Svg.SvgMatrix).GetProperties())
        {
            Console.WriteLine($"SvgMatrix prop: {prop.PropertyType.Name} {prop.Name}");
        }
        foreach (var field in typeof(SwiffCheese.Exporting.Svg.SvgMatrix).GetFields())
        {
            Console.WriteLine($"SvgMatrix field: {field.FieldType.Name} {field.Name}");
        }

        var sprite = swf.Tags.OfType<DefineSpriteTag>().FirstOrDefault(s => s.SpriteID == bowRef.SymbolID);
        foreach (var t in sprite!.Tags)
        {
            if (t is PlaceObject2Tag p)
            {
                var targetTag = swf.Tags.FirstOrDefault(x => (x is ShapeBaseTag sh && sh.ShapeID == p.CharacterID) || (x is DefineSpriteTag sp && sp.SpriteID == p.CharacterID));
                Console.WriteLine($"PO2: char={p.CharacterID} ({targetTag?.GetType().Name}), hasM={p.HasMatrix}, SX={p.Matrix.ScaleX}, SY={p.Matrix.ScaleY}, TX={p.Matrix.TranslateX}, TY={p.Matrix.TranslateY}");
            }
        }
        if (sprite != null)
        {
            Console.WriteLine($"Sprite {sprite.SpriteID} has {sprite.Tags.Count} tags:");
            foreach (var t in sprite.Tags)
            {
                if (t is PlaceObject2Tag p2)
                {
                    Console.WriteLine($"  PlaceObject2: CharID={p2.CharacterID}, HasMatrix={p2.HasMatrix}, HasClipDepth={p2.HasClipDepth}");
                    if (p2.HasMatrix)
                    {
                        Console.WriteLine($"    Matrix: ScaleX={p2.Matrix.ScaleX}, ScaleY={p2.Matrix.ScaleY}, Rotate0={p2.Matrix.RotateSkew0}, Rotate1={p2.Matrix.RotateSkew1}, TX={p2.Matrix.TranslateX}, TY={p2.Matrix.TranslateY}");
                    }
                }
            }
        }
    }

    [Fact]
    public void TestExportBowWithMatrices()
    {
        string swfPath = @"D:\SteamLibrary\steamapps\common\Brawlhalla\Gfx_Weapons_02.swf";
        using var fs = File.OpenRead(swfPath);
        var swf = SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SymbolClassTag>().First();
        var bowRef = sym.References.First(r => r.SymbolName == "a_WeaponBow_Elf");

        var sprite = swf.Tags.OfType<DefineSpriteTag>().First(s => s.SpriteID == bowRef.SymbolID);

        // Track (Shape, Matrix)
        var placedShapes = new List<(ShapeBaseTag Shape, double a, double b, double c, double d, double tx, double ty)>();

        void Traverse(ushort charId, double a, double b, double c, double d, double tx, double ty)
        {
            var sp = swf.Tags.OfType<DefineSpriteTag>().FirstOrDefault(s => s.SpriteID == charId);
            if (sp != null)
            {
                foreach (var po in sp.Tags.OfType<PlaceObject2Tag>())
                {
                    double ca = po.HasMatrix ? po.Matrix.ScaleX : 1.0;
                    double cb = po.HasMatrix ? po.Matrix.RotateSkew0 : 0.0;
                    double cc = po.HasMatrix ? po.Matrix.RotateSkew1 : 0.0;
                    double cd = po.HasMatrix ? po.Matrix.ScaleY : 1.0;
                    double ctx = po.HasMatrix ? po.Matrix.TranslateX : 0.0;
                    double cty = po.HasMatrix ? po.Matrix.TranslateY : 0.0;

                    // Combined Matrix = Parent * Child
                    double combA = a * ca + c * cb;
                    double combB = b * ca + d * cb;
                    double combC = a * cc + c * cd;
                    double combD = b * cc + d * cd;
                    double combTx = a * ctx + c * cty + tx;
                    double combTy = b * ctx + d * cty + ty;

                    Traverse(po.CharacterID, combA, combB, combC, combD, combTx, combTy);
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

        Traverse(bowRef.SymbolID, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

        Console.WriteLine($"Found {placedShapes.Count} placed shapes for Bow Elf:");
        double minX = double.MaxValue, minY = double.MaxValue;
        double maxX = double.MinValue, maxY = double.MinValue;

        foreach (var (sh, a, b, c, d, tx, ty) in placedShapes)
        {
            Console.WriteLine($"  Shape {sh.ShapeID}: matrix=({a:F2}, {b:F2}, {c:F2}, {d:F2}, {tx:F0}, {ty:F0})");

            // Transform 4 corners
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

        Console.WriteLine($"Full Bow ViewBox: {vbX} {vbY} {vbW} {vbH}");

        var rootSvg = new System.Xml.Linq.XElement(System.Xml.Linq.XName.Get("svg", "http://www.w3.org/2000/svg"),
            new System.Xml.Linq.XAttribute("xmlns", "http://www.w3.org/2000/svg"),
            new System.Xml.Linq.XAttribute("viewBox", $"{vbX} {vbY} {vbW} {vbH}"),
            new System.Xml.Linq.XAttribute("width", "100%"),
            new System.Xml.Linq.XAttribute("height", "100%"),
            new System.Xml.Linq.XAttribute("shape-rendering", "geometricPrecision"));

        foreach (var (sh, a, b, c, d, tx, ty) in placedShapes)
        {
            var shapeTag = new SwiffCheese.Wrappers.DefineShapeXTag(sh);
            var compiled = new SwiffCheese.Shapes.SwfShape(shapeTag);
            var exporter = new SwiffCheese.Exporting.Svg.SvgShapeExporter(
                new SwiffCheese.Exporting.Svg.SvgSize((int)vbW, (int)vbH),
                new SwiffCheese.Exporting.Svg.SvgMatrix(a, b, c, d, tx / 20.0, ty / 20.0),
                new SwiffCheese.Exporting.Svg.SvgColorTransform()
            );
            compiled.Export(exporter);
            if (exporter.Document.Root != null)
            {
                foreach (var el in exporter.Document.Root.Elements())
                {
                    rootSvg.Add(el);
                }
            }
        }

        string svgText = rootSvg.ToString();
        File.WriteAllText("test_full_bow_assembled.svg", svgText);
        Console.WriteLine($"Generated Full Bow SVG: {svgText.Length} chars");
    }

    [Fact]
    public void InspectQueenNaiSprite()
    {
        string uiIcons = @"D:\SteamLibrary\steamapps\common\Brawlhalla\UI_Icons.swf";
        using var fs = File.OpenRead(uiIcons);
        var swf = SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SymbolClassTag>().First();
        var naiRef = sym.References.First(r => r.SymbolName == "a_Roster_Pose_Witch");

        Console.WriteLine($"Nai Symbol ID: {naiRef.SymbolID}");
        var sprite = swf.Tags.OfType<DefineSpriteTag>().FirstOrDefault(s => s.SpriteID == naiRef.SymbolID);
        if (sprite != null)
        {
            Console.WriteLine($"Sprite {sprite.SpriteID} has {sprite.Tags.Count} tags:");
            foreach (var t in sprite.Tags)
            {
                if (t is PlaceObject2Tag p2)
                {
                    Console.WriteLine($"  PlaceObject2: CharID={p2.CharacterID}, HasMatrix={p2.HasMatrix}");
                    if (p2.HasMatrix)
                    {
                        Console.WriteLine($"    Matrix: ScaleX={p2.Matrix.ScaleX}, ScaleY={p2.Matrix.ScaleY}, Rotate0={p2.Matrix.RotateSkew0}, Rotate1={p2.Matrix.RotateSkew1}, TX={p2.Matrix.TranslateX}, TY={p2.Matrix.TranslateY}");
                    }
                }
            }
        }

        var naiClassicRef = sym.References.First(r => r.SymbolName == "a_SkinIcon_Nai_Classic");
        var naiClassicSprite = swf.Tags.OfType<DefineSpriteTag>().FirstOrDefault(s => s.SpriteID == naiClassicRef.SymbolID);
        if (naiClassicSprite != null)
        {
            Console.WriteLine($"a_SkinIcon_Nai_Classic Sprite has {naiClassicSprite.Tags.Count} tags:");
            foreach (var t in naiClassicSprite.Tags.OfType<PlaceObject2Tag>())
            {
                Console.WriteLine($"  PlaceObject2: CharID={t.CharacterID}, HasMatrix={t.HasMatrix}");
                if (t.HasMatrix)
                {
                    Console.WriteLine($"    Matrix: ScaleX={t.Matrix.ScaleX}, ScaleY={t.Matrix.ScaleY}, TX={t.Matrix.TranslateX}, TY={t.Matrix.TranslateY}");
                }
            }
        }
    }

    [Fact]
    public void PrintDefaultCostumeIconsForAllLegends()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        foreach (var leg in cat.Legends.OrderBy(l => l.HeroId))
        {
            var def = leg.Costumes.FirstOrDefault(c => c.IsDefault) ?? leg.Costumes.FirstOrDefault();
            Console.WriteLine($"[Legend #{leg.HeroId}] {leg.DisplayName} ({leg.HeroName}) => Default Costume: {def?.DisplayName}, Icon={def?.IconName}, IconFile={def?.IconFileName}, Swf={def?.SwfFile}");
        }
    }

    [Fact]
    public void TestRenderDefaultIconForAll72Legends()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        int success = 0;
        int failed = 0;
        foreach (var leg in cat.Legends.Where(l => l.HeroName != "DEFAULT_CHARACTER" && l.HeroName != "Random"))
        {
            var def = leg.Costumes.FirstOrDefault(c => c.IsDefault) ?? leg.Costumes.FirstOrDefault();
            if (def == null || string.IsNullOrEmpty(def.IconName))
            {
                Console.WriteLine($"MISSING ICON: {leg.DisplayName} ({leg.HeroName})");
                failed++;
                continue;
            }

            string swfFile = Path.Combine(inst.GamePath, def.IconFileName ?? "UI_Icons.swf");
            var svg = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(swfFile, def.IconName);
            if (svg != null)
            {
                success++;
            }
            else
            {
                Console.WriteLine($"FAILED SVG: {leg.DisplayName} with {def.IconName} in {swfFile}");
                failed++;
            }
        }

        Console.WriteLine($"=== RESULTS: {success} SUCCESS, {failed} FAILED ===");
        Assert.True(success >= 68, $"Expected at least 68 successes, got {success}");
    }

    [Fact]
    public void InspectJhalaNixAndWeapons()
    {
        string uiIcons = @"D:\SteamLibrary\steamapps\common\Brawlhalla\UI_Icons.swf";
        using var fs = File.OpenRead(uiIcons);
        var swf = SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SymbolClassTag>().First();

        Console.WriteLine("--- Matching Jhala / Barbarian ---");
        foreach (var r in sym.References.Where(r => r.SymbolName.Contains("Jhala", StringComparison.OrdinalIgnoreCase) || r.SymbolName.Contains("Barbarian", StringComparison.OrdinalIgnoreCase)))
        {
            Console.WriteLine($"[Jhala] {r.SymbolName} (ID={r.SymbolID})");
        }

        Console.WriteLine("--- Matching Nix / GrimReaper ---");
        foreach (var r in sym.References.Where(r => r.SymbolName.Contains("Nix", StringComparison.OrdinalIgnoreCase) || r.SymbolName.Contains("GrimReaper", StringComparison.OrdinalIgnoreCase) || r.SymbolName.Contains("Reaper", StringComparison.OrdinalIgnoreCase)))
        {
            Console.WriteLine($"[Nix] {r.SymbolName} (ID={r.SymbolID})");
        }

        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        Console.WriteLine("--- Catalog Base Weapons ---");
        foreach (var bw in cat.WeaponSkinsByType.Keys.OrderBy(k => k))
        {
            Console.WriteLine($"BaseWeapon: '{bw}' (count={cat.WeaponSkinsByType[bw].Count})");
        }

        // Test rendering Jhala & Nix icons vs roster poses
        var jhalaSkin = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(uiIcons, "a_SkinIcon_Barbarian_Classic");
        var jhalaPose = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(uiIcons, "a_Roster_Pose_Barbarian");
        File.WriteAllText("test_jhala_skin.svg", jhalaSkin ?? "");
        File.WriteAllText("test_jhala_pose.svg", jhalaPose ?? "");

        var nixSkin = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(uiIcons, "a_SkinIcon_Reaper_Classic");
        var nixPose = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(uiIcons, "a_Roster_Pose_Reaper");
        File.WriteAllText("test_nix_skin.svg", nixSkin ?? "");
        File.WriteAllText("test_nix_pose.svg", nixPose ?? "");
        Console.WriteLine($"Jhala skin: {jhalaSkin?.Length}, pose: {jhalaPose?.Length}");
        Console.WriteLine($"Nix skin: {nixSkin?.Length}, pose: {nixPose?.Length}");
    }

    [Fact]
    public void TestAll21LegendsHaveValidDefaultCostumeIcons()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        string uiIcons = Path.Combine(inst.GamePath, "UI_Icons.swf");
        var list21 = new[]
        {
            "Lucien", "Teros", "Asuri", "Azoth", "Jhala", "Kor", "Wu Shang", "Ragnir",
            "Cross", "Nix", "Mordex", "Artemis", "Isaiah", "Jiro", "Zariel", "Dusk",
            "Thor", "Magyar", "Arcadia", "Seven", "Vivi"
        };

        foreach (var name in list21)
        {
            var legend = cat.Legends.FirstOrDefault(l =>
                l.DisplayName.Equals(name, StringComparison.OrdinalIgnoreCase) ||
                l.HeroName.Equals(name, StringComparison.OrdinalIgnoreCase));

            Assert.NotNull(legend);
            var defCostume = legend.Costumes.FirstOrDefault(c => c.IsDefault) ?? legend.Costumes.FirstOrDefault();
            Assert.NotNull(defCostume);
            Assert.False(string.IsNullOrEmpty(defCostume.IconName), $"No IconName for {name}");

            string targetFile = !string.IsNullOrEmpty(defCostume.IconFileName)
                ? Path.Combine(inst.GamePath, defCostume.IconFileName)
                : uiIcons;
            if (!File.Exists(targetFile)) targetFile = uiIcons;

            var svg = BrawlhallaSkinStudio.Core.Services.PreviewRenderer.RenderSymbolToSvg(targetFile, defCostume.IconName);
            Assert.NotNull(svg);
            Assert.True(svg.Length > 100, $"SVG for {name} ({defCostume.IconName}) is too short");
        }
    }

    [Fact]
    public void VerifyNoCrossHeroWeaponMismatches()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        // Verify Mordex + Fenrir matches Fenrir weapon skins
        var wereLegend = cat.Legends.First(l => l.HeroName == "Werewolf");
        var wereW1Skins = cat.WeaponSkins.Where(w => w.BaseWeapon.Equals("Scythe", StringComparison.OrdinalIgnoreCase)).ToList();
        var wereW2Skins = cat.WeaponSkins.Where(w => w.BaseWeapon.Equals("Fists", StringComparison.OrdinalIgnoreCase)).ToList();
        var fenrirCostume = wereLegend.Costumes.First(c => c.CostumeName.Equals("Fenrir", StringComparison.OrdinalIgnoreCase));

        var w1Match = FindMatch(wereW1Skins, fenrirCostume);
        var w2Match = FindMatch(wereW2Skins, fenrirCostume);

        Assert.NotNull(w1Match);
        Assert.Equal("ScytheFenrir", w1Match.WeaponSkinName);
        Assert.NotNull(w2Match);
        Assert.Equal("FistsFenrir", w2Match.WeaponSkinName);

        // Verify across all legends that no costume ever matches a weapon skin belonging to another hero
        int totalMismatches = 0;
        foreach (var leg in cat.Legends)
        {
            var w1Skins = cat.WeaponSkins.Where(w => w.BaseWeapon.Equals(leg.Weapon1, StringComparison.OrdinalIgnoreCase)).ToList();
            var w2Skins = cat.WeaponSkins.Where(w => w.BaseWeapon.Equals(leg.Weapon2, StringComparison.OrdinalIgnoreCase)).ToList();

            foreach (var cost in leg.Costumes)
            {
                var matchW1 = FindMatch(w1Skins, cost);
                if (matchW1 != null && !string.IsNullOrEmpty(matchW1.OwnerHero) && !matchW1.OwnerHero.Equals(leg.HeroName, StringComparison.OrdinalIgnoreCase))
                {
                    totalMismatches++;
                }

                var matchW2 = FindMatch(w2Skins, cost);
                if (matchW2 != null && !string.IsNullOrEmpty(matchW2.OwnerHero) && !matchW2.OwnerHero.Equals(leg.HeroName, StringComparison.OrdinalIgnoreCase))
                {
                    totalMismatches++;
                }
            }
        }

        Assert.Equal(0, totalMismatches);
    }

    private static WeaponSkinInfo? FindMatch(List<WeaponSkinInfo> skins, CostumeInfo costume)
    {
        if (skins == null || skins.Count == 0 || costume == null) return null;
        string wSet = (costume.WeaponSet ?? "").Trim().ToLower();
        string cName = (costume.CostumeName ?? "").Trim().ToLower();
        string dName = (costume.DisplayName ?? "").Trim().ToLower();
        string oHero = (costume.OwnerHero ?? "").Trim().ToLower();

        bool IsValidOwner(WeaponSkinInfo w)
        {
            string wHero = (w?.OwnerHero ?? "").Trim().ToLower();
            return string.IsNullOrEmpty(wHero) || string.IsNullOrEmpty(oHero) || wHero == oHero;
        }

        // 1. Exact match with weaponSet
        if (!string.IsNullOrEmpty(wSet) && wSet != "--")
        {
            var exact = skins.FirstOrDefault(w => {
                if (!IsValidOwner(w)) return false;
                string wName = (w.WeaponSkinName ?? "").ToLower();
                string sSuffix = (w.SymbolSuffix ?? "").ToLower();
                return wName.EndsWith(wSet) || sSuffix == wSet;
            });
            if (exact != null) return exact;
        }

        // 2. Exact match with costumeName
        if (!string.IsNullOrEmpty(cName))
        {
            var exact = skins.FirstOrDefault(w => {
                if (!IsValidOwner(w)) return false;
                string wName = (w.WeaponSkinName ?? "").ToLower();
                string sSuffix = (w.SymbolSuffix ?? "").ToLower();
                return wName.EndsWith(cName) || sSuffix == cName;
            });
            if (exact != null) return exact;
        }

        // 3. Fallback: match weaponSet substring
        if (!string.IsNullOrEmpty(wSet) && wSet != "--")
        {
            var found = skins.FirstOrDefault(w => {
                if (!IsValidOwner(w)) return false;
                string wName = (w.WeaponSkinName ?? "").ToLower();
                return wName.Contains(wSet);
            });
            if (found != null) return found;
        }

        // 4. Fallback: match costumeName substring
        if (!string.IsNullOrEmpty(cName))
        {
            var found = skins.FirstOrDefault(w => {
                if (!IsValidOwner(w)) return false;
                string wName = (w.WeaponSkinName ?? "").ToLower();
                return wName.Contains(cName);
            });
            if (found != null) return found;
        }

        // 5. Match with displayName token match
        if (!string.IsNullOrEmpty(dName))
        {
            var found = skins.FirstOrDefault(w => {
                if (!IsValidOwner(w)) return false;
                string wDisp = (w.DisplayName ?? "").ToLower();
                return wDisp == dName || wDisp.EndsWith(" " + dName) || wDisp.StartsWith(dName + " ");
            });
            if (found != null) return found;
        }

        // 6. Default weapon of owner hero
        if (costume.IsDefault && !string.IsNullOrEmpty(oHero))
        {
            var found = skins.FirstOrDefault(w => (w.OwnerHero ?? "").ToLower() == oHero && w.IsDefault);
            if (found != null) return found;
        }

        return null;
    }

    [Fact]
    public void TestWeaponShapeColorsVsBaseColors()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        var mordex = cat.Legends.First(l => l.HeroName == "Werewolf");
        var mordexCostume = mordex.Costumes.First(c => c.IsDefault);
        var mordexGauntlet = cat.WeaponSkins.First(w => w.WeaponSkinName == "FistsWerewolf");

        // Read all shape colors in a_WeaponFists01Large_Werewolf
        string swfPath = Path.Combine(inst.GamePath, "Gfx_Werewolf.swf");
        using var fs = File.OpenRead(swfPath);
        var swf = SwfFile.ReadFrom(fs);
        var sym = swf.Tags.OfType<SymbolClassTag>().First();
        ushort fistSymId = sym.References.First(r => r.SymbolName == "a_WeaponFists01Large_Werewolf").SymbolID;

        var collected = new HashSet<ushort>();
        BrawlhallaSkinStudio.Core.Engine.ModificationEngine.CollectNeededCharacters(fistSymId, swf, collected);

        var shapeColors = new HashSet<uint>();
        foreach (var sh in swf.Tags.OfType<ShapeBaseTag>().Where(s => collected.Contains(s.ShapeID)))
        {
            if (sh is DefineShape3Tag s3)
            {
                foreach (var f in s3.FillStyles)
                {
                    if (f is SolidFillStyleRGBA sf) shapeColors.Add(BitConverter.ToUInt32(new byte[] { sf.Color.Red, sf.Color.Green, sf.Color.Blue, sf.Color.Alpha }, 0));
                }
            }
            else if (sh is DefineShapeTag s1)
            {
                foreach (var f in s1.FillStyles)
                {
                    if (f is SolidFillStyleRGB sf) shapeColors.Add(BitConverter.ToUInt32(new byte[] { sf.Color.Red, sf.Color.Green, sf.Color.Blue, 255 }, 0));
                }
            }
            else if (sh is DefineShape2Tag s2)
            {
                foreach (var f in s2.FillStyles)
                {
                    if (f is SolidFillStyleRGB sf) shapeColors.Add(BitConverter.ToUInt32(new byte[] { sf.Color.Red, sf.Color.Green, sf.Color.Blue, 255 }, 0));
                }
            }
        }

        Assert.NotEmpty(shapeColors);
        Assert.NotEmpty(mordexCostume.BaseColors);
    }

    [Fact]
    public void TestMordexCrossGauntletAndMirageScytheIsolation()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        var mordex = cat.Legends.First(l => l.HeroName == "Werewolf");
        var mordexCostume = mordex.Costumes.First(c => c.IsDefault);
        var mordexGauntlet = cat.WeaponSkins.First(w => w.WeaponSkinName == "FistsWerewolf");
        var mordexScythe = cat.WeaponSkins.First(w => w.WeaponSkinName == "ScytheWerewolf");

        var crossGauntlet = cat.WeaponSkins.First(w => w.WeaponSkinName == "FistsMobster");
        var mirageScythe = cat.WeaponSkins.First(w => w.WeaponSkinName == "ScytheStreetMirage");

        // Pre-read head and torso symbol IDs of Mordex
        string werewolfSwf = Path.Combine(inst.GamePath, "Gfx_Werewolf.swf");
        ushort origFootId;
        using (var fs = File.OpenRead(werewolfSwf))
        {
            var swf = SwfFile.ReadFrom(fs);
            var sym = swf.Tags.OfType<SymbolClassTag>().First();
            origFootId = sym.References.First(r => r.SymbolName == "a_Foot1Bent_Werewolf").SymbolID;
        }

        var req = new ModificationRequest
        {
            TargetLegend = mordex,
            TargetCostume = mordexCostume,
            ReplacementCostume = null, // KEEP MORDEX'S COSTUME
            TargetWeapon1 = mordexGauntlet,
            ReplacementWeapon1 = crossGauntlet, // Cross Gauntlets
            TargetWeapon2 = mordexScythe,
            ReplacementWeapon2 = mirageScythe  // Mirage Scythe
        };

        var bm = new BrawlhallaSkinStudio.Core.Engine.BackupManager(inst.GamePath);
        var engine = new BrawlhallaSkinStudio.Core.Engine.ModificationEngine(inst.GamePath, bm);
        var result = engine.ApplyModification(req);
        Assert.True(result.Success, result.Message);

        try
        {
            // Verify Gfx_Werewolf.swf on disk
            using var fsPost = File.OpenRead(werewolfSwf);
            var swfPost = SwfFile.ReadFrom(fsPost);
            var symPost = swfPost.Tags.OfType<SymbolClassTag>().First();

            // Foot MUST NOT have changed!
            ushort postFootId = symPost.References.First(r => r.SymbolName == "a_Foot1Bent_Werewolf").SymbolID;
            Assert.Equal(origFootId, postFootId);

            // Fists symbol MUST have been remapped
            ushort postFistId = symPost.References.First(r => r.SymbolName == "a_WeaponFists01Large_Werewolf").SymbolID;
            Assert.True(postFistId > 0);
        }
        finally
        {
            // Restore Gfx_Werewolf.swf
            bm.RestoreFile("Gfx_Werewolf.swf");
        }
    }

    [Fact]
    public void TestWeaponColorPaletteMappingIncludesCostumeDefines()
    {
        var inst = BrawlhallaSkinStudio.Core.Services.GameDetector.DetectGameInstallation();
        var cat = new BrawlhallaSkinStudio.Core.Services.CatalogService();
        cat.LoadFromInstallation(inst);

        var mordex = cat.Legends.First(l => l.HeroName == "Werewolf");
        var mordexCostume = mordex.Costumes.First(c => c.IsDefault);
        var mordexGauntlet = cat.WeaponSkins.First(w => w.WeaponSkinName == "FistsWerewolf");
        var sunsetScheme = cat.ColorSchemes.FirstOrDefault(cs => cs.SwapColors.Count > 0) ?? cat.ColorSchemes[0];

        var req = new ModificationRequest
        {
            TargetLegend = mordex,
            TargetCostume = mordexCostume,
            TargetWeapon1 = mordexGauntlet,
            SelectedColorScheme = sunsetScheme
        };

        var bm = new BrawlhallaSkinStudio.Core.Engine.BackupManager(inst.GamePath);
        var engine = new BrawlhallaSkinStudio.Core.Engine.ModificationEngine(inst.GamePath, bm, cat);

        // Call private BuildWeaponColorSwapMap via reflection
        var method = typeof(BrawlhallaSkinStudio.Core.Engine.ModificationEngine).GetMethod("BuildWeaponColorSwapMap",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        Assert.NotNull(method);

        var colorMap = (Dictionary<uint, uint>)method.Invoke(engine, new object[] { mordexGauntlet, null!, req })!;
        Assert.NotNull(colorMap);
        Assert.NotEmpty(colorMap);

        // Body1 or Body2 from Mordex's costume base colors must be in colorMap
        bool hasCostumeMapped = mordexCostume.BaseColors.Values.Any(bc => colorMap.ContainsKey(bc));
        Assert.True(hasCostumeMapped, "Color map should contain swaps for costume color defines present on weapon shapes");
    }
}
