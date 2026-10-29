using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;
using BrawlhallaSkinStudio.Core.Models;

namespace BrawlhallaSkinStudio.Core.Engine;

public class BackupManager
{
    private const int MaxRetainedBackups = 25;

    private readonly string _gamePath;
    private readonly string _backupRoot;
    private readonly string _manifestPath;
    private readonly List<BackupRecord> _records = [];

    // Hash cache keyed by absolute path. Invalidated automatically when the file
    // timestamp or size changes, so repeated "is this modified?" scans stay cheap.
    private readonly Dictionary<string, (DateTime LastWriteUtc, long Length, string Hash)> _hashCache =
        new(StringComparer.OrdinalIgnoreCase);
    private readonly object _hashCacheLock = new();

    public BackupManager(string gamePath, string? customBackupDir = null)
    {
        _gamePath = gamePath;
        _backupRoot = customBackupDir ?? Path.Combine(gamePath, ".studio_backups");
        _manifestPath = Path.Combine(_backupRoot, "backup_manifest.json");

        if (!Directory.Exists(_backupRoot))
        {
            Directory.CreateDirectory(_backupRoot);
        }

        LoadManifest();
    }

    public IReadOnlyList<BackupRecord> Records => _records.AsReadOnly();

    public string CreateBackup(IEnumerable<string> filePaths, string reason = "Mod Application")
    {
        string backupId = CreateUniqueBackupId();
        string backupFolder = Path.Combine(_backupRoot, backupId);
        Directory.CreateDirectory(backupFolder);

        var record = new BackupRecord
        {
            BackupId = backupId,
            Timestamp = DateTime.UtcNow,
            Reason = reason
        };

        foreach (var fullPath in filePaths)
        {
            if (!File.Exists(fullPath)) continue;

            string relPath = Path.GetRelativePath(_gamePath, fullPath);
            string destPath = Path.Combine(backupFolder, relPath);
            string? dir = Path.GetDirectoryName(destPath);
            if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
            {
                Directory.CreateDirectory(dir);
            }

            CopyFileSafely(fullPath, destPath);
            string hash = ComputeSha256(fullPath);

            record.FileOriginalHashes[relPath] = hash;
            record.BackupFilesRelativePath[relPath] = Path.GetRelativePath(_backupRoot, destPath);
        }

        _records.Insert(0, record);
        PruneBackups();
        SaveManifest();
        return backupId;
    }

    public bool RestoreFile(string relativePath)
    {
        relativePath = Path.GetRelativePath(_gamePath, Path.Combine(_gamePath, relativePath));

        // Find oldest (pristine original) backup of this relative path
        for (int i = _records.Count - 1; i >= 0; i--)
        {
            var record = _records[i];
            var match = record.BackupFilesRelativePath.FirstOrDefault(kvp => kvp.Key.Equals(relativePath, StringComparison.OrdinalIgnoreCase));
            if (!string.IsNullOrEmpty(match.Value))
            {
                string backupFullPath = Path.Combine(_backupRoot, match.Value);
                string gameFullPath = Path.Combine(_gamePath, match.Key);

                if (File.Exists(backupFullPath))
                {
                    CopyFileSafely(backupFullPath, gameFullPath);
                    InvalidateHash(gameFullPath);
                    return true;
                }
            }
        }
        return false;
    }

    public int RestoreAll()
    {
        int restoredCount = 0;
        var restoredRelPaths = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        // Iterate backwards from oldest to preserve true pristine state
        for (int i = _records.Count - 1; i >= 0; i--)
        {
            var record = _records[i];
            foreach (var kvp in record.BackupFilesRelativePath)
            {
                if (restoredRelPaths.Add(kvp.Key))
                {
                    string backupFullPath = Path.Combine(_backupRoot, kvp.Value);
                    string gameFullPath = Path.Combine(_gamePath, kvp.Key);
                    if (File.Exists(backupFullPath))
                    {
                        CopyFileSafely(backupFullPath, gameFullPath);
                        InvalidateHash(gameFullPath);
                        restoredCount++;
                    }
                }
            }
        }

        return restoredCount;
    }

    public bool IsFileModified(string relativePath)
    {
        relativePath = Path.GetRelativePath(_gamePath, Path.Combine(_gamePath, relativePath));
        string fullPath = Path.Combine(_gamePath, relativePath);
        if (!File.Exists(fullPath)) return false;

        string currentHash = ComputeSha256Cached(fullPath);
        // Compare with the earliest backup original hash
        for (int i = _records.Count - 1; i >= 0; i--)
        {
            var match = _records[i].FileOriginalHashes.FirstOrDefault(kvp => kvp.Key.Equals(relativePath, StringComparison.OrdinalIgnoreCase));
            if (!string.IsNullOrEmpty(match.Key))
            {
                return !string.Equals(currentHash, match.Value, StringComparison.OrdinalIgnoreCase);
            }
        }
        return false;
    }

