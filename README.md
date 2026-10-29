# BrawlCraft

**BrawlCraft** is a native Windows desktop studio for creating and applying Brawlhalla cosmetic
modifications. It combines a **React 19 + Vite** user interface hosted inside **Microsoft Edge
WebView2** with a **C# / .NET 8** engine that reads real game data, performs verified vector edits
inside SWF packages, and keeps bit-exact backups so every change can be undone.

Current release: **v3.1.0** · License: **MIT** · Interface languages: **English / العربية** (LTR only)

---

## Features

- **Automatic Game Detection** — finds Brawlhalla through the Steam registry, `libraryfolders.vdf`,
  and common Steam library locations on every fixed/removable drive, then validates
  `Brawlhalla.exe`, `Game.swz`, and the loose `.swf` packages.
- **Authentic Asset Catalog** — decrypts and parses `Game.swz` in place (Well512 PRNG) to load
  **71 legends**, **500+ costumes**, **500+ weapon skins**, and **70+ color schemes** directly from
  `HeroTypes.xml`, `costumeTypes.csv`, `weaponSkinTypes.csv`, and `ColorSchemeTypes.xml`.
- **Legend-First Workflow** — browse the roster with search and weapon filters, open a legend, and
  customize costume, primary/secondary weapon skins, and color palette in one workbench.
- **Independent Weapon Cosmetics** — swap primary and secondary weapon skins independently of the
  selected costume across all 14 weapon classes.
- **Intra-SWF & Cross-SWF Porting** — remap symbols inside a single SWF (e.g. Bödvar Classic →
  Beardvar) or inject shapes, sprites, and linkages across character packages.
- **Palette Baking & Recoloring** — replace baseline defines (`Body1_Define`, `Hair_Define`,
  `Cloth_Define`) with official color-scheme swaps or custom hex values.
- **Atomic Backup & Rollback** — SHA-256 fingerprints are recorded before any write, with
  timestamped snapshots in `.studio_backups/`. Single-file or full restoration returns the
  installation to a byte-identical pristine state.
- **Presets** — save, duplicate, export, and import complete cosmetic loadouts as portable JSON.
- **Diagnostics & Verification** — system health checks, SWF scanner, backup manager, and an in-game
  verification checklist.
- **Signed Update Flow** — update checks read the release asset digest (or `SHA256SUMS.txt`) and
  installation is refused unless the downloaded package matches the published SHA-256.
- **Bilingual UI, no RTL** — Arabic and English text can be switched at runtime. Layout direction is
  always LTR.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      FRONTEND LAYER                         │
│   React 19  •  Vite 8  •  Lucide Icons  •  CSS3 Tokens      │
│   Roster (LegendsPage) → LegendCustomizer                   │
│   Modals: Settings, Update, Card Picker, Toasts             │
└──────────────────────────────▲──────────────────────────────┘
                               │  JSON-RPC PostWebMessage
                               │  { action, requestId, payload }
┌──────────────────────────────▼──────────────────────────────┐
│                    WEBVIEW2 HOST & BRIDGE                   │
│   Microsoft Edge WebView2 Runtime  •  WebViewBridge.cs      │
│   Virtual Host: https://appassets.local/                    │
│   Dev Server:   http://localhost:5173/                      │
└──────────────────────────────▲──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                      BACKEND ENGINE                         │
│   C#  •  .NET 8  •  WPF Native Shell                        │
│   • GameDetector & CatalogService (Well512 decryption)      │
│   • ModificationEngine & SwfLib (vector tag rebinding)      │
│   • BackupManager (SHA-256 pristine rollbacks)              │
│   • PreviewRenderer (SVG/SWF previews)                      │
│   • PresetManager & DiagnosticsService                      │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Details |
|---|---|---|
| **Frontend UI** | React 19 + Vite 8 | HTML5, CSS3, Lucide Icons, dark theme (`#171717` / `#242424` / `#3B82F6`) |
| **Native Host** | WPF + Edge WebView2 | `Microsoft.Web.WebView2`, virtual host `appassets.local` |
| **Communication** | `WebViewBridge` (JSON-RPC) | 41 async actions with correlation IDs |
| **SWZ Engine** | Well512 PRNG | In-place decryption with dynamic seed key |
| **SWF Parser** | SwfLib | Shape reading, character re-indexing, symbol linkage mapping |
| **Integrity** | `BackupManager` | SHA-256 fingerprinting before writes; bounded snapshot history |

### Security model

- Update downloads are restricted to HTTPS URLs on GitHub-controlled hosts.
- A valid SHA-256 must be published for the asset; missing or mismatched checksums abort installation.
- ZIP updates are extracted with Zip Slip protection and size/entry-count limits.
- WebView navigation is restricted to the local virtual host and the dev server. External links open
  in the default browser and only over HTTP(S); script dialogs are disabled.
- Repository identifiers, version strings, and checksums are validated before use.

---

## Project Structure

