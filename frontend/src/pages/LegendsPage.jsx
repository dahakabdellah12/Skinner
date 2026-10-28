import React, { useState, useMemo } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import AssetImage from '../components/AssetImage';
import SearchBox from '../components/SearchBox';
import CustomDropdown from '../components/CustomDropdown';

const normalizeWeapon = (w) => {
  if (!w) return '';
  const s = String(w).toLowerCase().replace(/[\s_]+/g, '');
  if (s === 'gauntlet' || s === 'gauntlets') return 'fists';
  if (s === 'battleboots' || s === 'boot') return 'boots';
  if (s === 'katars') return 'katar';
  if (s === 'pistol' || s === 'blaster') return 'blasters';
  if (s === 'lance') return 'rocketlance';
  return s;
};

export default function LegendsPage({
  catalog,
  systemInfo,
  onSelectLegend,
  onSelectGamePath,
  search: externalSearch,
  setSearch: externalSetSearch,
  weaponFilter: externalWeaponFilter,
  setWeaponFilter: externalSetWeaponFilter
}) {
  const { tWeapon, isRTL } = useLanguage();
  const [isSelectingPath, setIsSelectingPath] = useState(false);
  const [customPathInput, setCustomPathInput] = useState('');

  const [localSearch, setLocalSearch] = useState('');
  const [localWeaponFilter, setLocalWeaponFilter] = useState('All');

  const search = externalSearch !== undefined ? externalSearch : localSearch;
  const setSearch = externalSetSearch || setLocalSearch;

  const weaponFilter = externalWeaponFilter !== undefined ? externalWeaponFilter : localWeaponFilter;
  const setWeaponFilter = externalSetWeaponFilter || setLocalWeaponFilter;

  const weaponFilterOptions = useMemo(() => {
    const rawTypes = new Set(catalog?.weaponTypes || []);
    if (catalog?.legends) {
      for (const leg of catalog.legends) {
        if (leg.weapon1 && leg.weapon1 !== '--') rawTypes.add(leg.weapon1);
        if (leg.weapon2 && leg.weapon2 !== '--') rawTypes.add(leg.weapon2);
      }
    }

    const uniqueMap = new Map();
    for (const wt of rawTypes) {
      const norm = normalizeWeapon(wt);
      if (!uniqueMap.has(norm)) {
        uniqueMap.set(norm, wt);
      }
    }

    const firstOpt = { value: 'All', label: isRTL ? 'جميع الأسلحة (الكل)' : 'All Weapons' };
    const rest = Array.from(uniqueMap.values())
      .map((wt) => ({
        value: wt,
        label: tWeapon(wt)
      }))
      .sort((a, b) => a.label.localeCompare(b.label, isRTL ? 'ar' : 'en'));

    return [firstOpt, ...rest];
  }, [catalog, isRTL, tWeapon]);

  const filteredLegends = useMemo(() => {
    if (!catalog?.legends) return [];
    const normFilter = normalizeWeapon(weaponFilter);

    return catalog.legends.filter((legend) => {
      if (legend.heroName === 'DEFAULT_CHARACTER' || legend.heroName === 'Random') {
        return false;
      }

      const q = search.trim().toLowerCase();
      const matchSearch =
        !q ||
        legend.displayName?.toLowerCase().includes(q) ||
        legend.heroName?.toLowerCase().includes(q) ||
        legend.weapon1?.toLowerCase().includes(q) ||
        legend.weapon2?.toLowerCase().includes(q);

      const matchWeapon =
        normFilter === 'all' ||
        normalizeWeapon(legend.weapon1) === normFilter ||
        normalizeWeapon(legend.weapon2) === normFilter;

      return matchSearch && matchWeapon;
    });
  }, [catalog, search, weaponFilter]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      {/* Top Filter and Search Bar */}
      <div style={{ display: 'flex', gap: '14px', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flex: 1, minWidth: '280px' }}>
          <SearchBox
            value={search}
            onChange={setSearch}
            placeholder={isRTL ? 'بحث سريع عن أسطورة (Ctrl + K)...' : 'Quick search legends (Ctrl + K)...'}
          />

          <div style={{ width: '200px' }}>
            <CustomDropdown
              options={weaponFilterOptions}
              value={weaponFilter}
              onChange={setWeaponFilter}
              placeholder={isRTL ? 'نوع السلاح' : 'Weapon Type'}
            />
          </div>
        </div>

        <div style={{ fontSize: '13px', color: 'var(--text-muted)' }}>
          {isRTL
            ? `${filteredLegends.length} من أصل ${catalog?.legends?.length || 0} أسطورة`
            : `${filteredLegends.length} of ${catalog?.legends?.length || 0} Legends`}
        </div>
      </div>

      {/* Roster Grid */}
      <div className="legends-grid">
        {filteredLegends.map((legend, index) => {
          const costumeCount = legend.costumes?.length || 0;
          const defaultCostume = legend.costumes?.find((c) => c.isDefault) || legend.costumes?.[0];
          return (
            <div
              key={legend.heroId}
              className="store-card"
              style={{ cursor: 'pointer' }}
              onClick={() => onSelectLegend(legend)}
            >
              <div className="card-image-wrap">
                <div className="card-image">
                  <AssetImage
                    type="legend"
                    item={legend}
                    style={{ width: '100%', height: '100%' }}
                    fallbackText={legend.displayName}
                  />
                </div>
              </div>

              <div className="card-content">
                <div className="card-heading">
                  <h3 className="product-title" title={legend.displayName}>
                    {legend.displayName}
                  </h3>
                  <span className="product-price">
                    #{index + 1}
                  </span>
                </div>

                <p className="product-description">
                  {isRTL
                    ? `${costumeCount} مظهر وزي متاح • ${tWeapon(legend.weapon1)} و ${tWeapon(legend.weapon2)}`
                    : `${costumeCount} costumes available • ${legend.weapon1} & ${legend.weapon2}`}
                </p>

                <div className="card-divider" />

                <div className="card-footer">
                  <button
                    type="button"
                    className="card-button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onSelectLegend(legend);
                    }}
                  >
                    <span>{isRTL ? 'تخصيص الأسطورة' : 'Customize Legend'}</span>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M7 17L17 7M17 7H7M17 7V17" />
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {filteredLegends.length === 0 && (
        <div style={{
          textAlign: 'center',
          padding: '50px 20px',
          background: 'rgba(255, 255, 255, 0.02)',
          borderRadius: '16px',
          border: '1px solid var(--border-secondary)',
          margin: '20px 0',
          maxWidth: '680px',
          alignSelf: 'center',
          width: '100%'
        }}>
          <div style={{
            width: '64px',
            height: '64px',
            borderRadius: '50%',
            background: 'rgba(239, 68, 68, 0.1)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            margin: '0 auto 16px',
            color: 'var(--accent, #6366f1)'
          }}>
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
              <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
              <line x1="12" y1="22.08" x2="12" y2="12" />
            </svg>
          </div>

          <h3 style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '8px' }}>
            {(!catalog?.legends || catalog.legends.length === 0)
              ? (isRTL ? 'لم يتم العثور على ملفات لعبة Brawlhalla' : 'Brawlhalla Game Directory Not Found')
              : (isRTL ? 'لم يتم العثور على أساطير تطابق بحثك' : 'No legends match your search')}
          </h3>

          <p style={{ fontSize: '13px', color: 'var(--text-secondary)', lineHeight: 1.6, marginBottom: '20px', maxWidth: '500px', margin: '0 auto 20px' }}>
            {(!catalog?.legends || catalog.legends.length === 0)
              ? (isRTL
                  ? 'يرجى تحديد مسار مجلد اللعبة أو اختيار ملف تشغيل اللعبة Brawlhalla.exe مباشرة لتحميل كافة الشخصيات والمظاهر.'
                  : 'Please locate your Brawlhalla installation folder or select Brawlhalla.exe directly to load all legends and skins.')
              : (isRTL ? 'جرب البحث باسم آخر أو اختر سلاحاً مختلفاً أو أعد ضبط مسار اللعبة.' : 'Try searching with a different term, or select your game path manually below.')}
          </p>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', alignItems: 'center' }}>
            <div style={{ display: 'flex', gap: '10px', width: '100%', maxWidth: '480px' }}>
              <input
                type="text"
                placeholder={isRTL ? 'مسار مجلد اللعبة أو ملف Brawlhalla.exe...' : 'Path to Brawlhalla folder or Brawlhalla.exe...'}
                value={customPathInput}
                onChange={(e) => setCustomPathInput(e.target.value)}
                style={{
                  flex: 1,
                  padding: '10px 14px',
                  borderRadius: '8px',
                  background: 'var(--bg-card, #1e1e2d)',
                  border: '1px solid var(--border-primary, #333)',
                  color: 'var(--text-primary, #fff)',
                  fontSize: '13px',
                  outline: 'none'
                }}
              />
              <button
                type="button"
                className="btn btn-secondary"
                disabled={isSelectingPath}
                onClick={async () => {
                  if (onSelectGamePath) {
                    setIsSelectingPath(true);
                    try {
                      await onSelectGamePath('browse');
                    } finally {
                      setIsSelectingPath(false);
                    }
                  }
                }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '10px 16px',
                  borderRadius: '8px',
                  fontWeight: 600,
                  fontSize: '13px',
                  whiteSpace: 'nowrap',
                  cursor: 'pointer'
                }}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                </svg>
                <span>{isRTL ? 'استعراض...' : 'Browse...'}</span>
              </button>
            </div>

            {customPathInput.trim() && (
              <button
                type="button"
                className="btn btn-primary"
                disabled={isSelectingPath}
                onClick={async () => {
                  if (onSelectGamePath && customPathInput.trim()) {
                    setIsSelectingPath(true);
                    try {
                      await onSelectGamePath(customPathInput.trim());
                    } finally {
                      setIsSelectingPath(false);
                    }
                  }
                }}
                style={{
                  padding: '10px 24px',
                  borderRadius: '8px',
                  fontWeight: 700,
                  fontSize: '13px',
                  cursor: 'pointer'
                }}
              >
                <span>{isRTL ? 'تأكيد المسار وتحميل الأساطير' : 'Confirm Path & Load Legends'}</span>
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
