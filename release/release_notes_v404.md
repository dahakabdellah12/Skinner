# Skinner v4.0.4 — Crisp KO Previews, Unified Weapon Studio & Performance Polish

### ✨ Key Features & Improvements

1. **Lockout During Startup Preload & 60fps Window Motion**:
   - Tab switching in the Sidebar and accessing the Settings modal from the TitleBar are now disabled during initial preloading with visual muted feedback.
   - Micro-yielding (16ms) between batch chunks prevents IPC thread starvation and ensures buttery-smooth window dragging, maximizing, and resizing.

2. **Complete Asset Preloading**:
   - Expanded the preloading pipeline (`preloadService.js`) to cache all weapon skins (`catalog.weaponSkins`), playable legends, costumes, knockout effects, and color schemes during startup.
   - Instant zero-wait navigation when switching between Legends, KO Effects, and Weapon Skins.

3. **Crisp Single-Frame KO Effect Previews**:
   - Fixed renderer frame extraction in `PreviewRenderer.cs` to capture the iconic midpoint/peak frame instead of superimposing all animated frames simultaneously.
   - Completely resolved blurry smoke dissipation artifacts across all knockout effect cards and studio showcases.
   - Isolated new SVG cache storage to `cache/svg_v4` to permanently bypass any outdated or corrupted renders.

4. **Harmonized Weapon Customizer & Target Selection Fix**:
   - The Weapon Customizer layout now mirrors the KO Effect Customizer and Legend Customizer 1:1 with balanced side-by-side cards (`repeat(2, 1fr)`).
   - Fixed the `CardPickerModal` item selection callback so target weapon selection reliably updates the target weapon and triggers appropriate visual updates.

5. **Elimination of UI Glow Artifacts**:
   - Completely removed all fuzzy box shadows and glow effects from toggle switches (in Settings -> Notifications & Updates), primary buttons, and active pills.

6. **Cryptographic Integrity & Signed Release**:
   - Release binaries verified via SHA-256 manifests and signed with official Ed25519 key.