```
BrawlCraft/
├── frontend/                               <-- React 19 + Vite web UI
│   ├── package.json                        <-- version 3.1.0
│   ├── vite.config.js
│   └── src/
│       ├── main.jsx
│       ├── App.jsx                         <-- global state, roster/customizer flow, updates
│       ├── version.js                      <-- APP_VERSION (single source for UI version)
│       ├── i18n/LanguageContext.jsx        <-- Arabic/English text switching (no RTL)
│       ├── pages/LegendsPage.jsx           <-- legend roster, search, weapon filter
│       ├── components/
│       │   ├── LegendCustomizer.jsx        <-- costume / weapon / palette workbench
│       │   ├── SettingsModal.jsx           <-- diagnostics, presets, backups, updates, about
│       │   ├── UpdateModal.jsx             <-- update prompt and progress
│       │   ├── CardPickerModal.jsx, CustomDropdown.jsx, SearchBox.jsx
│       │   ├── AssetImage.jsx, ErrorBoundary.jsx, TitleBar.jsx, Toast.jsx
│       │   └── (cached asset previews live in services/assetImageCache.js)
│       ├── services/api.js                 <-- WebView2 IPC client
│       └── styles/                         <-- variables.css, global.css, components.css
├── src/
│   ├── BrawlhallaSkinStudio.Core/          <-- engine library (.NET 8)
│   │   ├── Engine/BackupManager.cs
│   │   ├── Engine/ModificationEngine.cs
│   │   └── Services/                       <-- GameDetector, CatalogService, PreviewRenderer,
│   │                                           DiagnosticsService, PresetManager, GameLauncher
│   └── BrawlhallaSkinStudio.App/           <-- WPF host (.NET 8 Windows)
│       ├── MainWindow.xaml / MainWindow.xaml.cs
│       └── Bridge/WebViewBridge.cs         <-- 41 JSON-RPC backend actions
├── tests/BrawlhallaSkinStudio.Tests/       <-- xUnit suite (33 tests, 4 files)
├── installer.iss                           <-- Inno Setup 6 installer definition
├── scripts/release.ps1                     <-- build + publish + zip + SHA256SUMS
└── BrawlhallaSkinStudio.sln
```

---

## Build & Run

### Requirements

- Windows 10/11 x64
- .NET 8 SDK
- Node.js 20+
- Microsoft Edge WebView2 Runtime (included with Windows 11 and modern Windows 10)
- Brawlhalla installed via Steam for catalog and modification features

### Production run

```powershell
dotnet publish src\BrawlhallaSkinStudio.App\BrawlhallaSkinStudio.App.csproj `
    -c Release -r win-x64 --self-contained false -o publish

.\publish\BrawlCraft.exe
```

### Frontend development (hot reload)

```powershell
# Terminal 1
cd frontend
npm install
npm run dev

# Terminal 2 — MainWindow automatically connects to http://localhost:5173
dotnet run --project src\BrawlhallaSkinStudio.App\BrawlhallaSkinStudio.App.csproj
```

### Lint, build, test

```powershell
npm --prefix frontend run lint
npm --prefix frontend run build

dotnet build BrawlhallaSkinStudio.sln
dotnet test tests\BrawlhallaSkinStudio.Tests\BrawlhallaSkinStudio.Tests.csproj
```

> Some tests assert against a real Brawlhalla installation. Destructive tests operate on temporary
> copies and never write to the live game directory.

---

## Release

`scripts/release.ps1` reads the version from the App `.csproj`, builds the frontend, publishes the
application, creates `release/BrawlCraft-v<version>-Windows.zip`, and writes
`release/SHA256SUMS.txt`. If Inno Setup 6 (`ISCC.exe`) is installed, it also compiles the setup
executable; otherwise the step is skipped with a warning.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\release.ps1
```

The updater prefers the portable ZIP, falls back to the setup EXE, and requires a matching SHA-256
before installing.

---

## Versioning

- Backend version: `<Version>` / `AssemblyVersion` in `src/BrawlhallaSkinStudio.App/BrawlhallaSkinStudio.App.csproj`.
- Frontend version: `version` in `frontend/package.json`, re-exported by `frontend/src/version.js`.
- Installer version: `MyAppVersion` in `installer.iss`.

All three must be bumped together for a release.

---

## Automated Tests

33 xUnit tests across four suites:

| File | Tests | Coverage |
|---|---|---|
| `CatalogTests.cs` | 2 | Game detection, SWZ catalog parsing (71 legends), roster symbols |
| `LiveGameTests.cs` | 4 | Live file access, isolated copy/engine cycles, SWF path inspection |
| `ModificationTests.cs` | 10 | Backup/restore, intra- and cross-SWF edits, previews, presets, rejection rules |
| `SearchUiSymbols.cs` | 17 | Symbol/texture discovery, SVG export, weapon rendering, color mapping, Mordex isolation |

---

## Disclaimer

BrawlCraft is an unofficial fan project and is not affiliated with or endorsed by Blue Mammoth Games
or Ubisoft. Modifications are applied locally to your own installation; use them at your own risk and
follow Brawlhalla's Terms of Service. Always keep backups — the built-in restore flow makes this easy.

## License

MIT — see [LICENSE](LICENSE).
