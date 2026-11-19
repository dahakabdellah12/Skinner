import React, { useState, useMemo } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import SearchBox from '../components/SearchBox';
import CustomDropdown from '../components/CustomDropdown';
import CardPickerModal, { ColorEmblem } from '../components/CardPickerModal';
import AssetImage from '../components/AssetImage';
import { api } from '../services/api';

// Canonical in-game weapon ordering
const CANONICAL_WEAPON_ORDER = [
  'Hammer',
  'Sword',
  'Blasters',
  'Spear',
  'Rocket Lance',
  'Katars',
  'Bow',
  'Axe',
  'Gauntlets',
  'Scythe',
  'Cannon',
  'Orb',
  'Greatsword',
  'Battle Boots'
];

const getWeaponOrderIndex = (baseWeapon) => {
  if (!baseWeapon) return 999;
  const lower = baseWeapon.toLowerCase().replace(/[\s_-]+/g, '');
  const idx = CANONICAL_WEAPON_ORDER.findIndex(
    (w) => w.toLowerCase().replace(/[\s_-]+/g, '') === lower
  );
  return idx !== -1 ? idx : 999;
};

export default function WeaponsPage({
  catalog,
  systemInfo,
  showToast,
  isBusy,
  onApplyModification,
  onRestartGame,
  onLaunchGame
}) {
  const { isArabic, tWeapon } = useLanguage();

  // Search & Filter state for gallery
  const [search, setSearch] = useState('');
  const [weaponFilter, setWeaponFilter] = useState('All');

  // Customizer state
  const [activeWeapon, setActiveWeapon] = useState(null);
  const [targetWeapon, setTargetWeapon] = useState(null);
  const [replacementWeapon, setReplacementWeapon] = useState(null);
  const [selectedColorScheme, setSelectedColorScheme] = useState(null);
  const [activePicker, setActivePicker] = useState(null);
  const [autoRestart, setAutoRestart] = useState(() => {
    try {
      const saved = localStorage.getItem('brawlcraft_auto_restart');
      return saved !== null ? JSON.parse(saved) : true;
    } catch {
      return true;
    }
  });

  // Weapon Filter Dropdown Options in canonical order
  const weaponFilterOptions = useMemo(() => {
    const rawTypes = new Set();
    if (catalog?.weaponSkins) {
      for (const w of catalog.weaponSkins) {
        if (w.baseWeapon && w.baseWeapon !== '--') {
          rawTypes.add(w.baseWeapon);
        }
      }
    }

    const firstOpt = { value: 'All', label: isArabic ? 'جميع الأسلحة (الكل)' : 'All Weapons' };
    const rest = Array.from(rawTypes)
      .map((wt) => ({
        value: wt,
        label: tWeapon(wt),
        orderIndex: getWeaponOrderIndex(wt)
      }))
      .sort((a, b) => a.orderIndex - b.orderIndex);

    return [firstOpt, ...rest];
  }, [catalog?.weaponSkins, isArabic, tWeapon]);

  // Filtered & Canonically Ordered weapon skins for the gallery
  const filteredWeaponSkins = useMemo(() => {
    if (!catalog?.weaponSkins) return [];
    const q = search ? search.trim().toLowerCase() : '';
    const isAll = weaponFilter === 'All' || !weaponFilter;
    const normFilter = weaponFilter.toLowerCase();

    return catalog.weaponSkins
      .filter((w) => {
        // Exclude invalid / dummy entries
        if (!w.weaponSkinName || w.weaponSkinName === 'DEFAULT_WEAPON') return false;

        // Filter by weapon type
        if (!isAll && (w.baseWeapon || '').toLowerCase() !== normFilter) {
          return false;
        }

        // Filter by search query
        if (q) {
          const dName = (w.displayName || '').toLowerCase();
          const sName = (w.weaponSkinName || '').toLowerCase();
          const bWeapon = (w.baseWeapon || '').toLowerCase();
          const oHero = (w.ownerHero || '').toLowerCase();
          if (!dName.includes(q) && !sName.includes(q) && !bWeapon.includes(q) && !oHero.includes(q)) {
            return false;
          }
        }

        return true;
      })
      .sort((a, b) => {
        // Group by Canonical Weapon Order
        const catDiff = getWeaponOrderIndex(a.baseWeapon) - getWeaponOrderIndex(b.baseWeapon);
        if (catDiff !== 0) return catDiff;

        // Within same weapon type: default stock skin first
        if (a.isDefault && !b.isDefault) return -1;
        if (!a.isDefault && b.isDefault) return 1;

        // Then alphabetical
        return (a.displayName || a.weaponSkinName || '').localeCompare(b.displayName || b.weaponSkinName || '');
      });
  }, [catalog?.weaponSkins, search, weaponFilter]);

  // Open Weapon Customizer for a clicked weapon card
  const handleSelectWeapon = (skin) => {
    setActiveWeapon(skin);
    setReplacementWeapon(skin);

    // Find default stock weapon skin for this weapon type
    const defaultTarget = (catalog?.weaponSkins || []).find((w) =>
      (w.baseWeapon || '').toLowerCase() === (skin.baseWeapon || '').toLowerCase() &&
      (w.isDefault || (w.weaponSkinName || '').toLowerCase() === (skin.baseWeapon || '').toLowerCase() || w.costumeId === 0)
    ) || skin;

    setTargetWeapon(defaultTarget);
    setSelectedColorScheme(null);
  };

  // Back to gallery
  const handleBackToGallery = () => {
    setActiveWeapon(null);
    setActivePicker(null);
  };

  // Apply weapon modification
  const handleApply = async () => {
    if (!targetWeapon || !replacementWeapon) {
      showToast(isArabic ? 'يرجى تحديد السلاح المستهدف وسكن الاستبدال.' : 'Target and replacement weapons are required.', 'error');
      return;
    }

    const ownerLegend = catalog?.legends?.find(
      (l) => (l.heroName || '').toLowerCase() === (targetWeapon.ownerHero || '').toLowerCase()
    );

    const defaultCostume = ownerLegend?.costumes?.find((c) => c.isDefault) || ownerLegend?.costumes?.[0] || null;

    const req = {
      targetLegend: ownerLegend || null,
      targetCostume: defaultCostume,
      targetWeapon1: targetWeapon,
      replacementWeapon1: replacementWeapon,
      targetWeapon2: null,
      replacementWeapon2: null,
      selectedColorScheme: selectedColorScheme || null,
      autoRestart: autoRestart
    };

    await onApplyModification(req);
  };

  // Restore weapon modification
  const handleRestore = async () => {
    if (!targetWeapon) return;
    try {
      if (targetWeapon.ownerHero) {
        const count = await api.restoreLegend(targetWeapon.ownerHero);
        showToast(
          isArabic
            ? `تمت استعادة أسلحة وملفات "${targetWeapon.ownerHero}" لأصلها بنجاح (${count} ملف).`
            : `Restored files for "${targetWeapon.ownerHero}" successfully (${count} files).`,
          'success'
        );
      } else if (targetWeapon.swfFile) {
        await api.restoreCostume(targetWeapon.swfFile);
        showToast(
          isArabic ? 'تمت استعادة ملف السلاح الأصلي بنجاح.' : 'Target weapon SWF restored successfully.',
          'success'
        );
      }
    } catch (err) {
      showToast((isArabic ? 'فشلت الاستعادة: ' : 'Restore failed: ') + err.message, 'error');
    }
  };

  // Cards for CardPickerModal: Weapon skins for the current base weapon
  const currentBaseWeaponSkinsCards = useMemo(() => {
    if (!activeWeapon?.baseWeapon || !catalog?.weaponSkins) return [];
    const base = activeWeapon.baseWeapon.toLowerCase();

    return catalog.weaponSkins
      .filter((w) => (w.baseWeapon || '').toLowerCase() === base)
      .map((w) => {
        const ownerLegend = catalog?.legends?.find(
          (l) => (l.heroName || '').toLowerCase() === (w.ownerHero || '').toLowerCase()
        );
        const ownerDisp = ownerLegend ? ownerLegend.displayName : (w.ownerHero ? w.ownerHero : (isArabic ? 'عام' : 'Universal'));

        return {
          id: w.weaponSkinId != null && w.weaponSkinId !== 0 ? w.weaponSkinId : w.weaponSkinName,
          title: w.displayName || w.weaponSkinName,
          subtitle: `${tWeapon(w.baseWeapon)} • ${ownerDisp}`,
          badge: w.isDefault ? (isArabic ? 'افتراضي' : 'Default') : null,
          rawItem: w,
          searchKey: `${w.displayName || ''} ${ownerDisp} ${w.weaponSkinName || ''}`
        };
      })
      .sort((a, b) => {
        if (a.rawItem?.isDefault && !b.rawItem?.isDefault) return -1;
        if (!a.rawItem?.isDefault && b.rawItem?.isDefault) return 1;
        return (a.title || '').localeCompare(b.title || '');
      });
  }, [activeWeapon?.baseWeapon, catalog?.weaponSkins, catalog?.legends, isArabic, tWeapon]);

  // Cards for CardPickerModal: Color schemes
  const colorSchemeCards = useMemo(() => {
    const noneCard = {
      id: '__NONE__',
      title: isArabic ? 'الألوان الافتراضية (Classic)' : 'Default Colors (Classic)',
      subtitle: isArabic ? 'الألوان الأصلية للسلاح' : 'Stock Weapon Colors',
      isNone: true
    };

    if (!catalog?.colorSchemes) return [noneCard];

    const rest = catalog.colorSchemes
      .map((cs) => ({
        id: cs.schemeId || cs.schemeName,
        title: cs.displayName || cs.schemeName,
        subtitle: cs.theme || (isArabic ? 'صبغة لونية' : 'Palette'),
        rawItem: cs,
        colorData: cs,
        searchKey: `${cs.displayName || ''} ${cs.schemeName || ''} ${cs.theme || ''}`
      }))
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));

    return [noneCard, ...rest];
  }, [catalog?.colorSchemes, isArabic]);

  // =========================================================================
  // VIEW 1: WEAPON CUSTOMIZER (Character Studio Mirror Design)
  // =========================================================================
  if (activeWeapon) {
    const baseWeaponName = tWeapon(replacementWeapon?.baseWeapon || activeWeapon.baseWeapon);

    return (
      <div className="customizer-workbench">
        {/* Navigation Header */}
        <div className="workbench-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
            <button
              type="button"
              className="button-secondary"
              onClick={handleBackToGallery}
              style={{ padding: '8px 14px', fontSize: '13px' }}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M19 12H5M12 19l-7-7 7-7" />
              </svg>
              <span>{isArabic ? 'العودة إلى سكنات الأسلحة' : 'Back to Weapon Skins'}</span>
            </button>

            <div>
              <h2 style={{ fontSize: '18px', fontWeight: 800 }}>
                {replacementWeapon?.displayName || replacementWeapon?.weaponSkinName}
              </h2>
              <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                {baseWeaponName}
              </span>
            </div>
          </div>
        </div>

        {/* 2 Authentic Showcase Cards (Matching Legends Studio & KO Effect Studio 1:1) */}
        <div className="workbench-cards-grid" style={{ gridTemplateColumns: 'repeat(2, 1fr)' }}>
          {/* Card 1: Target Weapon in Game (A) */}
          <div className="store-card">
            <div className="card-image-wrap">
              <div className="card-image">
                <AssetImage
                  type="weapon"
                  item={targetWeapon}
                  style={{ width: '100%', height: '100%' }}
                  fallbackText={targetWeapon?.displayName || targetWeapon?.weaponSkinName}
                />
              </div>
              <span style={{
                position: 'absolute',
                top: '10px',
                right: '10px',
                color: 'var(--text-primary)',
                fontSize: '14px',
                fontWeight: 800,
                zIndex: 2,
                background: 'transparent',
                padding: 0
              }}>
                A
              </span>
            </div>
            <div className="card-content">
              <div className="card-heading">
                <h3 className="product-title" title={targetWeapon?.displayName || targetWeapon?.weaponSkinName}>
                  {targetWeapon?.displayName || targetWeapon?.weaponSkinName}
                </h3>
              </div>
              <p className="product-description">
                {isArabic ? 'السلاح الذي تملكه وسيتم استبدال شكله في اللعبة' : 'Weapon in your game inventory to be cosmetically replaced.'}
              </p>
            </div>
          </div>

          {/* Card 2: Replacement Weapon Skin (B) */}
          <div className="store-card">
            <div className="card-image-wrap">
              <div className="card-image">
                <AssetImage
                  type="weapon"
                  item={replacementWeapon}
                  colorScheme={selectedColorScheme}
                  style={{ width: '100%', height: '100%' }}
                  fallbackText={replacementWeapon?.displayName || replacementWeapon?.weaponSkinName}
                />
              </div>
              <span style={{
                position: 'absolute',
                top: '10px',
                right: '10px',
                color: 'var(--text-primary)',
                fontSize: '14px',
                fontWeight: 800,
                zIndex: 2,
                background: 'transparent',
                padding: 0
              }}>
                B
              </span>
            </div>
            <div className="card-content">
              <div className="card-heading">
                <h3 className="product-title" title={replacementWeapon?.displayName || replacementWeapon?.weaponSkinName}>
                  {replacementWeapon?.displayName || replacementWeapon?.weaponSkinName}
                </h3>
              </div>
              <p className="product-description">
                {isArabic ? 'المظهر الذي سيظهر في اللعبة أثناء المعارك' : 'The visual look you will see during combat and animations.'}
              </p>
            </div>
          </div>
        </div>

        {/* Configuration Controls Panel (Matching Legend Customizer Structure) */}
        <div className="config-panel">
          <h3 style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '14px' }}>
            {isArabic ? 'خيارات التخصيص للسلاح' : 'Weapon Customization Configuration'}
          </h3>

          {/* Weapons Triggers Row: Target Weapon & Replacement Weapon */}
          <div className="config-row">
            <div className="config-field">
              <label className="config-label">
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M14.5 17.5L3 6V3h3l11.5 11.5-3 3z" />
                </svg>
                <span>{isArabic ? 'السلاح المستهدف في مخزونك (Target Weapon)' : 'Target Weapon in Inventory'}</span>
              </label>
              <div
                className="card-selector-trigger"
                onClick={() => setActivePicker('targetWeapon')}
                title={isArabic ? 'اضغط لاختيار السلاح المستهدف' : 'Click to select target weapon from cards'}
              >
                <div className="card-selector-trigger-thumb">
                  <AssetImage
                    type="weapon"
                    item={targetWeapon}
                    style={{ width: '100%', height: '100%' }}
                  />
                </div>
                <div className="card-selector-trigger-info">
                  <div className="card-selector-trigger-title">
                    {targetWeapon?.displayName || targetWeapon?.weaponSkinName}
                  </div>
                  <div className="card-selector-trigger-sub">
                    {targetWeapon?.isDefault ? (isArabic ? '• السلاح الافتراضي' : '• Stock Weapon') : (targetWeapon?.ownerHero || '')}
                  </div>
                </div>
                <div className="card-selector-trigger-badge">
                  <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                    <rect x="3" y="3" width="7" height="7" />
                    <rect x="14" y="3" width="7" height="7" />
                    <rect x="14" y="14" width="7" height="7" />
                    <rect x="3" y="14" width="7" height="7" />
                  </svg>
                </div>
              </div>
            </div>

            <div className="config-field">
              <label className="config-label">
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M14.5 17.5L3 6V3h3l11.5 11.5-3 3z" />
                </svg>
                <span>{isArabic ? 'سكن السلاح البديل (Visual Replacement Skin)' : 'Visual Replacement Skin'}</span>
              </label>
              <div
                className="card-selector-trigger"
                onClick={() => setActivePicker('replacementWeapon')}
                title={isArabic ? 'اضغط لاختيار سكن السلاح البديل' : 'Click to select replacement weapon skin from cards'}
              >
                <div className="card-selector-trigger-thumb">
                  <AssetImage
                    type="weapon"
                    item={replacementWeapon}
                    colorScheme={selectedColorScheme}
                    style={{ width: '100%', height: '100%' }}
                  />
                </div>
                <div className="card-selector-trigger-info">
                  <div className="card-selector-trigger-title">
                    {replacementWeapon?.displayName || replacementWeapon?.weaponSkinName}
                  </div>
                  <div className="card-selector-trigger-sub">
                    {replacementWeapon?.ownerHero ? `${replacementWeapon.ownerHero}` : (isArabic ? `${currentBaseWeaponSkinsCards.length} سكن متاح` : `${currentBaseWeaponSkinsCards.length} skins available`)}
                  </div>
                </div>
                <div className="card-selector-trigger-badge">
                  <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                    <rect x="3" y="3" width="7" height="7" />
                    <rect x="14" y="3" width="7" height="7" />
                    <rect x="14" y="14" width="7" height="7" />
                    <rect x="3" y="14" width="7" height="7" />
                  </svg>
                </div>
              </div>
            </div>
          </div>

          {/* Color Scheme Strip (Exact Mirror of Character Studio Strip) */}
          <div className="config-field">
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="10" />
                <path d="M12 2a10 10 0 0 1 10 10c0 5.523-4.477 10-10 10" />
              </svg>
              <span>{isArabic ? 'نظام الألوان والصبغة (Color Palette)' : 'Color Scheme / Palette'}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('colorScheme')}
              title={isArabic ? 'اضغط لاختيار نظام الألوان' : 'Click to pick color palette from cards'}
            >
              <div className="card-selector-trigger-thumb" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <ColorEmblem colorData={selectedColorScheme} size={36} />
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {selectedColorScheme ? (selectedColorScheme.displayName || selectedColorScheme.schemeName) : (isArabic ? 'ألوان السلاح الأصلية (بدون تعديل)' : 'Default Weapon Colors')}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedColorScheme ? (isArabic ? 'أيقونة اللعبة الرسمية • صبغة معتمدة' : 'Official in-game color palette') : (isArabic ? `${catalog?.colorSchemes?.length || 0} نظام ألوان متاح مع أيقونات اللعبة` : `${catalog?.colorSchemes?.length || 0} palettes available`)}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
              </div>
            </div>
          </div>
        </div>

        {/* Action Footer Bar (Exact Mirror of Character Studio Order) */}
        <div className="workbench-footer">
          <label className="checkbox-label" style={{ cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '8px' }}>
            <input
              type="checkbox"
              checked={autoRestart}
              onChange={(e) => {
                const val = e.target.checked;
                setAutoRestart(val);
                try {
                  localStorage.setItem('brawlcraft_auto_restart', JSON.stringify(val));
                } catch { }
              }}
              style={{ accentColor: '#3b82f6', width: '16px', height: '16px', cursor: 'pointer' }}
            />
            <span>{isArabic ? 'إعادة تشغيل اللعبة تلقائياً لتطبيق المظهر فوراً' : 'Auto restart game instantly to apply visual change'}</span>
          </label>

          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <button
              className="button-secondary"
              onClick={handleRestore}
              disabled={isBusy}
              title={isArabic ? 'استعادة ملف السلاح الأصلي' : 'Restore target weapon to original SWF'}
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="1 4 1 10 7 10" />
                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
              </svg>
              <span>{isArabic ? 'استعادة الأصلية' : 'Restore Original'}</span>
            </button>

            {onRestartGame && systemInfo?.gameRunning && (
              <button
                className="button-secondary"
                onClick={onRestartGame}
                disabled={isBusy}
                title={isArabic ? 'إعادة تشغيل اللعبة وتطبيق التعديلات المعلقة' : 'Terminate game, apply queued modifications, and restart'}
              >
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.19" />
                </svg>
                <span>{isArabic ? 'تحديث اللعبة' : 'Restart Game'}</span>
              </button>
            )}

            {onLaunchGame && !systemInfo?.gameRunning && (
              <button
                type="button"
                className="button-secondary"
                onClick={onLaunchGame}
                disabled={isBusy}
                title={isArabic ? 'تشغيل اللعبة الآن' : 'Launch game executable'}
              >
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                  <polygon points="5 3 19 12 5 21 5 3" />
                </svg>
                <span>{isArabic ? 'تشغيل اللعبة' : 'Launch Game'}</span>
              </button>
            )}

            <button
              className="button-primary"
              onClick={handleApply}
              disabled={isBusy || !targetWeapon || !replacementWeapon}
            >
              <span>{isBusy ? (isArabic ? 'جاري التطبيق...' : 'Applying...') : (isArabic ? 'تطبيق التعديل على اللعبة' : 'Apply Modification to Game')}</span>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M7 17L17 7M17 7H7M17 7V17" />
              </svg>
            </button>
          </div>
        </div>

        {/* Modals for CardPicker */}
        {activePicker === 'targetWeapon' && (
          <CardPickerModal
            isOpen={true}
            onClose={() => setActivePicker(null)}
            title={isArabic ? 'اختيار السلاح المستهدف في اللعبة' : 'Select Target Weapon in Game'}
            subtitle={isArabic ? `اختر السلاح الذي تملكه وسيتم استبدال مظهره` : `Weapon in inventory to be replaced`}
            items={currentBaseWeaponSkinsCards}
            selectedId={targetWeapon?.weaponSkinId != null && targetWeapon.weaponSkinId !== 0 ? targetWeapon.weaponSkinId : targetWeapon?.weaponSkinName}
            onSelect={(selectedId, cardItem) => {
              const item = cardItem?.rawItem || currentBaseWeaponSkinsCards.find((c) => String(c.id) === String(selectedId))?.rawItem;
              if (item) {
                setTargetWeapon(item);
              }
              setActivePicker(null);
            }}
            type="weapon"
            baseWeapon={activeWeapon.baseWeapon}
          />
        )}

        {activePicker === 'replacementWeapon' && (
          <CardPickerModal
            isOpen={true}
            onClose={() => setActivePicker(null)}
            title={isArabic ? 'اختيار سكن السلاح البديل' : 'Select Replacement Weapon Skin'}
            subtitle={isArabic ? `اختر سكن السلاح الجديد الذي تريد رؤيته في اللعبة` : `Choose the skin you want to wield in game`}
            items={currentBaseWeaponSkinsCards}
            selectedId={replacementWeapon?.weaponSkinId != null && replacementWeapon.weaponSkinId !== 0 ? replacementWeapon.weaponSkinId : replacementWeapon?.weaponSkinName}
            onSelect={(selectedId, cardItem) => {
              const item = cardItem?.rawItem || currentBaseWeaponSkinsCards.find((c) => String(c.id) === String(selectedId))?.rawItem;
              if (item) {
                setReplacementWeapon(item);
              }
              setActivePicker(null);
            }}
            type="weapon"
            baseWeapon={activeWeapon.baseWeapon}
            colorScheme={selectedColorScheme}
          />
        )}

        {activePicker === 'colorScheme' && (
          <CardPickerModal
            isOpen={true}
            onClose={() => setActivePicker(null)}
            title={isArabic ? 'اختيار لون السلاح' : 'Select Weapon Color Scheme'}
            subtitle={isArabic ? `اختر الصبغة اللونية الرسمية لتلوين سكن السلاح` : `Choose an authentic game palette for this weapon`}
            items={colorSchemeCards}
            selectedId={selectedColorScheme ? (selectedColorScheme.schemeId || selectedColorScheme.schemeName) : '__NONE__'}
            onSelect={(selectedId, cardItem) => {
              if (selectedId === '__NONE__') {
                setSelectedColorScheme(null);
              } else {
                const item = cardItem?.rawItem || colorSchemeCards.find((c) => String(c.id) === String(selectedId))?.rawItem;
                setSelectedColorScheme(item || null);
              }
              setActivePicker(null);
            }}
            type="color"
          />
        )}
      </div>
    );
  }

  // =========================================================================
  // VIEW 2: WEAPONS GALLERY (Ordered Canonical Catalog)
  // =========================================================================
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      {/* Top Filter and Search Bar (Exact same layout and styling as LegendsPage) */}
      <div className="roster-header-glass" style={{ display: 'flex', gap: '14px', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flex: 1, minWidth: '280px' }}>
          <SearchBox
            value={search}
            onChange={setSearch}
            placeholder={isArabic ? 'بحث سريع عن سكن سلاح (Ctrl + K)...' : 'Quick search weapon skins (Ctrl + K)...'}
          />

          <div style={{ width: '200px' }}>
            <CustomDropdown
              options={weaponFilterOptions}
              value={weaponFilter}
              onChange={setWeaponFilter}
              placeholder={isArabic ? 'نوع السلاح' : 'Weapon Type'}
              showSearch={false}
            />
          </div>
        </div>

        <div style={{ fontSize: '13px', color: 'var(--text-muted)' }}>
          {isArabic
            ? `${filteredWeaponSkins.length} سكن سلاح`
            : `${filteredWeaponSkins.length} Weapon Skins`}
        </div>
      </div>

      {/* Weapons Grid */}
      <div className="legends-grid">
        {filteredWeaponSkins.map((weaponSkin) => {
          const ownerLegend = catalog?.legends?.find(
            (l) => (l.heroName || '').toLowerCase() === (weaponSkin.ownerHero || '').toLowerCase()
          );
          const ownerDisp = ownerLegend ? ownerLegend.displayName : (weaponSkin.ownerHero || (isArabic ? 'سكن عام' : 'Universal'));

          return (
            <div
              key={weaponSkin.weaponSkinId || weaponSkin.weaponSkinName}
              className="store-card"
              style={{ cursor: 'pointer' }}
              onClick={() => handleSelectWeapon(weaponSkin)}
            >
              <div className="card-image-wrap">
                <div className="card-image" style={{ padding: '12px' }}>
                  <AssetImage
                    type="weapon"
                    item={weaponSkin}
                    style={{ width: '100%', height: '100%' }}
                    fallbackText={weaponSkin.displayName || weaponSkin.weaponSkinName}
                  />
                </div>
              </div>

              <div className="card-content">
                <div className="card-heading">
                  <h3 className="product-title" title={weaponSkin.displayName || weaponSkin.weaponSkinName}>
                    {weaponSkin.displayName || weaponSkin.weaponSkinName}
                  </h3>
                </div>

                <p className="product-description">
                  {tWeapon(weaponSkin.baseWeapon)} • {ownerDisp}
                </p>

                <div className="card-divider" />

                <div className="card-footer">
                  <button
                    type="button"
                    className="card-button"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleSelectWeapon(weaponSkin);
                    }}
                  >
                    <span>{isArabic ? 'تخصيص السلاح' : 'Customize Weapon'}</span>
                  </button>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
