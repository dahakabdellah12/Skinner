# Skinner v4.0.0 — Multi-Slot Character Customization, High-Res Midpoint KO Effects & Authentic Game Catalog

### 🌟 What's New in v4.0.0

#### 1. Multi-Slot Customization Studio (Skin 1, Skin 2, ...)
- **Customization Slots / Tabs:** You can now create multiple modification setups per character (`Skin 1`, `Skin 2`, etc.).
- **Slot Management:** Easily add new slots (`+ New Slot`), duplicate existing configurations, rename slots, reset, or delete with instant switching.
- **Persistent Storage:** All slot configurations are safely stored in your local profile and automatically restored whenever you select a character.

#### 2. High-Definition Single Midpoint Frame KO Previews
- **Crisp Midpoint Extraction:** KO effects now capture exactly one key frame from the center of the animation timeline (`targetFrame = max(1, FramesCount / 2)`), producing clean, distortion-free vector previews.
- **Studio & Page Consistency:** The improved rendering applies both in the KO Effects gallery and inside the dedicated KO Effect Customizer studio.

#### 3. Ghost & Missing KO Effects Pruned
- Removed ghost effects whose animation symbols or assets do not exist in the game archives (such as `BP13KO` and `TitanChompKO`).
- Validated every effect dynamically against `SFX_KO.swf` and `SFX_KO2.swf`.

#### 4. Loading Screen Preload Optimization
- The initial loading screen now proactively preloads active tabs (including Legends and verified KO Effects), eliminating navigation lag.

#### 5. 100% Authentic In-Game Names
- Restored original authentic English game strings across the entire catalog (over 13,900 strings).
- Fixed internal/corrupted names for Legend skins (`Default Diana`, etc.), weapon skins (`Mastercraft Recurve`, `Five-Sevens`, etc.), and KO effects (`Divine Encounter`, `Fallout`, `Loki's Grasp`, etc.).

---

### 📦 Release Assets & Cryptographic Checksums

| File | Type | Size | SHA-256 Digest |
| :--- | :--- | :--- | :--- |
| **`Skinner-Setup-4.0.0.exe`** | Windows Inno Setup Installer | 44.8 MB | `85c5fcda9d82950f78ae9267b7f176027b7ea466c7a25c9aa518ca7af880bda4` |
| **`Skinner-4.0.0-setup-x64.exe`** | Windows Setup (Alternate Naming) | 44.8 MB | `85c5fcda9d82950f78ae9267b7f176027b7ea466c7a25c9aa518ca7af880bda4` |
| **`Skinner-4.0.0-windows-x64.zip`** | Portable Windows Package | 90.8 MB | `7f6c181685338829e5b6faf4002228f0ef94d9d1841cfe27ffae47c61eb9eb88` |
| **`SHA256SUMS.txt`** | Authenticated Manifest | 362 bytes | Checksum verification file |
| **`SHA256SUMS.txt.sig`** | Digital Signature | 64 bytes | Ed25519 digital signature |
