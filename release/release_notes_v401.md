# Skinner v4.0.1 — Sleek Studio UI, Authentic KO Effects, Instant Navigation & Polished Presets

### 🌟 What's New & Fixed in v4.0.1:

1. **Sleek Settings-Style Preset Dropdown (مستدلة أنيقة كالإعدادات):**
   - Replaced crowded horizontal tab pills with a clean, compact dropdown selector matching the application design system.
   - Shows active preset (`Skin 1`, `Skin 2` or custom name) with instant switching, clear checkmarks, and intuitive preset management.

2. **Dedicated In-App Slot Rename Modal (نافذة تسمية الخانات المدمجة):**
   - Completely replaced default native browser prompts (`brawlcraft.localhost says:`) with a bespoke in-app dark-themed modal.
   - Supports keyboard shortcuts (`Enter` to confirm, `Esc` to cancel) and smooth focus animations.

3. **Restored Full-Fidelity KO Effects (تأثيرات الإطاحة فائقة الدقة والوضوح):**
   - Reverted premature midpoint frame slicing that caused fragment distortions.
   - Restored complete, authentic composite timeline SVG rendering with crisp visuals across all 49 knockout effects.

4. **Fixed Sidebar & Scrollbar Layout (استعادة الشريط الجانبي وشريط التمرير بالكامل):**
   - Restored the vertical docked sidebar on the left with dedicated navigation icons for Legends and KO Effects.
   - Bounded viewport height correctly, fixing scrollbar visibility and enabling fluid scrolling.

5. **Instant Zero-Lag Tab Switching (تنقل فوري وسلس بين التبويبات):**
   - Converted views to keep-alive DOM rendering, eliminating frame drops and DOM reconstruction when switching between Legends and KO Effects.

6. **Ultra-Smooth Window Dragging During Startup (سلاسة تامة أثناء تحريك النافذة وقت التحميل):**
   - Balanced asset preloading batch chunk sizes and introduced 16ms message pump yields, ensuring 60fps window drag responsiveness.

7. **Cryptographic Ed25519 Signing & Checksums:**
   - All update artifacts signed with the official Ed25519 private key and verified with SHA-256 manifests.

### 📦 Checksums & Artifacts:
- `Skinner-Setup-4.0.1.exe`: SHA-256 `5657a4e16ea56d325c716d484bae2dfda481ecc0b69579304e2cf14dc155f266`
- `Skinner-4.0.1-windows-x64.zip`: SHA-256 `8cfacb132522486f86871a7e4cc7fb2a57c6d5b06ed1064f4d7ac55342382c03`
- `Skinner.exe`: SHA-256 `2147599bae76188c0bc1498568e36b66c3bc56fa15e5e8ade29435ac54f51126`
