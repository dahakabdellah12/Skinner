# Skinner v3.3.5 — Silent In-Place Updates, UI Refinements & Gnash Cleanup

### 🌟 What's New in v3.3.5

#### 1. Streamlined Update Experience & Seamless Auto-Relaunch
- **Silent In-Place Installation:** Background installer execution now runs 100% silently (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /DIR=...`), completely eliminating Setup wizard dialogs and options during automatic updates.
- **Instant App Relaunch:** Skinner automatically reopens immediately once the update completes, eliminating manual steps.
- **Minimalist Update Dialog:**
  - Removed close (x) button to streamline modal flow.
  - Removed redundant "View Changelog" button.
  - Removed header download icon for a clean, minimalist header.
  - Simplified dialog title to direct phrase "تحديث جديد" / "New Update".
  - Cleaned up progress bar metrics to eliminate confusing "0" number artifact.

#### 2. Gnash (Nash) Catalog Pruning
- Pruned all three ZombieWalker variant skins (`ZombieWalker1` ID 482, `ZombieWalker2` ID 483, `ZombieWalker3` ID 484) from Gnash (`caveman`), leaving exactly 12 authentic distinct costumes.

---

### 📦 Release Assets & Cryptographic Checksums

| File | Type | Size | SHA-256 Digest |
| :--- | :--- | :--- | :--- |
| **`Skinner-Setup-3.3.5.exe`** | Windows Inno Setup Installer | 54.9 MB | `315dce44287e83873b18813d795b6f702349d49b91a5c71795cc3c1f3f7e5a23` |
| **`Skinner-3.3.5-setup-x64.exe`** | Windows Setup (Alternate Naming) | 54.9 MB | `315dce44287e83873b18813d795b6f702349d49b91a5c71795cc3c1f3f7e5a23` |
| **`Skinner-3.3.5-windows-x64.zip`** | Portable Windows Package | 76.8 MB | `2c2af3b939ab5febba273d1ae51c09ec293696b4a2e820ce811f9b4d47b5bbb3` |
| **`SHA256SUMS.txt`** | Authenticated Manifest | 362 bytes | Checksum verification file |
| **`SHA256SUMS.txt.sig`** | Digital Signature | 64 bytes | Ed25519 signature |
