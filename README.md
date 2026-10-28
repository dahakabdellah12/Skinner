# Brawlhalla Skin Studio

**Brawlhalla Skin Studio** is a complete, native Windows desktop application and local game cosmetic mod manager for Brawlhalla. Built with a modern hybrid architecture—a **React 18 + Vite frontend** hosted inside **Microsoft Edge WebView2** paired with a high-performance **C# / .NET 8 backend engine**—it reads actual game data, allows independent costume and weapon cosmetic customization, creates atomic backups, applies verifiable vector modifications to game asset packages, and provides instant pristine restoration.

---

## Key Features

- **Automatic Game Detection**: Seamlessly locates Steam libraries via Windows registry and `libraryfolders.vdf`, discovers `Brawlhalla.exe`, `Game.swz`, and all 560+ loose `.swf` packages.
- **Authentic Asset Catalog**: Decrypts and parses `Game.swz` in place using the Well512 PRNG engine (`HeroTypes.xml`, `costumeTypes.csv`, `weaponSkinTypes.csv`, `ColorSchemeTypes.xml`) to display all 72 playable legends, 795+ costumes, 2,130+ weapon skins, and 82 color schemes.
- **Modern Responsive Web UI**: Fast, responsive desktop user interface built with React 18, Vite, and modern CSS3 dark-theme design tokens, embedded natively via Microsoft Edge WebView2.
- **Independent Weapon Cosmetics**: Replace primary and secondary weapon skins independently from costume selections across all 14 weapon classes (Sword, Hammer, Blasters, Spear, Rocket Lance, Katars, Bow, Axe, Gauntlets, Scythe, Cannon, Orb, Greatsword, Battle Boots).
- **Intra-SWF & Cross-SWF Porting**: Supports instant intra-SWF symbol remapping (e.g., Bödvar Classic -> Beardvar) and cross-package asset injection (transferring shapes, sprites, and linkages across character files).
- **Palette Baking & Recoloring**: Replaces baseline define colors (`Body1_Define`, `Hair_Define`, `Cloth_Define`) with official color scheme swaps or custom hex values.
- **Atomic Backup & Rollback Engine**: Never overwrites files without first computing SHA-256 hashes and creating timestamped backups in `.studio_backups/`. Single-click restoration restores 100% bit-exact pristine state.
- **Preset Management**: Save, edit, duplicate, export, and import complete cosmetic loadouts as portable JSON presets.
- **In-Game Verification Workflow**: Dedicated diagnostics panel and interactive in-game verification checklist.

---

## Architecture & Technology Stack

```
┌─────────────────────────────────────────────────────────────┐
│                      FRONTEND LAYER                         │
│   React 18  •  Vite  •  Lucide Icons  •  CSS3 Design Tokens │
│  9 Pages: Dashboard, Legends, Costumes, Weapons, Studio,    │
│           Presets, Mod Manager, Diagnostics, Verification   │
└──────────────────────────────▲──────────────────────────────┘
                               │  JSON-RPC PostWebMessage
                               │  (Action, RequestId, Payload)
┌──────────────────────────────▼──────────────────────────────┐
│                    WEBVIEW2 HOST & BRIDGE                   │
│   Microsoft Edge WebView2 Runtime  •  WebViewBridge.cs      │
│   Virtual Host Mapping: https://appassets.local/            │
│   Hot-Reload Dev Mode:  http://localhost:5173/              │
└──────────────────────────────▲──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                      BACKEND ENGINE                         │
│   C#  •  .NET 8  •  WPF Native Shell                        │
│   • GameDetector & CatalogService (Well512 Decryption)      │
│   • ModificationEngine & SwfLib (Vector Tag Rebinding)      │
│   • BackupManager (Atomic SHA-256 Pristine Rollbacks)       │
│   • PresetManager & DiagnosticsService                      │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Details |
|---|---|---|
| **Frontend UI** | React 18 + Vite | HTML5, CSS3, Lucide Icons, Modern Dark UI (`#171717` / `#242424` / `#3B82F6`) |
| **Native Host** | WPF + Edge WebView2 | Microsoft.Web.WebView2 (`1.0.4314.35`), Virtual Host `appassets.local` |
| **Communication** | WebViewBridge (JSON-RPC) | Structured async messages `{ id, action, payload }` with correlation IDs |
| **SWZ Engine** | Well512 PRNG (Core) | In-place decryption with dynamic seed keys (`154281100u`) |
| **SWF Parser** | SwfLib | Binary shape reading, character re-indexing, symbol class linkage mapping |
| **Integrity & Backup** | BackupManager (Core) | SHA-256 fingerprinting before any write; bit-exact restore points |

---

## Project Structure

