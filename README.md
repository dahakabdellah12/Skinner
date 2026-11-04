<div align="center">

<img src="assets/logo.png" alt="Skinner Logo" width="180" />

# Skinner

**High-Performance Native Windows Desktop Studio for Brawlhalla Cosmetic Customization**

[![Version](https://img.shields.io/badge/version-v3.3.4-blue.svg?style=flat-square)](https://github.com/dahakabdellah12/Skinner/releases/tag/v3.3.4)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-0078D6.svg?style=flat-square&logo=windows)](https://github.com/dahakabdellah12/Skinner)
[![Rust](https://img.shields.io/badge/Rust-2021%20edition-DEA584.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/Frontend-React%2019%20%2B%20Vite-61DAFB.svg?style=flat-square&logo=react)](https://react.dev/)
[![Security](https://img.shields.io/badge/Security-Ed25519%20Dalek%20Signed-brightgreen.svg?style=flat-square)](https://github.com/dahakabdellah12/Skinner/releases)
[![License](https://img.shields.io/badge/license-Proprietary-gray.svg?style=flat-square)](LICENSE)

</div>

---

## Overview

**Skinner** is a lightweight, ultra-fast native Windows desktop studio engineered specifically for exploring, previewing, and applying cosmetic modifications to Brawlhalla.

Combining a memory-safe **Rust** host process, **Microsoft Edge WebView2**, and a modern **React 19** frontend with a high-throughput **.NET 8** vector rendering engine, Skinner gives players complete creative control over character costumes, weapon combinations, and palette schemes with bit-exact atomic rollbacks.

---

## Key Features

### 🚀 Performance & Real-Time Engine
- **Instant Catalog Loading:** Full `Game.swz` decryption and parsing of all **70 playable Legends, 794 costumes, 2,137 weapon skins, and 81 color schemes in under 250 milliseconds**.
- **Zero Idle Disk Thrashing:** Intelligent file metadata stat caching eliminates continuous disk reading during background operation.
- **Sub-Millisecond Process Detection:** Native Win32 `Toolhelp32` process snapshotting detects active game sessions in 0.14 ms with zero child process overhead.
- **Hardware-Accelerated UI:** 60+ FPS fluid navigation, instant search, and real-time weapon filtering powered by React 19 and Vite.

### 🎨 Cosmetic Customization Workbench
- **Independent Weapon Loadouts:** Equip primary and secondary weapon skins from any costume across all 14 weapon types (Sword, Hammer, Blasters, Spear, Rocket Lance, Katars, Bow, Axe, Gauntlets, Scythe, Cannon, Orb, Greatsword, Battle Boots).
- **Intra- & Cross-SWF Asset Porting:** Inject shapes, sprites, and animation linkages across character packages seamlessly.
- **Palette Baking & Dynamic Recoloring:** Real-time color palette preview chips with instant live indicators.
- **Loadout Presets:** Save, duplicate, export, and load custom legend loadouts as portable JSON files.

### 🛡️ Bit-Exact Safety & Atomic Rollbacks
- **Cryptographic Fingerprinting:** Computes SHA-256 hashes prior to applying any modification.
- **Automatic Snapshots:** Preserves pristine original game files in `.studio_backups/`.
- **Single-Click Rollback:** Instant "Rollback All" restores game assets to byte-identical official conditions without re-verifying Steam cache.

### 🔒 Cryptographically Authenticated Auto-Updater
- **Automatic Discovery:** Queries official GitHub Releases asynchronously without slowing down application startup.
- **Digital Signatures:** Releases verified using **Ed25519 digital signatures** against an embedded official public key.
- **Mandatory Hash Verification:** Enforces strict SHA-256 checksum matching against signed release manifests.
- **Safe Windows File-Lock Handoff:** Dedicated update helper resolves `ERROR_SHARING_VIOLATION` locks during in-place portable updates while preserving all user configurations, presets, and backups.

---

## Architecture & Technology Stack

```
┌─────────────────────────────────────────────────────────────┐
│                      FRONTEND LAYER                         │
│   React 19  •  Vite 8  •  Lucide Icons  •  CSS3 Tokens      │
│   Roster (LegendsPage) → Customizer Workbench → Presets     │
└──────────────────────────────▲──────────────────────────────┘
                               │  JSON-RPC IPC Bridge
                               │  Virtual Host: brawlcraft://localhost
┌──────────────────────────────▼──────────────────────────────┐
│                    DESKTOP HOST & RUNTIME                   │
│   Rust 2021  •  Wry 0.57  •  Tao 0.37  •  Win32 API         │
│   • GameDetector: Steam Registry & libraryfolders.vdf parser│
│   • SWZ Decryptor: Well512 PRNG + Deflate Decompressor      │
│   • BackupManager: SHA-256 fingerprinting & Stat Caching    │
│   • UpdateManager: Ed25519 Dalek & HTTPS Redirect Validator │
└──────────────────────────────▲──────────────────────────────┘
                               │  Named Pipe IPC
┌──────────────────────────────▼──────────────────────────────┐
│                   VECTOR PREVIEW ENGINE                     │
│   .NET 8 Core  •  SwfLib  •  BrawlhallaSkinStudio.Core      │
│   • Vector Tag Rebinding & Shape Extraction                 │
│   • High-Performance SVG Rasterization & Asset Caching      │
└─────────────────────────────────────────────────────────────┘
```

---

## Installation & Download

Skinner is distributed as a self-contained portable Windows x64 package. No installation setup or administrative privileges are required.

### 1. Download Official Release
Download the latest verified release archive from the official GitHub repository:

👉 **[Download Skinner v3.3.4 (Windows x64)](https://github.com/dahakabdellah12/Skinner/releases/latest)**

### 2. Verify Cryptographic Integrity
To guarantee authenticity and protect against tampering, verify the archive's SHA-256 checksum against the official manifest:

```powershell
# Compute local hash
Get-FileHash -Algorithm SHA256 Skinner-3.3.4-windows-x64.zip
```

**Expected SHA-256 Digest (v3.3.4):**
```text
cd4572618363748c0bf8436f2047995107a65e040396c768a8a0e99801a961a1
```

**Official Embedded Ed25519 Public Verification Key:**
```text
105b8cf237a941fd069f8281a7b3e4e5361b3eddeff10e8626da224054e90f5a
```

### 3. Launch
Extract `Skinner-3.3.4-windows-x64.zip` to your preferred folder and launch:
```text
Skinner.exe
```

---

## Building from Source

### Prerequisites
- **Windows 10 / 11 (64-bit)**
- **Rust Toolchain:** 1.80+ (`stable-x86_64-pc-windows-msvc` or `gnu`)
- **Node.js:** v20+ with npm
- **.NET SDK:** 8.0+
- **Microsoft Edge WebView2 Runtime** (installed by default on Windows 10/11)

### Build Steps

1. **Clone the Repository:**
   ```bash
   git clone https://github.com/dahakabdellah12/Skinner.git
   cd Skinner
   ```

2. **Build the Frontend:**
   ```bash
   npm install
   npm run build
   ```

3. **Compile the Native Executable:**
   ```bash
   cargo build --release
   ```
   The stripped, optimized executable will be located at:
   ```text
   target/release/Skinner.exe
   ```

4. **Run the Test Suite:**
   ```bash
   cargo test
   ```
   *Executes all 78 automated integration and cryptographic security tests.*

---

## Automated Test Coverage

The project maintains comprehensive regression and security test suites:

- **Core Engine & Catalog Tests (`tests/game_tests.rs`):** 66 tests covering SWZ decryption, SWF tag modification, character selector animation bindings, weapon skin independence, and atomic rollback safety.
- **Cryptographic Updater Tests (`tests/updater_tests.rs`):** 12 tests covering Ed25519 signature verification, tampering detection, SHA-256 checksums, URL whitelist boundaries, downgrade rejection, and user data preservation.

---

## Security Policy

- **Fail-Closed Verification:** Updates without valid Ed25519 signatures and exact bit-for-bit SHA-256 matching are automatically rejected and deleted.
- **Sealed WebView Navigation:** All navigation events inside the desktop UI are strictly confined to internal virtual host protocols (`brawlcraft://`), preventing remote code execution or IPC manipulation from external URLs.
- **Decompression Bombs:** Hard buffer caps (64 MB) protect all archive extraction routines against unbounded memory consumption denial-of-service vectors.

---

## Disclaimer

**Skinner** is an independent, unofficial fan project and is not affiliated with, endorsed by, or associated with Ubisoft or Blue Mammoth Games. Brawlhalla and all associated assets are trademarks or registered trademarks of Blue Mammoth Games / Ubisoft. Modifications are applied locally to your own game installation; always follow Brawlhalla's Terms of Service and maintain backups.

---

## License & Support

- **Repository:** [https://github.com/dahakabdellah12/Skinner](https://github.com/dahakabdellah12/Skinner)
- **Issues & Bug Reports:** [https://github.com/dahakabdellah12/Skinner/issues](https://github.com/dahakabdellah12/Skinner/issues)
- **Author:** [dahakabdellah12](https://github.com/dahakabdellah12)
