# Skinner v3.3.4 — Final Catalog Cleanup, Targeted Removals & Duplicate Fixes

### 🌟 What's New in v3.3.4

#### 1. Targeted Skin Precision Removals (13 records across 12 characters)
- **Ezio:** Anakin Skywalker (`Anakin`, Costume ID: 641) removed.
- **Petra:** Ken (`SFKen`, Costume ID: 574) removed.
- **Rayman:** Dhalsim (`SFDhalsim`, Costume ID: 573) removed.
- **Jiro:** Yoshimitsu (`Yoshimitsu`, Costume ID: 637) removed.
- **Kor:** Four Arms (`FourArms`, Costume ID: 456) removed.
- **Koji:** Sei Jo-Kai Koji / Shen Sekai Koji (`CyberSam`, Costume ID: 376) removed.
- **Asuri:** Puss in Boots (`Puss`, Costume ID: 793) removed.
- **Red Raptor:** Eren Yeager (`Eren`, Costume ID: 735) removed.
- **Teros:** Roman Reigns (`RomanReigns`, Costume ID: 421) and Gruagach / Goroagash (`Gruagach`, Costume ID: 396) removed.
- **Ada:** Heatblast (`Heatblast`, Costume ID: 454) removed.
- **Lord Vraxx:** Alien Warlord (`AlienWarlord`, Costume ID: 309) removed.
- **Nash (Gnash):** Becky Lynch (`Becky`, Costume ID: 409) removed.

#### 2. Duplicate Stance Record Pruning (3 characters)
- **Priya (`BladeDancer`, ID 67):** Pruned redundant `BladeDancerStance2` (Costume ID 964); canonical default `BladeDancer` (Costume ID 965) and 4 other skins preserved (5 total).
- **Aurus (`ActualGladiator`, ID 71):** Pruned redundant `ActualGladiatorStance2` (Costume ID 785); canonical default `ActualGladiator` (Costume ID 783) and 3 other skins preserved (4 total).
- **Lady Vera (`Cleric`, ID 69):** Pruned redundant `ClericStance2` (Costume ID 738); canonical default `Cleric` (Costume ID 737) and 4 other skins preserved (5 total).
- **Nash (`Caveman`, ID 7):** Verified full catalog; no stance duplicate exists. Becky Lynch (409) and Shovel Knight (355) removed; all 15 distinct skins preserved, including ZombieWalker1, 2, and 3.

#### 3. Complete Dual-Installer & Cryptographic Distribution
- Distributes both `Skinner-Setup-3.3.4.exe` and `Skinner-3.3.4-setup-x64.exe` to guarantee seamless automated in-app updates across all previous versions.
- All release assets are cryptographically authenticated via `SHA256SUMS.txt` and signed using official **Ed25519** digital signatures (`SHA256SUMS.txt.sig`).

---

### 📦 Release Assets & Cryptographic Checksums

| File | Type | Size | SHA-256 Digest |
| :--- | :--- | :--- | :--- |
| **`Skinner-Setup-3.3.4.exe`** | Windows Inno Setup Installer | 54.9 MB | `82d4dd03f19292d425e52e49187a7f9f83b233d1c52fe2ad46f9adeb50b8d7f4` |
| **`Skinner-3.3.4-setup-x64.exe`** | Windows Setup (Alternate Naming) | 54.9 MB | `82d4dd03f19292d425e52e49187a7f9f83b233d1c52fe2ad46f9adeb50b8d7f4` |
| **`Skinner-3.3.4-windows-x64.zip`** | Portable Windows Package | 76.8 MB | `cd4572618363748c0bf8436f2047995107a65e040396c768a8a0e99801a961a1` |
| **`SHA256SUMS.txt`** | Authenticated Manifest | 362 bytes | Checksum verification file |
| **`SHA256SUMS.txt.sig`** | Digital Signature | 64 bytes | Ed25519 signature |
