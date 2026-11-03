# Changelog — Skinner

All notable changes to the Skinner (formerly BrawlCraft) project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [3.3.2] — 2026-11-03

### Added
- **Cryptographically Authenticated Auto-Updater:**
  - Automated update discovery using the official GitHub Releases API (`dahakabdellah12/Skinner`).
  - Ed25519 digital signature verification using an embedded official 32-byte public key.
  - Mandatory SHA-256 integrity verification against signed `SHA256SUMS.txt` manifests.
  - Safe Windows update installation helper (`apply_update.cmd`) resolving `ERROR_SHARING_VIOLATION` file locks during in-place upgrades.
  - Asynchronous startup check with interactive user confirmation modal and "View Changelog" option.
  - Session-aware dismissal remembering "Later" choices without badgering users.
- **Automated Update Testing Suite (`tests/updater_tests.rs`):**
  - Semantic version comparison tests (`3.3.10` > `3.3.9`, `3.3.2` > `3.3.1`, stable > prerelease).
  - Ed25519 signature validation and tampering detection tests.
  - Checksum manifest parsing and file hash validation tests.
  - URL whitelist boundaries and path traversal rejection tests.

### Security Hardening
- **Multi-Tenant CDN Isolation:**
  - Removed permissive `objects.githubusercontent.com` prefix matching. Update downloads are strictly constrained to `https://github.com/dahakabdellah12/Skinner/releases/download/` (with backward compatibility for `BrawlCraft`).
- **Closed Verification Logic:**
  - Update installer execution now fails closed if release signature or checksum is absent or invalid.
- **WebView Navigation Sealing:**
  - Registered `.with_navigation_handler` on Wry's `WebViewBuilder` to strictly restrict navigation to `brawlcraft://` protocols, eliminating IPC exposure to third-party web origins.
- **Decompression Zip-Bomb Protection:**
  - Implemented `.take(64 * 1024 * 1024)` hard limits on zlib decompressors in `src/anm_engine.rs`.
- **Bounded Archive Headers:**
  - Header capacity caps on `.swz` and `.swf` binary parsers preventing allocation crashes from corrupt game assets.

### Performance Optimizations
- **Zero Idle Disk Thrashing:**
  - `BackupManager` stat caching verifies file length and mtime via OS metadata, dropping idle background disk reads from ~300 MB every 3.5 seconds to 0 bytes.
- **Sub-millisecond Process Detection:**
  - Native Win32 `CreateToolhelp32Snapshot` process checks execute in 0.14 ms with zero child process spawning overhead.
- **Concurrent IPC Fast-Path:**
  - Preview and SVG asset queries bypass the global application lock via `SVG_MEM_CACHE`, responding in 0.04 ms.
- **Stripped Release Footprint:**
  - Pruned dead Slint dependencies and enabled ThinLTO + stripping, producing a compact 4.6 MB Windows executable.

---

## [3.3.1] — 2026-11-03

### Changed
- Rebranded desktop studio application to Skinner.
- Refined loading screen UI with minimalist logo styling.
- Curated roster expansion supporting all 70 playable official Brawlhalla Legends, 794 costumes, and 2,137 weapon skins.
