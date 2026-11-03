import React, { useState, useEffect, useMemo, useRef } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import AssetImage from './AssetImage';

function uintToHex(val, fallback = '#6b7280') {
  if (val == null || val === undefined) return fallback;
  const num = Number(val);
  if (isNaN(num) || num === 0) return fallback;
  return '#' + num.toString(16).padStart(6, '0');
}

export function ColorEmblem({ colorData, size = 68 }) {
  if (!colorData || colorData.isNone) {
    return (
      <svg viewBox="0 0 100 100" width={size} height={size} style={{ filter: 'drop-shadow(0 4px 10px rgba(0,0,0,0.5))' }}>
        <circle cx="50" cy="50" r="45" fill="#14161b" stroke="#333a48" strokeWidth="3" />
        <path d="M50 50 L18.18 18.18 A45 45 0 0 1 81.82 18.18 Z" fill="#ef4444" />
        <path d="M50 50 L81.82 18.18 A45 45 0 0 1 81.82 81.82 Z" fill="#3b82f6" />
        <path d="M50 50 L81.82 81.82 A45 45 0 0 1 18.18 81.82 Z" fill="#eab308" />
        <path d="M50 50 L18.18 81.82 A45 45 0 0 1 18.18 18.18 Z" fill="#10b981" />
        <circle cx="50" cy="50" r="14" fill="#22c55e" stroke="#14161b" strokeWidth="3" />
        <circle cx="50" cy="50" r="45" fill="none" stroke="rgba(255,255,255,0.3)" strokeWidth="2" />
      </svg>
    );
  }

  const ind = uintToHex(colorData.indicatorColor, '#3b82f6');
  const sw = colorData.swapColors || {};
  const b1 = uintToHex(sw.body1 ?? sw.Body1 ?? sw.hair, ind);
  const b2 = uintToHex(sw.body2 ?? sw.Body2 ?? sw.cloth, b1);
  const sp = uintToHex(sw.special ?? sw.Special, b1);
  const wp = uintToHex(sw.weapon ?? sw.Weapon, b2);

  return (
    <svg viewBox="0 0 100 100" width={size} height={size} style={{ filter: 'drop-shadow(0 4px 12px rgba(0,0,0,0.55))' }}>
      <circle cx="50" cy="50" r="45" fill="#14161b" stroke="#333a48" strokeWidth="3" />
      {/* Top Wedge: Body1 / Hair */}
      <path d="M50 50 L18.18 18.18 A45 45 0 0 1 81.82 18.18 Z" fill={b1} />
      {/* Right Wedge: Body2 / Cloth */}
      <path d="M50 50 L81.82 18.18 A45 45 0 0 1 81.82 81.82 Z" fill={b2} />
      {/* Bottom Wedge: Special */}
      <path d="M50 50 L81.82 81.82 A45 45 0 0 1 18.18 81.82 Z" fill={sp} />
      {/* Left Wedge: Weapon */}
      <path d="M50 50 L18.18 81.82 A45 45 0 0 1 18.18 18.18 Z" fill={wp} />
      {/* Center Core: Indicator Color */}
      <circle cx="50" cy="50" r="14" fill={ind} stroke="#14161b" strokeWidth="3" />
      <circle cx="50" cy="50" r="45" fill="none" stroke="rgba(255,255,255,0.25)" strokeWidth="2" />
    </svg>
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

  // Filter items by search query
  const filteredItems = useMemo(() => {
    // First remove any Random characters or random items completely
    const sanitizedItems = items.filter((it) => {
      const titleLower = (it.title || '').toLowerCase();
      const idLower = String(it.id || '').toLowerCase();
      return titleLower !== 'random' && idLower !== 'random' && idLower !== '__random_roll__';
    });

    const q = search.trim().toLowerCase();
    if (!q) return sanitizedItems;

    const matchesFiltered = sanitizedItems.filter((it) => {
      if (it.isNone) return false;
      const titleMatch = it.title && it.title.toLowerCase().includes(q);
      const subMatch = it.subtitle && it.subtitle.toLowerCase().includes(q);
      const searchKeyMatch = it.searchKey && it.searchKey.toLowerCase().includes(q);
      return titleMatch || subMatch || searchKeyMatch;
    });

    const specialItems = sanitizedItems.filter((it) => it.isNone);
    return [...specialItems, ...matchesFiltered];
  }, [items, search]);

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
                      onSelect(item.id);
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
                      onSelect(item.id);
                      onClose();
                    }}
                    title={item.title}
                  >
                    <div className="card-picker-image-wrap">
                      <ColorEmblem colorData={item.rawItem} size={72} />

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
                    onSelect(item.id);
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
