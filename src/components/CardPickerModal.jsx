import React, { useState, useEffect, useMemo, useRef } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import AssetImage from './AssetImage';
import {
  COLOR_SCHEME_ICON_MAP,
  COLOR_NAME_ICON_MAP,
  getColorIconSymbol,
  CORE_COLOR_DEFAULTS
} from '../utils/colorUtils';

export {
  COLOR_SCHEME_ICON_MAP,
  COLOR_NAME_ICON_MAP,
  getColorIconSymbol,
  CORE_COLOR_DEFAULTS
};

function uintToHex(val, fallback = '#6b7280') {
  if (val == null || val === undefined) return fallback;
  const num = Number(val);
  if (isNaN(num) || num === 0) return fallback;
  return '#' + num.toString(16).padStart(6, '0');
}

export function isCoreColorScheme(colorData) {
  if (!colorData || colorData.isNone) return true;
  const id = Number(colorData.schemeId);
  if ([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 19, 20].includes(id)) return true;
  const name = String(colorData.schemeName || colorData.displayName || '').replace(/[\s\-_]/g, '').toLowerCase();
  return ['red', 'blue', 'yellow', 'green', 'orange', 'purple', 'brown', 'cyan', 'pink', 'white', 'black', 'sunset', 'grey', 'gray', 'default', 'none', 'stock'].includes(name);
}