    /// <summary>
    /// Counts distinct files that currently differ from their pristine backup copy.
    /// O(unique files), not O(records × files), because duplicates across backups are de-duplicated.
    /// </summary>
    public int CountModifiedFiles()
    {
        var pristineHashes = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        for (int i = _records.Count - 1; i >= 0; i--)
        {
            foreach (var kvp in _records[i].FileOriginalHashes)
            {
                pristineHashes[kvp.Key] = kvp.Value;
            }
        }

        int modified = 0;
        foreach (var kvp in pristineHashes)
        {
            string fullPath = Path.Combine(_gamePath, kvp.Key);
            if (!File.Exists(fullPath)) continue;

            string currentHash = ComputeSha256Cached(fullPath);
            if (!string.Equals(currentHash, kvp.Value, StringComparison.OrdinalIgnoreCase))
            {
                modified++;
            }
        }
        return modified;
    }

    /// <summary>
    /// Keeps the newest <see cref="MaxRetainedBackups"/> snapshots plus the oldest
    /// (pristine) one, deleting stale folders so the backup tree cannot grow forever.
    /// </summary>
    private void PruneBackups()
    {
        if (_records.Count <= MaxRetainedBackups + 1) return;

        var keep = new HashSet<BackupRecord>(_records.Take(MaxRetainedBackups));
        keep.Add(_records[^1]);

        var removed = _records.Where(r => !keep.Contains(r)).ToList();
        if (removed.Count == 0) return;

        _records.RemoveAll(r => !keep.Contains(r));

        foreach (var record in removed)
        {
            try
            {
                string folder = Path.Combine(_backupRoot, record.BackupId);
                if (Directory.Exists(folder))
                {
                    Directory.Delete(folder, recursive: true);
                }
            }
            catch
            {
                // A locked/read-only backup folder is not fatal; keep the manifest entry gone.
            }
        }
    }

    public static void CopyFileSafely(string sourcePath, string destPath)
    {
        try
        {
            File.Copy(sourcePath, destPath, overwrite: true);
        }
        catch (IOException)
        {
            // Fallback for live game access when file cannot be copied directly
            using var sourceStream = new FileStream(sourcePath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
            using var destStream = new FileStream(destPath, FileMode.Create, FileAccess.Write, FileShare.ReadWrite);
            sourceStream.CopyTo(destStream);
            destStream.Flush(true);
        }
    }

    public static string ComputeSha256(string filePath)
    {
        using var sha256 = SHA256.Create();
        using var stream = new FileStream(filePath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
        byte[] hash = sha256.ComputeHash(stream);
        return BitConverter.ToString(hash).Replace("-", "").ToLowerInvariant();
    }

    private string ComputeSha256Cached(string fullPath)
    {
        var info = new FileInfo(fullPath);
        DateTime lastWrite = info.LastWriteTimeUtc;
        long length = info.Length;

        lock (_hashCacheLock)
        {
            if (_hashCache.TryGetValue(fullPath, out var cached)
                && cached.LastWriteUtc == lastWrite
                && cached.Length == length)
            {
                return cached.Hash;
            }

            string hash = ComputeSha256(fullPath);
            _hashCache[fullPath] = (lastWrite, length, hash);
            return hash;
        }
    }

    private void InvalidateHash(string fullPath)
    {
        lock (_hashCacheLock)
        {
            _hashCache.Remove(fullPath);
        }
    }

    private string CreateUniqueBackupId()
    {
        string baseId = DateTime.UtcNow.ToString("yyyyMMdd_HHmmss_fff");
        string candidate = baseId;
        int suffix = 1;
        while (Directory.Exists(Path.Combine(_backupRoot, candidate))
               || _records.Any(r => string.Equals(r.BackupId, candidate, StringComparison.OrdinalIgnoreCase)))
        {
            candidate = $"{baseId}_{suffix++}";
        }
        return candidate;
    }

    private void LoadManifest()
    {
        if (File.Exists(_manifestPath))
        {
            try
            {
                string json = File.ReadAllText(_manifestPath);
                var loaded = JsonSerializer.Deserialize<List<BackupRecord>>(json);
                if (loaded != null)
                {
                    _records.Clear();
                    _records.AddRange(loaded);
                }
            }
            catch { }
        }
    }

    private void SaveManifest()
    {
        try
        {
            string json = JsonSerializer.Serialize(_records, new JsonSerializerOptions { WriteIndented = true });
            File.WriteAllText(_manifestPath, json);
        }
        catch { }
    }
}
