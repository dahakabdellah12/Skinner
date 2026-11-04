# Changelog — Skinner

All notable changes to the Skinner (formerly BrawlCraft) project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [3.3.5] — 2026-11-04

### Added & Improved
- **Update Dialog Simplification:**
  - Removed close (x) button to streamline modal flow.
  - Removed redundant "View Changelog" button.
  - Removed header download icon for a clean, minimalist header.
  - Simplified dialog title to direct phrase "تحديث جديد" / "New Update".
  - Cleaned up progress bar metrics to eliminate confusing "0" number artifact.
- **Seamless Silent In-Place Update & Auto-Relaunch:**
  - Configured update execution to run Inno Setup silently (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /DIR=...`), completely eliminating Setup wizard dialogs and options during auto-updates.
  - Automatically relaunches the updated Skinner application immediately upon installation completion.
- **Gnash (Nash) Catalog Pruning:**
  - Removed all three ZombieWalker variant skins (`ZombieWalker1` ID 482, `ZombieWalker2` ID 483, `ZombieWalker3` ID 484) from Gnash (`caveman`), leaving exactly 12 authentic distinct costumes.

## [3.3.4] — 2026-11-04

### Added & Improved
- **Final Skins Catalog Cleanup & Duplicate Pruning:**
  - Removed 13 newly targeted skin records across 12 legends:
    - Ezio: Anakin Skywalker (`Anakin`, Costume ID 641)
    - Petra: Ken (`SFKen`, Costume ID 574)
    - Rayman: Dhalsim (`SFDhalsim`, Costume ID 573)
    - Jiro: Yoshimitsu (`Yoshimitsu`, Costume ID 637)
    - Kor: Four Arms (`FourArms`, Costume ID 456)
    - Koji: Sei Jo-Kai Koji / Shen Sekai Koji (`CyberSam`, Costume ID 376)
    - Asuri: Puss in Boots (`Puss`, Costume ID 793)
    - Red Raptor: Eren Yeager (`Eren`, Costume ID 735)
    - Teros: Roman Reigns (`RomanReigns`, Costume ID 421) and Gruagach / Goroagash (`Gruagach`, Costume ID 396)
    - Ada: Heatblast (`Heatblast`, Costume ID 454)
    - Lord Vraxx: Alien Warlord (`AlienWarlord`, Costume ID 309)
    - Nash (Gnash): Becky Lynch (`Becky`, Costume ID 409)
  - Pruned redundant internal stance duplicate records across 3 legends:
    - Priya (`BladeDancer`, ID 67): Pruned `BladeDancerStance2` (Costume ID 964), preserving canonical default (Costume ID 965).
    - Aurus (`ActualGladiator`, ID 71): Pruned `ActualGladiatorStance2` (Costume ID 785), preserving canonical default (Costume ID 783).
    - Lady Vera (`Cleric`, ID 69): Pruned `ClericStance2` (Costume ID 738), preserving canonical default (Costume ID 737).
  - Maintained complete integrity of Nash (`Caveman`, ID 7), preserving all 15 distinct skins including ZombieWalker1, 2, and 3.
  - Purged obsolete cached SVG files from disk (`BladeDancerStance2`, `ActualGladiatorStance2`, `ClericStance2`).
- **Updater Asset Packaging:**
  - Standardized dual installer naming (`Skinner-Setup-3.3.4.exe` and `Skinner-3.3.4-setup-x64.exe`) ensuring seamless automatic installation across all existing clients.
  - Full Ed25519 cryptographic signing of `SHA256SUMS.txt` manifest.

## [3.3.3] — 2026-11-04

### Added & Improved
- **Skins Catalog Cleanup & Precision Pruning:**
  - Removed 100 targeted skin records across 54 legends from Skinner's catalog based on verified internal identifiers (`CostumeID`) and character ownership (`OwnerHero`).
  - Pruned redundant stance duplicates for Ransom (`CyberVirus`, ID 68) and Seven (`Roboengineer`, ID 61) while preserving canonical default skins and visual variants.
  - Removed all Wolf Casey records on Brynn and both Lara Croft records on Diana.
  - Preserved distinct visual variants sharing similar names (e.g., ZombieWalker variants on Gnash/Nash).
  - Purged obsolete cached SVG assets from disk to prevent stale icons from appearing.
  - Cleaned up crossover weapon mappings in UI components.
- **Automated Verification:**
  - Added dedicated regression and data integrity test `test_skin_removal_and_duplicate_cleanup`.
  - Maintained 100% test pass rate across all engine, updater, and catalog suites.

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