```
d:\SteamLibrary\steamapps\common\Brawlhalla\Brawlhalla Skin\
├── publish\                             <-- Standalone packaged Windows release
│   ├── BrawlhallaSkinStudio.App.exe     <-- Native executable
│   └── frontend\dist\                   <-- Packaged web frontend assets
├── Launch_Skin_Studio.bat               <-- Convenient double-click launcher
├── frontend\                            <-- Modern Web UI project
│   ├── package.json
│   ├── vite.config.js                   <-- Base relative config for local WebView2
│   └── src\
│       ├── main.jsx                     <-- React entry point
│       ├── App.jsx                      <-- Global app state and navigation
│       ├── styles\                      <-- variables.css, global.css, components.css
│       ├── services\
│       │   └── api.js                   <-- Asynchronous WebView2 IPC bridge client
│       ├── components\                  <-- Sidebar, Header, Toast, Modal
│       └── pages\
│           ├── HomePage.jsx             <-- Dashboard and quick start
│           ├── LegendsPage.jsx          <-- 72 playable legends
│           ├── CostumesPage.jsx         <-- 795+ costumes catalog
│           ├── WeaponSkinsPage.jsx      <-- 2,130+ weapon cosmetics
│           ├── StudioPage.jsx           <-- 3-pane customization workbench
│           ├── PresetsPage.jsx          <-- Preset manager (save/duplicate/export/import)
│           ├── ModManagerPage.jsx       <-- SHA-256 tracking & pristine rollback
│           ├── DiagnosticsPage.jsx      <-- System health and SWF scanner
│           └── VerificationPage.jsx     <-- In-game Training Mode checklist
├── src\
│   ├── BrawlhallaSkinStudio.Core\       <-- Core engine library (.NET 8)
│   └── BrawlhallaSkinStudio.App\        <-- Native desktop host (.NET 8 Windows)
│       ├── MainWindow.xaml              <-- WebView2 container
│       ├── MainWindow.xaml.cs           <-- Environment setup & dev/prod URL router
│       └── Bridge\
│           └── WebViewBridge.cs         <-- 17 JSON-RPC backend actions
├── tests\
│   └── BrawlhallaSkinStudio.Tests\      <-- Automated xUnit test suite (9 tests)
└── BrawlhallaSkinStudio.sln
```

---

## Development & Build Workflows

### 1. Running in Production Mode (Pre-packaged UI)
The standalone release runs out of the box without Node.js or any external server:

Double-click `Launch_Skin_Studio.bat` or run:
```powershell
.\publish\BrawlhallaSkinStudio.App.exe
```

### 2. Frontend Development with Hot Reload (HMR)
When modifying UI components, CSS styles, or page layouts with instant hot reloading:

```powershell
# 1. Start the Vite dev server
cd frontend
npm run dev

# 2. In a separate terminal, launch the desktop app
# MainWindow automatically detects http://localhost:5173 and connects!
C:\dotnet_sdk\dotnet.exe run --project src\BrawlhallaSkinStudio.App\BrawlhallaSkinStudio.App.csproj
```

### 3. Rebuilding the Production Application
```powershell
# 1. Build the frontend web bundle
npm --prefix frontend run build

# 2. Publish the standalone desktop application
C:\dotnet_sdk\dotnet.exe publish src\BrawlhallaSkinStudio.App\BrawlhallaSkinStudio.App.csproj -c Release -r win-x64 --self-contained false -o publish
```

---

## Automated Test Verification

All 9 unit and integration tests validate the backend engine against test fixtures and real installed game data:

```
Test run for BrawlhallaSkinStudio.Tests.dll (.NETCoreApp,Version=v8.0)
Total tests: 9. Passed: 9. Failed: 0. Skipped: 0.

✓ TestGameDetection: Steam registry & appmanifest build ID (25942757) resolution.
✓ TestCatalogLoading: Game.swz decryption (72 legends, 795 costumes, 2137 weapons).
✓ TestBackupAndRestoreCycle: SHA-256 tracking and pristine file restoration.
✓ TestIntraSwfCostumeModification: Bodvar Classic -> Beardvar symbol rebinding.
✓ TestCrossSwfWeaponReplacement: Cassidy Hammer ported into Gfx_Viking.swf.
✓ TestCrossLegendCostumeConversion: Azoth appearance ported into Onyx (Gargoyle).
✓ TestPreviewRendererGeneratesValidSvg: Shape vector extraction into crisp SVG.
✓ TestIncompatibleWeaponTypeRejection: Enforces base weapon type parity (e.g. Hammer != Bow).
✓ TestPresetManagerOperations: Create, duplicate, export, import, and delete presets.
```

### Bit-Exact SHA-256 Restoration Proof
```
Pre-mod SHA-256:   0cdc8eecb93655c3b349732614006cccc07eea50de140991db4a3b58a65bcbdf
Post-mod SHA-256:  ebab994983deace55fdcf79948570b99f1fa7e9d3fe7828e5b6c1f16ba071383
Post-restore SHA:  0cdc8eecb93655c3b349732614006cccc07eea50de140991db4a3b58a65bcbdf (Bit-exact match)
```