export function GemStoneSvg({ colorData, size = 68, className = '' }) {
  const isNone = !colorData || colorData.isNone;
  const cid = Number(colorData?.schemeId || 0);
  const cname = String(colorData?.schemeName || colorData?.displayName || '').replace(/[\s\-_]/g, '').toLowerCase();
  const def = CORE_COLOR_DEFAULTS[cid] || CORE_COLOR_DEFAULTS[cname] || (isNone ? CORE_COLOR_DEFAULTS.default : null);

  const ind = isNone
    ? (def?.ind || '#38bdf8')
    : uintToHex(colorData?.indicatorColor, def?.ind || '#3b82f6');

  const sw = colorData?.swapColors || {};
  const b1 = uintToHex(sw.body1 ?? sw.Body1 ?? sw.hair, def?.b1 || ind);
  const b2 = uintToHex(sw.body2 ?? sw.Body2 ?? sw.cloth, def?.b2 || b1);
  const sp = uintToHex(sw.special ?? sw.Special, def?.sp || b1);
  const wp = uintToHex(sw.weapon ?? sw.Weapon, def?.wp || b2);

  return (
    <svg
      viewBox="0 0 100 100"
      width={size}
      height={size}
      className={className}
      style={{
        filter: 'drop-shadow(0 4px 10px rgba(0,0,0,0.55))',
        display: 'block'
      }}
    >
      <defs>
        <linearGradient id={`gemGloss_${size}`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.42" />
          <stop offset="100%" stopColor="#ffffff" stopOpacity="0" />
        </linearGradient>
        <linearGradient id={`stoneBevel_${size}`} x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#475569" />
          <stop offset="100%" stopColor="#1e222d" />
        </linearGradient>
      </defs>

      {/* Outer Chiseled Stone Plaque */}
      <polygon
        points="50,4 90,26 90,74 50,96 10,74 10,26"
        fill="#12151c"
        stroke={`url(#stoneBevel_${size})`}
        strokeWidth="3.2"
        strokeLinejoin="round"
      />
      {/* Inner Stone Inlay Rim */}
      <polygon
        points="50,10 84,29 84,71 50,90 16,71 16,29"
        fill="#0d0f15"
        stroke="rgba(255,255,255,0.08)"
        strokeWidth="1.2"
        strokeLinejoin="round"
      />

      {/* Gemstone Crystal Facets */}
      {/* Top Facet (Body 1 / Armor) */}
      <polygon points="50,15 80,31 50,50 20,31" fill={b1} />
      <polygon points="50,15 80,31 50,50 20,31" fill={`url(#gemGloss_${size})`} />

      {/* Right Facet (Body 2 / Cloth) */}
      <polygon points="80,31 80,69 50,50" fill={b2} />
      <polygon points="80,31 80,69 50,50" fill="#000000" opacity="0.18" />

      {/* Bottom Facet (Special / Accent) */}
      <polygon points="80,69 50,85 20,69 50,50" fill={sp} />
      <polygon points="80,69 50,85 20,69 50,50" fill="#000000" opacity="0.25" />

      {/* Left Facet (Weapon / Details) */}
      <polygon points="20,69 20,31 50,50" fill={wp} />
      <polygon points="20,69 20,31 50,50" fill="#ffffff" opacity="0.12" />

      {/* Facet Divider Lines */}
      <line x1="50" y1="15" x2="50" y2="85" stroke="rgba(0,0,0,0.35)" strokeWidth="1.2" />
      <line x1="20" y1="31" x2="80" y2="69" stroke="rgba(0,0,0,0.22)" strokeWidth="1.2" />
      <line x1="20" y1="69" x2="80" y2="31" stroke="rgba(0,0,0,0.22)" strokeWidth="1.2" />

      {/* Central Raised Diamond Rune Jewel */}
      <polygon
        points="50,37 63,50 50,63 37,50"
        fill={ind}
        stroke="#0b0d13"
        strokeWidth="2.2"
        strokeLinejoin="round"
      />
      <polygon points="50,37 63,50 50,50 37,50" fill="#ffffff" opacity="0.32" />
      <circle cx="50" cy="50" r="3" fill="#ffffff" opacity="0.8" />
    </svg>
  );
}

export function ColorEmblem({ colorData, size = 68, className = '' }) {
  const isCore = isCoreColorScheme(colorData);
  const fallbackGem = <GemStoneSvg colorData={colorData} size={size} className={className} />;

  // Core colors always render the authentic multi-faceted gemstone symbol directly!
  if (isCore) {
    return (
      <div
        className={`color-gem-emblem ${className}`}
        style={{
          width: size,
          height: size,
          minWidth: size,
          minHeight: size,
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          position: 'relative'
        }}
      >
        {fallbackGem}
      </div>
    );
  }

  const iconSymbol = getColorIconSymbol(colorData);
  const itemObj = {
    ...(colorData || {}),
    iconName: iconSymbol,
    icon: iconSymbol
  };

  return (
    <div
      className={`color-gem-emblem ${className}`}
      style={{
        width: size,
        height: size,
        minWidth: size,
        minHeight: size,
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        position: 'relative'
      }}
    >
      <AssetImage
        type="color"
        item={itemObj}
        style={{ width: '100%', height: '100%', objectFit: 'contain' }}
        fallbackIcon={() => fallbackGem}
      />
    </div>
  );
}

export default function CardPickerModal({
  isOpen,
  onClose,
  title,
  subtitle,
  items = [],
  selectedId,
  onSelect,
  type = 'costume', // 'costume' | 'weapon' | 'hero' | 'color'
  baseWeapon = '',
  colorScheme = null,
  ownerHero = '',
  costumeName = ''
}) {
  const { isArabic } = useLanguage();
  const [search, setSearch] = useState('');
  const searchInputRef = useRef(null);

  // Focus search input when modal opens
  useEffect(() => {
    if (isOpen) {
      const t = setTimeout(() => {
        setSearch('');
        searchInputRef.current?.focus();
      }, 50);
      return () => clearTimeout(t);
    }
  }, [isOpen]);

  // Close on Escape key
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  // Pre-sanitize items only when items list changes
  const sanitizedItems = useMemo(() => {
    return items
      .map((it) => {
        if (!it) return null;
        const id = it.id ?? it.koEffectName ?? it.costumeId ?? it.heroName ?? it.schemeId;
        const title = it.title ?? it.displayName ?? it.heroName ?? it.costumeName ?? it.schemeName ?? '';
        const subtitle = it.subtitle ?? (it.category ? `${it.category} • ${it.swfFile || ''}` : it.swfFile) ?? '';
        const badge = it.badge ?? (it.isDefault ? (isArabic ? 'افتراضي' : 'Default') : it.category);
        const rawItem = it.rawItem ?? it;
        return {
          ...it,
          id,
          title,
          subtitle,
          badge,
          rawItem
        };
      })
      .filter((it) => {
        if (!it) return false;
        const titleLower = (it.title || '').toLowerCase();
        const idLower = String(it.id || '').toLowerCase();
        return titleLower !== 'random' && idLower !== 'random' && idLower !== '__random_roll__';
      });
  }, [items, isArabic]);

  // Filter items by search query in a single pass
  const filteredItems = useMemo(() => {
    const q = search ? search.trim().toLowerCase() : '';
    if (!q) return sanitizedItems;

    const specials = [];
    const matches = [];

    for (let i = 0; i < sanitizedItems.length; i++) {
      const it = sanitizedItems[i];
      if (it.isNone) {
        specials.push(it);
        continue;
      }
      const titleMatch = it.title && it.title.toLowerCase().includes(q);
      const subMatch = it.subtitle && it.subtitle.toLowerCase().includes(q);
      const searchKeyMatch = it.searchKey && it.searchKey.toLowerCase().includes(q);
      if (titleMatch || subMatch || searchKeyMatch) {
        matches.push(it);
      }
    }

    return specials.length > 0 ? specials.concat(matches) : matches;
  }, [sanitizedItems, search]);

  if (!isOpen) return null;

  const weaponClass = type === 'weapon'
    ? `weapon-${(baseWeapon || '').toLowerCase().replace(/[\s_]+/g, '')}`
    : '';

  return (
    <div className="card-picker-overlay" onClick={onClose}>
      <div
        className="card-picker-modal"
        onClick={(e) => e.stopPropagation()}
        style={{ direction: 'ltr' }}
      >
        {/* Header Bar */}
        <div className="card-picker-header">
          <div className="card-picker-header-title">
            <h3>{title}</h3>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flex: 1, justifyContent: 'flex-end', flexWrap: 'wrap' }}>
            <div className="card-picker-search-bar">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" style={{ color: 'var(--text-muted)', flexShrink: 0 }}>
                <circle cx="11" cy="11" r="8" />
                <path d="M21 21l-4.35-4.35" />
              </svg>
              <input
                ref={searchInputRef}
                type="text"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder={isArabic ? 'بحث سريع بالاسم...' : 'Quick search by name...'}
              />
              {search && (
                <button
                  type="button"
                  onClick={() => setSearch('')}
                  style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', padding: 0 }}
                  title={isArabic ? 'مسح' : 'Clear'}
                >
                  ✕
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Scrollable Fixed-Size Cards Grid */}
        <div className={`card-picker-grid ${type === 'hero' ? 'hero-grid' : ''}`}>
          {filteredItems.length === 0 ? (
            <div style={{ gridColumn: '1 / -1', textAlign: 'center', padding: '60px 0', color: 'var(--text-muted)' }}>
              <p style={{ fontSize: '15px', fontWeight: 600 }}>
                {isArabic ? 'لا توجد عناصر مطابقة لبحثك' : 'No items found matching your search'}
              </p>
              <p style={{ fontSize: '12px', marginTop: '6px' }}>
                {isArabic ? 'جرب البحث باسم مختلف' : 'Try searching with different keywords'}
              </p>
            </div>
          ) : (
            filteredItems.map((item) => {
              const isSelected = String(item.id) === String(selectedId);

              // 1. Character / Hero selector: Visual cards
              if (type === 'hero') {
                return (
                  <div
                    key={String(item.id)}
                    className={`card-picker-item ${isSelected ? 'active' : ''}`}
                    onClick={() => {
                      if (onSelect) onSelect(item.id, item);
                      onClose();
                    }}
                    title={item.title}
                  >
                    <div className="card-picker-image-wrap">
                      <div className="card-image" style={{ width: '100%', height: '100%' }}>
                        <AssetImage
                          type="legend"
                          item={item.rawItem}
                          style={{ width: '100%', height: '100%' }}
                          fallbackText={item.title}
                        />
                      </div>

                      {isSelected && (
                        <span className="product-badge" style={{ position: 'absolute', top: '8px', right: '8px', background: 'rgba(255, 255, 255, 0.16)', color: '#fff', border: '1px solid rgba(255, 255, 255, 0.45)', fontWeight: 800, fontSize: '13px', padding: '2px 7px', borderRadius: '4px' }}>
                          ✓
                        </span>
                      )}

                      {!isSelected && item.badge && (
                        <span className="product-badge" style={{ position: 'absolute', top: '8px', right: '8px' }}>
                          {item.badge}
                        </span>
                      )}
                    </div>

                    <div className="card-picker-item-content">
                      <div className="card-picker-item-title" title={item.title}>
                        {item.title}
                      </div>
                      {item.subtitle && (
                        <div className="card-picker-item-subtitle" title={item.subtitle}>
                          {item.subtitle}
                        </div>
                      )}
                    </div>
                  </div>
                );
              }

              // 2. Color Schemes: In-game style segmented color emblem
              if (type === 'color') {
                return (
                  <div
                    key={String(item.id)}
                    className={`card-picker-item ${isSelected ? 'active' : ''}`}
                    onClick={() => {
                      if (onSelect) onSelect(item.id, item);
                      onClose();
                    }}
                    title={item.title}
                  >
                    <div className="card-picker-image-wrap">
                      <ColorEmblem colorData={item.rawItem} size={86} />

                      {isSelected && (
                        <span className="product-badge" style={{ position: 'absolute', top: '8px', right: '8px', background: 'rgba(255, 255, 255, 0.16)', color: '#fff', border: '1px solid rgba(255, 255, 255, 0.45)', fontWeight: 800, fontSize: '13px', padding: '2px 7px', borderRadius: '4px' }}>
                          ✓
                        </span>
                      )}

                      {!isSelected && item.badge && (
                        <span className="product-badge" style={{ position: 'absolute', top: '8px', right: '8px' }}>
                          {item.badge}
                        </span>
                      )}
                    </div>

                    <div className="card-picker-item-content">
                      <div className="card-picker-item-title" title={item.title}>
                        {item.title}
                      </div>
                      {item.subtitle && (
                        <div className="card-picker-item-subtitle" title={item.subtitle}>
                          {item.subtitle}
                        </div>
                      )}
                    </div>
                  </div>
                );
              }

              // 3. KO Effects: In-game vector effect previews
              if (type === 'koEffect' || type === 'ko') {
                return (
                  <div
                    key={String(item.id)}
                    className={`card-picker-item ${isSelected ? 'active' : ''}`}
                    onClick={() => {
                      if (onSelect) onSelect(item.id, item);
                      onClose();
                    }}
                    title={item.title}
                  >
                    <div className="card-picker-image-wrap">
                      <div className="card-image" style={{ width: '100%', height: '100%' }}>
                        <AssetImage
                          type="koEffect"
                          item={item.rawItem}
                          style={{ width: '100%', height: '100%' }}
                          fallbackText={item.title}
                        />
                      </div>

                      {isSelected && (
                        <span className="product-badge" style={{ position: 'absolute', top: '8px', right: '8px', background: 'rgba(255, 255, 255, 0.16)', color: '#fff', border: '1px solid rgba(255, 255, 255, 0.45)', fontWeight: 800, fontSize: '13px', padding: '2px 7px', borderRadius: '4px' }}>
                          ✓
                        </span>
                      )}

                      {!isSelected && item.badge && (
                        <span className="product-badge" style={{ position: 'absolute', top: '8px', right: '8px' }}>
                          {item.badge}
                        </span>
                      )}
                    </div>

                    <div className="card-picker-item-content">
                      <div className="card-picker-item-title" title={item.title}>
                        {item.title}
                      </div>
                      {item.subtitle && (
                        <div className="card-picker-item-subtitle" title={item.subtitle}>
                          {item.subtitle}
                        </div>
                      )}
                    </div>
                  </div>
                );
              }

              // 3. Costumes & Weapons: Fixed-size visual preview cards
              return (
                <div
                  key={String(item.id)}
                  className={`card-picker-item ${isSelected ? 'active' : ''}`}
                  onClick={() => {
                    if (onSelect) onSelect(item.id, item);
                    onClose();
                  }}
                  title={item.title}
                >
                  <div className="card-picker-image-wrap">
                    <div className={`card-image ${type === 'weapon' ? `weapon-preview ${weaponClass}` : ''}`} style={{ width: '100%', height: '100%' }}>
                      {item.isNone ? (
                        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', gap: '8px', color: 'var(--text-muted)' }}>
                          <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" strokeWidth="1.8">
                            <circle cx="12" cy="12" r="10" />
                            <path d="M4.93 4.93l14.14 14.14" />
                          </svg>
                          <span style={{ fontSize: '11px', fontWeight: 700 }}>
                            {isArabic ? 'بدون تعديل (أصلي)' : 'Stock / Unchanged'}
                          </span>
                        </div>
                      ) : (
                        <AssetImage
                          type={type === 'weapon' ? 'weapon' : 'costume'}
                          item={item.rawItem}
                          colorScheme={colorScheme}
                          ownerHero={item.rawItem?.ownerHero || (type === 'weapon' ? '' : ownerHero)}
                          costumeName={type === 'weapon' ? '' : costumeName}
                          style={{ width: '100%', height: '100%' }}
                          fallbackText={item.title}
                        />
                      )}
                    </div>

                    {isSelected && (
                      <span className="product-badge" style={{ position: 'absolute', top: '8px', right: '8px', background: 'rgba(255, 255, 255, 0.16)', color: '#fff', border: '1px solid rgba(255, 255, 255, 0.45)', fontWeight: 800, fontSize: '13px', padding: '2px 7px', borderRadius: '4px' }}>
                        ✓
                      </span>
                    )}

                    {!isSelected && item.badge && (
                      <span className="product-badge" style={{ position: 'absolute', top: '8px', right: '8px' }}>
                        {item.badge}
                      </span>
                    )}
                  </div>

                  <div className="card-picker-item-content">
                    <div className="card-picker-item-title" title={item.title}>
                      {item.title}
                    </div>
                    {item.subtitle && (
                      <div className="card-picker-item-subtitle" title={item.subtitle}>
                        {item.subtitle}
                      </div>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="card-picker-footer">
          <div style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
            {isArabic
              ? `إجمالي المعروض: ${filteredItems.length}`
              : `Total displayed: ${filteredItems.length}`}
          </div>
          <button
            type="button"
            className="button-secondary"
            onClick={onClose}
            style={{ padding: '6px 16px', fontSize: '12px' }}
          >
            {isArabic ? 'إغلاق' : 'Close'}
          </button>
        </div>
      </div>
    </div>
  );
}
