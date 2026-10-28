import React, { useState, useMemo } from 'react';
import { Search, Sparkles } from 'lucide-react';
import { useLanguage } from '../i18n/LanguageContext';
import AssetImage from '../components/AssetImage';

export default function CostumesPage({ catalog, onSelectCostumeForStudio }) {
  const { t } = useLanguage();
  const [search, setSearch] = useState('');
  const [legendFilter, setLegendFilter] = useState('All');

  const filteredCostumes = useMemo(() => {
    if (!catalog?.costumes) return [];
    return catalog.costumes.filter((costume) => {
      const matchSearch =
        costume.displayName?.toLowerCase().includes(search.toLowerCase()) ||
        costume.costumeName?.toLowerCase().includes(search.toLowerCase()) ||
        costume.ownerHero?.toLowerCase().includes(search.toLowerCase());

      const matchLegend =
        legendFilter === 'All' ||
        costume.ownerHero?.toLowerCase() === legendFilter.toLowerCase();

      return matchSearch && matchLegend;
    });
  }, [catalog, search, legendFilter]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      {/* Search & Filter Bar */}
      <div style={{ display: 'flex', gap: '14px', alignItems: 'center' }}>
        <div style={{ position: 'relative', flex: 1 }}>
          <Search size={16} color="var(--text-muted)" style={{ position: 'absolute', insetInlineStart: '12px', top: '12px' }} />
          <input
            type="text"
            className="input-search"
            placeholder={t('costumes_search_placeholder')}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>

        <select
          className="select-dropdown"
          value={legendFilter}
          onChange={(e) => setLegendFilter(e.target.value)}
          style={{ minWidth: '180px' }}
        >
          <option value="All">{t('all_legends')}</option>
          {catalog?.legends?.map((l) => (
            <option key={l.heroName} value={l.heroName}>{l.displayName} ({l.heroName})</option>
          ))}
        </select>
      </div>

      {/* Costumes Grid */}
      <div className="grid-cards">
        {filteredCostumes.map((costume) => (
          <div key={`${costume.ownerHero}_${costume.costumeId}_${costume.costumeName}`} className="card card-store" style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
            <div>
              {/* Authentic Skin Visual from Game SWF */}
              <div style={{
                width: '100%',
                height: '140px',
                marginBottom: '12px',
                background: 'radial-gradient(circle at center, rgba(16, 185, 129, 0.18) 0%, rgba(15, 23, 42, 0.6) 80%)',
                borderRadius: 'var(--radius-sm)',
                border: '1px solid var(--border-subtle)',
                overflow: 'hidden',
                position: 'relative',
                boxShadow: 'inset 0 0 20px rgba(0,0,0,0.5)'
              }}>
                <AssetImage
                  type="costume"
                  item={costume}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Sparkles}
                  fallbackText={costume.displayName}
                />
                <span className="badge badge-cyan" style={{ position: 'absolute', top: '8px', insetInlineEnd: '8px' }}>
                  {costume.ownerHero}
                </span>
              </div>

              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <h3 style={{ fontSize: '15px', fontWeight: 'bold' }}>{costume.displayName}</h3>
              </div>

              {/* Status Badges */}
              <div style={{ display: 'flex', gap: '6px', margin: '8px 0' }}>
                {costume.isDefault && <span className="badge badge-success">{t('badge_default')}</span>}
                {costume.isCrossover && <span className="badge badge-danger">{t('badge_crossover')}</span>}
                {costume.isMetadev && <span className="badge badge-warning">{t('badge_metadev')}</span>}
              </div>

              <div style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'monospace' }}>
                {costume.swfFile}
              </div>
            </div>

            <button
              className="btn btn-secondary"
              style={{ marginTop: '14px', width: '100%', fontSize: '12px', padding: '8px' }}
              onClick={() => onSelectCostumeForStudio(costume)}
            >
              <Sparkles size={14} />
              <span>{t('btn_select_for_studio')}</span>
            </button>
          </div>
        ))}
      </div>

      {filteredCostumes.length === 0 && (
        <div style={{ textAlign: 'center', padding: '60px 0', color: 'var(--text-muted)' }}>
          {t('no_costumes_match')}
        </div>
      )}
    </div>
  );
}
