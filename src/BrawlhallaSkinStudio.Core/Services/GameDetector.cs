using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using BrawlhallaSkinStudio.Core.Models;
using Microsoft.Win32;

namespace BrawlhallaSkinStudio.Core.Services;

public static class GameDetector
{
    public static GameInstallation DetectGameInstallation(string? customPath = null)
    {
        var installation = new GameInstallation();

        var candidates = new List<string>();
        if (!string.IsNullOrWhiteSpace(customPath))
        {
            string clean = customPath.Trim().Trim('"', '\'');
            if (File.Exists(clean) && Path.GetFileName(clean).Equals("Brawlhalla.exe", StringComparison.OrdinalIgnoreCase))
            {
                clean = Path.GetDirectoryName(clean) ?? clean;
            }
            candidates.Add(clean);
        }

        // Try detecting from saved user config
        try
        {
            string configPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BrawlhallaSkinStudio", "game_path.txt");
            if (File.Exists(configPath))
            {
                string saved = File.ReadAllText(configPath).Trim();
                if (!string.IsNullOrEmpty(saved))
                {
                    candidates.Add(saved);
                }
            }
        }
        catch { }

        // Check relative parent directory (e.g. workspace inside game dir)
        string currentDir = AppDomain.CurrentDomain.BaseDirectory;
        candidates.Add(Path.GetFullPath(Path.Combine(currentDir, "..")));
        candidates.Add(Path.GetFullPath(Path.Combine(currentDir, "..", "..")));
        candidates.Add(Path.GetFullPath(Path.Combine(currentDir, "..", "..", "..")));

        // Common Steam install locations derived from the current machine (no hard-coded drives).
        foreach (var steamRoot in GetDefaultSteamRoots())
        {
            candidates.Add(Path.Combine(steamRoot, "steamapps", "common", "Brawlhalla"));
        }

        // Try detecting from Windows Uninstall Registry (Steam App 291550)
        if (OperatingSystem.IsWindows())
        {
            try
            {
                using var uKey = Registry.LocalMachine.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 291550");
                string? loc = uKey?.GetValue("InstallLocation") as string;
                if (!string.IsNullOrEmpty(loc))
                {
                    candidates.Add(loc);
                }
            }
            catch { }

            try
            {
                using var uKey32 = Registry.LocalMachine.OpenSubKey(@"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 291550");
                string? loc32 = uKey32?.GetValue("InstallLocation") as string;
                if (!string.IsNullOrEmpty(loc32))
                {
                    candidates.Add(loc32);
                }
            }
            catch { }
        }

        // Try detecting from Steam registry & libraryfolders.vdf
        try
        {
            var steamLibraries = GetSteamLibraryFolders();
            foreach (var lib in steamLibraries)
            {
                candidates.Add(Path.Combine(lib, "steamapps", "common", "Brawlhalla"));
            }
        }
        catch { }

        // Find the first valid candidate
        string? resolvedPath = null;
        foreach (var candidate in candidates.Distinct())
        {
            if (Directory.Exists(candidate) && File.Exists(Path.Combine(candidate, "Brawlhalla.exe")))
            {
                resolvedPath = Path.GetFullPath(candidate);
                break;
            }
        }

        if (resolvedPath == null)
        {
            installation.IsValid = false;
            installation.ValidationMessages.Add("Could not locate a valid Brawlhalla installation directory.");
            return installation;
        }

        installation.GamePath = resolvedPath;
        installation.ExePath = Path.Combine(resolvedPath, "Brawlhalla.exe");
        installation.GameSwzPath = Path.Combine(resolvedPath, "Game.swz");
        installation.BrawlhallaAirSwfPath = Path.Combine(resolvedPath, "BrawlhallaAir.swf");

        // Validate critical files
        if (!File.Exists(installation.ExePath))
            installation.ValidationMessages.Add("Missing Brawlhalla.exe");

        if (!File.Exists(installation.GameSwzPath))
            installation.ValidationMessages.Add("Missing Game.swz");

        if (!File.Exists(installation.BrawlhallaAirSwfPath))
            installation.ValidationMessages.Add("Missing BrawlhallaAir.swf");

        // Discovered SWFs
        try
        {
            var swfFiles = Directory.GetFiles(resolvedPath, "*.swf", SearchOption.TopDirectoryOnly);
            installation.DiscoveredSwfCount = swfFiles.Length;
        }
        catch (Exception ex)
        {
            installation.ValidationMessages.Add($"Error scanning SWFs: {ex.Message}");
        }

        // Detect Build ID from Steam appmanifest
        try
        {
            string appManifestPath = Path.Combine(resolvedPath, "..", "..", "appmanifest_291550.acf");
            if (File.Exists(appManifestPath))
            {
                string text = File.ReadAllText(appManifestPath);
                var match = Regex.Match(text, @"""buildid""\s+""(\d+)""");
                if (match.Success)
                {
                    installation.BuildId = match.Groups[1].Value;
                }
            }
        }
        catch { }

        // Detect Version from BrawlhallaAir.swf
        try
        {
            if (File.Exists(installation.BrawlhallaAirSwfPath))
            {
                using var fs = new FileStream(installation.BrawlhallaAirSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
                byte[] buffer = new byte[Math.Min(1024 * 1024, fs.Length)];
                fs.Read(buffer, 0, buffer.Length);
                string content = System.Text.Encoding.Latin1.GetString(buffer);
                var match = Regex.Match(content, @"\b(1[0-9]\.\d{2}\.\d+)\b");
                if (match.Success)
                {
                    installation.Version = match.Groups[1].Value;
                }
                else
                {
                    installation.Version = "11.00+";
                }
            }
        }
        catch
        {
            installation.Version = "Unknown";
        }

        installation.IsValid = installation.ValidationMessages.Count == 0;
        return installation;
    }

    private static IEnumerable<string> GetDefaultSteamRoots()
    {
        string? programFilesX86 = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86);
        string? programFiles = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles);

