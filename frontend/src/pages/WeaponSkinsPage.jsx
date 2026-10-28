import React, { useState, useMemo } from 'react';
import { Search, Sword } from 'lucide-react';
import { useLanguage } from '../i18n/LanguageContext';
import AssetImage from '../components/AssetImage';

export default function WeaponSkinsPage({ catalog, onSelectWeaponForStudio }) {
  const { t, tWeapon } = useLanguage();
  const [search, setSearch] = useState('');
  const [weaponTypeFilter, setWeaponTypeFilter] = useState('All');

  const filteredWeapons = useMemo(() => {
    if (!catalog?.weaponSkins) return [];
    return catalog.weaponSkins.filter((w) => {
      const matchSearch =
        w.displayName?.toLowerCase().includes(search.toLowerCase()) ||
        w.weaponSkinName?.toLowerCase().includes(search.toLowerCase()) ||
        w.ownerHero?.toLowerCase().includes(search.toLowerCase()) ||
        w.baseWeapon?.toLowerCase().includes(search.toLowerCase());

      const matchType =
        weaponTypeFilter === 'All' ||
        w.baseWeapon?.toLowerCase() === weaponTypeFilter.toLowerCase();

      return matchSearch && matchType;
    });
  }, [catalog, search, weaponTypeFilter]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      {/* Category Pills & Search */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        <div style={{ position: 'relative' }}>
          <Search size={16} color="var(--text-muted)" style={{ position: 'absolute', insetInlineStart: '12px', top: '12px' }} />
          <input
            type="text"
            className="input-search"
            placeholder={t('weapons_search_placeholder')}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>

        {/* Category Pills */}
        <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
          <button
            className={`btn ${weaponTypeFilter === 'All' ? 'btn-primary' : 'btn-secondary'}`}
            style={{ padding: '6px 12px', fontSize: '12px' }}
            onClick={() => setWeaponTypeFilter('All')}
          >
            {t('all_types')}
          </button>
          {catalog?.weaponTypes?.map((wt) => (
            <button
              key={wt}
              className={`btn ${weaponTypeFilter === wt ? 'btn-primary' : 'btn-secondary'}`}
              style={{ padding: '6px 12px', fontSize: '12px' }}
              onClick={() => setWeaponTypeFilter(wt)}
            >
              {tWeapon(wt)}
            </button>
          ))}
        </div>
      </div>

      {/* Weapons Grid */}
      <div className="grid-cards">
        {filteredWeapons.map((weapon) => (
          <div key={`${weapon.weaponSkinId}_${weapon.weaponSkinName}`} className="card card-store" style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
            <div>
              {/* Authentic Weapon Vector Preview */}
              <div style={{
                width: '100%',
                height: '110px',
                marginBottom: '10px',
                background: 'radial-gradient(circle at center, rgba(6, 182, 212, 0.18) 0%, rgba(15, 23, 42, 0.6) 80%)',
                borderRadius: 'var(--radius-sm)',
                border: '1px solid var(--border-subtle)',
                overflow: 'hidden',
                position: 'relative',
                boxShadow: 'inset 0 0 15px rgba(0,0,0,0.5)'
              }}>
                <AssetImage
                  type="weapon"
                  item={weapon}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Sword}
                  fallbackText={weapon.displayName}
                />
              </div>

              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <h3 style={{ fontSize: '14px', fontWeight: 'bold' }}>{weapon.displayName}</h3>
              </div>

              <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '4px' }}>
                {t('origin_label')} {weapon.ownerHero || t('universal_label')}
              </div>

              <div style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'monospace', marginTop: '4px' }}>
                {weapon.swfFile}
              </div>
            </div>

            <div style={{ display: 'flex', gap: '6px', marginTop: '14px' }}>
              <button
                className="btn btn-secondary"
                style={{ flex: 1, fontSize: '11px', padding: '6px 8px' }}
                onClick={() => onSelectWeaponForStudio(weapon, 1)}
              >
                {t('btn_equip_w1')}
              </button>
              <button
                className="btn btn-secondary"
                style={{ flex: 1, fontSize: '11px', padding: '6px 8px' }}
                onClick={() => onSelectWeaponForStudio(weapon, 2)}
              >
                {t('btn_equip_w2')}
              </button>
            </div>
          </div>
        ))}
      </div>

      {filteredWeapons.length === 0 && (
        <div style={{ textAlign: 'center', padding: '60px 0', color: 'var(--text-muted)' }}>
          {t('no_weapons_match')}
        </div>
      )}
    </div>
  );
}
