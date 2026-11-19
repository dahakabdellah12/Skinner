import React, { useState, useMemo, useCallback } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import SearchBox from '../components/SearchBox';
import CustomDropdown from '../components/CustomDropdown';
import AssetImage from '../components/AssetImage';
import KoEffectCustomizer from '../components/KoEffectCustomizer';

export default function KoEffectsPage({
  catalog,
  systemInfo,
  showToast,
  isBusy,
  onApplyModification,
  onRestartGame,
  onLaunchGame,
  onRestoreKoEffects
}) {
  const { isArabic } = useLanguage();

  // Search & Category Filter state
  const [search, setSearch] = useState('');
  const [categoryFilter, setCategoryFilter] = useState('All');

  // Selected KO Effect for entering the studio
  const [activeKoEffect, setActiveKoEffect] = useState(null);

  const allKoEffects = useMemo(() => {
    return catalog?.koEffects || [];
  }, [catalog?.koEffects]);

  // Category filter options
  const categoryOptions = useMemo(() => {
    const rawCategories = new Set();
    for (const k of allKoEffects) {
      if (k.category) {
        rawCategories.add(k.category);
      }
    }

    const firstOpt = { value: 'All', label: isArabic ? 'جميع التأثيرات (الكل)' : 'All KO Effects' };
    const rest = Array.from(rawCategories).map((cat) => {
      let label = cat;
      if (isArabic) {
        if (cat === 'Default') label = 'الافتراضي';
        else if (cat === 'Crossover') label = 'كروس أوفر';
        else if (cat === 'Battle Pass') label = 'باتل باس';
        else if (cat === 'Seasonal') label = 'مواسم وفعاليات';
        else if (cat === 'Store') label = 'المتجر والتصاميم';
      }
      return { value: cat, label };
    });

    return [firstOpt, ...rest];
  }, [allKoEffects, isArabic]);

  // Filtered KO Effects for the Roster Gallery
  const filteredKoEffects = useMemo(() => {
    const q = search ? search.trim().toLowerCase() : '';
    const isAll = categoryFilter === 'All' || !categoryFilter;
    const normFilter = categoryFilter.toLowerCase();

    return allKoEffects.filter((k) => {
      if (!isAll && (k.category || '').toLowerCase() !== normFilter) {
        return false;
      }
      if (!q) return true;
      const corpus = `${k.displayName || ''} ${k.koEffectName || ''} ${k.animClass || ''} ${k.category || ''}`.toLowerCase();
      return corpus.includes(q);
    });
  }, [allKoEffects, search, categoryFilter]);

  // Handle entering customizer
  const handleSelectEffect = useCallback((ko) => {
    setActiveKoEffect(ko);
  }, []);

  // Handle returning to gallery
  const handleBackToGallery = useCallback(() => {
    setActiveKoEffect(null);
  }, []);

  // 1. DEDICATED STUDIO VIEW (Independent per-effect customizer file)
  if (activeKoEffect) {
    return (
      <KoEffectCustomizer
        koEffect={activeKoEffect}
        catalog={catalog}
        systemInfo={systemInfo}
        onApplyModification={onApplyModification}
        onBack={handleBackToGallery}
        onRestartGame={onRestartGame}
        onLaunchGame={onLaunchGame}
        showToast={showToast}
        isBusy={isBusy}
      />
    );
  }

  // 2. ROSTER GALLERY VIEW (Identical layout to LegendsPage)
  return (
    <div className="legends-page" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      {/* Top Filter and Search Bar */}
      <div className="roster-header-glass" style={{ display: 'flex', gap: '14px', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flex: 1, minWidth: '280px' }}>
          <SearchBox
            value={search}
            onChange={setSearch}
            placeholder={isArabic ? 'بحث سريع عن تأثير إطاحة (Ctrl + K)...' : 'Quick search KO effects (Ctrl + K)...'}
          />

          <div style={{ width: '200px' }}>
            <CustomDropdown
              options={categoryOptions}
              value={categoryFilter}
              onChange={setCategoryFilter}
              placeholder={isArabic ? 'التصنيف' : 'Category'}
              showSearch={false}
            />
          </div>
        </div>

        <div style={{ fontSize: '13px', color: 'var(--text-muted)' }}>
          {isArabic
            ? `${filteredKoEffects.length} تأثير إطاحة`
            : `${filteredKoEffects.length} KO Effects`}
        </div>
      </div>

      {/* Roster Grid */}
      <div className="legends-grid">
        {filteredKoEffects.map((ko) => (
          <div
            key={ko.koEffectId || ko.koEffectName}
            className="store-card"
            style={{ cursor: 'pointer' }}
            onClick={() => handleSelectEffect(ko)}
          >
            <div className="card-image-wrap">
              <div className="card-image">
                <AssetImage
                  type="koEffect"
                  item={ko}
                  style={{ width: '100%', height: '100%' }}
                  fallbackText={ko.displayName}
                />
              </div>
            </div>

            <div className="card-content">
              <div className="card-heading">
                <h3 className="product-title" title={ko.displayName}>
                  {ko.displayName}
                </h3>
              </div>

              <p className="product-description">
                {ko.category} • {ko.swfFile}
              </p>

              <div className="card-divider" />

              <div className="card-footer">
                <button
                  type="button"
                  className="card-button"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleSelectEffect(ko);
                  }}
                >
                  <span>{isArabic ? 'تخصيص التأثير' : 'Customize Effect'}</span>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M7 17L17 7M17 7H7M17 7V17" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>

      {filteredKoEffects.length === 0 && (
        <div
          style={{
            textAlign: 'center',
            padding: '50px 20px',
            background: 'rgba(255, 255, 255, 0.02)',
            borderRadius: '16px',
            border: '1px solid var(--border-secondary)',
            margin: '20px 0',
            maxWidth: '680px',
            alignSelf: 'center',
            width: '100%'
          }}
        >
          <h3 style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '8px' }}>
            {isArabic ? 'لم يتم العثور على تأثيرات إطاحة تطابق بحثك' : 'No KO effects match your search'}
          </h3>
          <p style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
            {isArabic ? 'جرب البحث باسم آخر أو اختر تصنيفاً مختلفاً.' : 'Try searching with a different name or category.'}
          </p>
        </div>
      )}
    </div>
  );
}