        if (!string.IsNullOrEmpty(programFilesX86))
            yield return Path.Combine(programFilesX86, "Steam");
        if (!string.IsNullOrEmpty(programFiles))
            yield return Path.Combine(programFiles, "Steam");

        // Secondary libraries are commonly installed as <Drive>:\SteamLibrary.
        foreach (var drive in DriveInfo.GetDrives())
        {
            string? root = null;
            try
            {
                if (drive.IsReady && (drive.DriveType == DriveType.Fixed || drive.DriveType == DriveType.Removable))
                    root = drive.RootDirectory.FullName;
            }
            catch { }

            if (!string.IsNullOrEmpty(root))
                yield return Path.Combine(root, "SteamLibrary");
        }
    }

    private static List<string> GetSteamLibraryFolders()
    {
        var result = new List<string>();
        if (!OperatingSystem.IsWindows())
            return result;

        string? steamPath = null;

        using (var key = Registry.CurrentUser.OpenSubKey(@"Software\Valve\Steam"))
        {
            steamPath = key?.GetValue("SteamPath") as string;
        }

        if (string.IsNullOrEmpty(steamPath))
        {
            using var key = Registry.LocalMachine.OpenSubKey(@"Software\Valve\Steam");
            steamPath = key?.GetValue("InstallPath") as string;
        }

        if (string.IsNullOrEmpty(steamPath) || !Directory.Exists(steamPath))
            return result;

        result.Add(steamPath);

        string vdfPath = Path.Combine(steamPath, "steamapps", "libraryfolders.vdf");
        if (File.Exists(vdfPath))
        {
            string vdfContent = File.ReadAllText(vdfPath);
            var matches = Regex.Matches(vdfContent, @"""path""\s+""([^""]+)""");
            foreach (Match m in matches)
            {
                string p = m.Groups[1].Value.Replace(@"\\", @"\");
                if (Directory.Exists(p))
                    result.Add(p);
            }
        }

        return result.Distinct(StringComparer.OrdinalIgnoreCase).ToList();
    }
}
