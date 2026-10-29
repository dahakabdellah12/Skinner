import React, { useState, useEffect, useMemo } from 'react';
import {
  Users,
  AlertTriangle,
  Play,
  RotateCcw,
  CheckCircle2,
  Layers,
  Sparkles,
  Sword,
  Palette,
  RefreshCw,
  ShoppingBag
} from 'lucide-react';
import api from '../services/api';
import { useLanguage } from '../i18n/LanguageContext';
import AssetImage from '../components/AssetImage';

export default function StudioPage({
  catalog,
  systemInfo,
  selectedLegend,
  setSelectedLegend,
  selectedTargetCostume,
  setSelectedTargetCostume,
  selectedReplacementCostume,
  setSelectedReplacementCostume,
  selectedTargetWeapon1,
  setSelectedTargetWeapon1,
  selectedReplacementWeapon1,
  setSelectedReplacementWeapon1,
  selectedTargetWeapon2,
  setSelectedTargetWeapon2,
  selectedReplacementWeapon2,
  setSelectedReplacementWeapon2,
  selectedColorScheme,
  setSelectedColorScheme,
  onApplyModification,
  onRestoreTargetCostume,
  onLaunchGame,
  onRestartGame,
  onSelectLegendForStudio,
  isBusy
}) {
  const { t, tWeapon } = useLanguage();
  const [compatibility, setCompatibility] = useState(null);
  const [isValidating, setIsValidating] = useState(false);
  const [previewSvg, setPreviewSvg] = useState('');
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);
  const [autoRestart, setAutoRestart] = useState(true);

  // Available costumes for currently selected host legend (fallback to catalog if legend.costumes is not populated)
  const hostCostumes = useMemo(() => {
    if (!selectedLegend) return [];
    if (selectedLegend.costumes && selectedLegend.costumes.length > 0) {
      return selectedLegend.costumes;
    }
    return catalog?.costumes?.filter((c) => c.ownerHero === selectedLegend.heroName) || [];
  }, [selectedLegend, catalog]);

  // Group costumes for the replacement dropdown
  const sameLegendCostumes = useMemo(() => {
    if (!selectedLegend) return [];
    return hostCostumes;
  }, [selectedLegend, hostCostumes]);

  const crossLegendCostumes = useMemo(() => {
    if (!selectedLegend || !catalog?.costumes) return [];
    return catalog.costumes.filter((c) => c.ownerHero !== selectedLegend.heroName);
  }, [selectedLegend, catalog]);

  // Compatible weapon skins matching Legend Weapon 1 and Weapon 2 categories
  const compatibleWeapon1Skins = useMemo(() => {
    if (!catalog?.weaponSkins || !selectedLegend?.weapon1) return [];
    return catalog.weaponSkins.filter(
      (w) => w.baseWeapon?.toLowerCase() === selectedLegend.weapon1.toLowerCase()
    );
  }, [catalog, selectedLegend]);

  const compatibleWeapon2Skins = useMemo(() => {
    if (!catalog?.weaponSkins || !selectedLegend?.weapon2) return [];
    return catalog.weaponSkins.filter(
      (w) => w.baseWeapon?.toLowerCase() === selectedLegend.weapon2.toLowerCase()
    );
  }, [catalog, selectedLegend]);

  // Handle Legend Change from dropdown
  const handleLegendChange = (heroName) => {
    const leg = catalog?.legends?.find((l) => l.heroName === heroName);
    if (!leg) return;

    if (onSelectLegendForStudio) {
      onSelectLegendForStudio(leg);
    } else {
      setSelectedLegend(leg);
      const costumes = (leg.costumes && leg.costumes.length > 0)
        ? leg.costumes
        : catalog?.costumes?.filter((c) => c.ownerHero === leg.heroName) || [];
      const defCostume = costumes.find((c) => c.isDefault) || costumes[0] || null;
      setSelectedTargetCostume(defCostume);
      setSelectedReplacementCostume(costumes.find((c) => !c.isDefault) || null);

      if (catalog?.weaponSkins) {
        const w1 = catalog.weaponSkins.find((w) => w.ownerHero === leg.heroName && w.baseWeapon === leg.weapon1);
        const w2 = catalog.weaponSkins.find((w) => w.ownerHero === leg.heroName && w.baseWeapon === leg.weapon2);
        setSelectedTargetWeapon1(w1 || null);
        setSelectedTargetWeapon2(w2 || null);
        setSelectedReplacementWeapon1(null);
        setSelectedReplacementWeapon2(null);
      }
    }
  };

  // Re-run compatibility engine validation when selections change
  useEffect(() => {
    async function validate() {
      if (!selectedLegend || !selectedTargetCostume) {
        setCompatibility(null);
        return;
      }

      setIsValidating(true);
      try {
        const req = {
          targetLegend: selectedLegend,
          targetCostume: selectedTargetCostume,
          replacementCostume: selectedReplacementCostume,
          targetWeapon1: selectedTargetWeapon1,
          replacementWeapon1: selectedReplacementWeapon1,
          targetWeapon2: selectedTargetWeapon2,
          replacementWeapon2: selectedReplacementWeapon2,
          selectedColorScheme: selectedColorScheme
        };
        const report = await api.validateModification(req);
        setCompatibility(report);
      } catch (err) {
        console.error('Validation error:', err);
        setCompatibility({
          isCompatible: false,
          level: 'Incompatible',
          diagnostics: [`Validation error: ${err.message}`]
        });
      } finally {
        setIsValidating(false);
      }
    }

    validate();
  }, [
    selectedLegend,
    selectedTargetCostume,
    selectedReplacementCostume,
    selectedTargetWeapon1,
    selectedReplacementWeapon1,
    selectedTargetWeapon2,
    selectedReplacementWeapon2,
    selectedColorScheme
  ]);

  // Load vector preview for the replacement weapon or target weapon
  useEffect(() => {
    let isCancelled = false;

    async function loadPreview() {
      const weaponToPreview = selectedReplacementWeapon1 || selectedTargetWeapon1 || selectedReplacementWeapon2 || selectedTargetWeapon2;
      if (!weaponToPreview || !weaponToPreview.swfFile || !weaponToPreview.symbolSuffix) {
        setPreviewSvg('');
        return;
      }

      setIsLoadingPreview(true);
      try {
        let symbol = weaponToPreview.symbolSuffix;
        if (!symbol.startsWith('a_Weapon')) {
          symbol = `a_Weapon${weaponToPreview.baseWeapon}_${symbol}`;
        }
        const svg = await api.renderPreview(weaponToPreview.swfFile, symbol);
        if (!isCancelled) {
          setPreviewSvg(svg || '');
        }
      } catch (err) {
        console.warn('Preview render failed:', err);
        if (!isCancelled) {
          setPreviewSvg('');
        }
      } finally {
        if (!isCancelled) {
          setIsLoadingPreview(false);
        }
      }
    }

    loadPreview();
    return () => {
      isCancelled = true;
    };
  }, [selectedReplacementWeapon1, selectedTargetWeapon1, selectedReplacementWeapon2, selectedTargetWeapon2]);

  // If no legend is selected or catalog is not ready
  if (!catalog || !catalog.legends || catalog.legends.length === 0) {
    return (
      <div style={{ textAlign: 'center', padding: '60px 0', color: 'var(--text-muted)' }}>
        <div className="spinner" style={{ margin: '0 auto 16px' }} />
        <p>{t('loading_engine')}</p>
      </div>
    );
  }

  const isLevelIncompatible = compatibility?.isCompatible === false || compatibility?.level === 'Incompatible';
  const isLevelExperimental = compatibility?.level === 'CrossLegendExperimental';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: '16px' }}>
      {/* Live Game Notification Banner */}
      {systemInfo?.isGameRunning && (
        <div style={{
          padding: '10px 16px',
          backgroundColor: 'rgba(16, 185, 129, 0.1)',
          border: '1px solid rgba(16, 185, 129, 0.35)',
          borderRadius: 'var(--radius-sm)',
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          fontSize: '12px',
          color: '#34D399',
          flexShrink: 0
        }}>
          <span style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: '#10B981', display: 'inline-block', flexShrink: 0 }} />
          <div>
            <strong>{t('live_banner_title')}</strong> {t('live_banner_body')}
          </div>
        </div>
      )}

      {/* 3-Pane Center Area */}
      <div style={{ display: 'grid', gridTemplateColumns: '360px 1fr 340px', gap: '16px', flex: 1, minHeight: 0 }}>

        {/* ================= PANE 1: SELECTION CONTROLS ================= */}
        <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '14px', overflowY: 'auto' }}>
          
          {/* 1. Host Legend */}
          <div>
            <h3 style={{ fontSize: '15px', fontWeight: 'bold', color: 'var(--accent-blue)', display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Users size={16} />
              <span>{t('pane1_legend_title')}</span>
            </h3>
            <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '2px 0 8px' }}>
              {t('pane1_legend_desc')}
            </p>
            <select
              className="select-dropdown"
              style={{ width: '100%' }}
              value={selectedLegend?.heroName || ''}
              onChange={(e) => handleLegendChange(e.target.value)}
            >
              {catalog?.legends?.map((l) => (
                <option key={l.heroName} value={l.heroName}>
                  {l.displayName} ({tWeapon(l.weapon1)}, {tWeapon(l.weapon2)})
                </option>
              ))}
            </select>

            {/* Legend Visual Preview Card */}
            {selectedLegend && (
              <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
                marginTop: '8px',
                padding: '8px 10px',
                background: 'var(--bg-deep)',
                borderRadius: 'var(--radius-sm)',
                border: '1px solid var(--border-subtle)'
              }}>
                <div style={{
                  width: '56px',
                  height: '56px',
                  borderRadius: 'var(--radius-sm)',
                  overflow: 'hidden',
                  background: '#0a0e17',
                  flexShrink: 0,
                  border: '1px solid var(--border-subtle)'
                }}>
                  <AssetImage
                    type="legend"
                    item={selectedLegend}
                    style={{ width: '100%', height: '100%' }}
                    fallbackIcon={Users}
                    fallbackText={selectedLegend.displayName}
                  />
                </div>
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div style={{ fontWeight: 'bold', fontSize: '13px', color: 'var(--text-primary)' }}>
                    {selectedLegend.displayName}
                  </div>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                    {selectedLegend.heroName}
                  </div>
                  <div style={{ display: 'flex', gap: '4px', marginTop: '4px' }}>
                    <span className="badge badge-cyan" style={{ fontSize: '10px', padding: '1px 6px' }}>
                      {tWeapon(selectedLegend.weapon1)}
                    </span>
                    <span className="badge badge-violet" style={{ fontSize: '10px', padding: '1px 6px' }}>
                      {tWeapon(selectedLegend.weapon2)}
                    </span>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* 2. Target Costume */}
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: 'bold', display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Layers size={16} />
              <span>{t('pane1_target_costume_title')}</span>
            </h3>
            <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '2px 0 8px' }}>
              {t('pane1_target_costume_desc')}
            </p>
            <select
              className="select-dropdown"
              style={{ width: '100%' }}
              value={selectedTargetCostume?.costumeName || ''}
              onChange={(e) => {
                const cost = hostCostumes.find((c) => c.costumeName === e.target.value);
                if (cost) setSelectedTargetCostume(cost);
              }}
            >
              {hostCostumes.map((c) => (
                <option key={c.costumeName} value={c.costumeName}>
                  {c.displayName} {c.isDefault ? t('opt_default_suffix') : ''}
                </option>
              ))}
            </select>

            {/* Target Costume Visual Preview */}
            {selectedTargetCostume && (
              <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: '10px',
                marginTop: '8px',
                padding: '6px 10px',
                background: 'var(--bg-deep)',
                borderRadius: 'var(--radius-sm)',
                border: '1px solid var(--border-subtle)'
              }}>
                <div style={{
                  width: '46px',
                  height: '46px',
                  borderRadius: 'var(--radius-sm)',
                  overflow: 'hidden',
                  background: '#0a0e17',
                  flexShrink: 0,
                  border: '1px solid var(--border-subtle)'
                }}>
                  <AssetImage
                    type="costume"
                    item={selectedTargetCostume}
                    style={{ width: '100%', height: '100%' }}
                    fallbackIcon={Layers}
                    fallbackText={selectedTargetCostume.displayName}
                  />
                </div>
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div style={{ fontSize: '12px', fontWeight: 'bold', color: 'var(--text-primary)' }}>
                    {selectedTargetCostume.displayName}
                  </div>
                  <div style={{ fontSize: '10px', color: 'var(--accent-cyan)' }}>
                    {selectedTargetCostume.isDefault ? t('badge_default') : (selectedTargetCostume.swfFile || 'Custom SWF')}
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* 3. Replacement Costume */}
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: 'bold', color: 'var(--color-success)', display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Sparkles size={16} />
              <span>{t('pane1_repl_costume_title')}</span>
            </h3>
            <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '2px 0 8px' }}>
              {t('pane1_repl_costume_desc')}
            </p>
            <select
              className="select-dropdown"
              style={{ width: '100%' }}
              value={selectedReplacementCostume?.costumeName || ''}
              onChange={(e) => {
                if (!e.target.value) {
                  setSelectedReplacementCostume(null);
                  return;
                }
                const cost = catalog?.costumes?.find((c) => c.costumeName === e.target.value);
                if (cost) setSelectedReplacementCostume(cost);
              }}
            >
              <option value="">{t('pane1_costume_unchanged')}</option>
              {sameLegendCostumes.length > 0 && (
                <optgroup label={t('pane1_group_same_legend', { legend: selectedLegend?.displayName })}>
                  {sameLegendCostumes.map((c) => (
                    <option key={`same_${c.costumeName}`} value={c.costumeName}>
                      {c.displayName} {c.isDefault ? t('opt_default_suffix') : ''}
                    </option>
                  ))}
                </optgroup>
              )}
              {crossLegendCostumes.length > 0 && (
                <optgroup label={t('pane1_group_cross_legend')}>
                  {crossLegendCostumes.map((c) => (
                    <option key={`cross_${c.ownerHero}_${c.costumeName}`} value={c.costumeName}>
                      {c.displayName} ({c.ownerHero})
                    </option>
                  ))}
                </optgroup>
              )}
            </select>

            {/* Replacement Costume Visual Preview */}
            <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              marginTop: '8px',
              padding: '6px 10px',
              background: 'var(--bg-deep)',
              borderRadius: 'var(--radius-sm)',
              border: '1px solid var(--border-subtle)'
            }}>
              <div style={{
                width: '46px',
                height: '46px',
                borderRadius: 'var(--radius-sm)',
                overflow: 'hidden',
                background: '#0a0e17',
                flexShrink: 0,
                border: '1px solid var(--border-subtle)'
              }}>
                <AssetImage
                  type="costume"
                  item={selectedReplacementCostume || selectedTargetCostume}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Sparkles}
                  fallbackText={selectedReplacementCostume?.displayName || selectedTargetCostume?.displayName}
                />
              </div>
              <div style={{ minWidth: 0, flex: 1 }}>
                <div style={{ fontSize: '12px', fontWeight: 'bold', color: selectedReplacementCostume ? 'var(--color-success)' : 'var(--text-secondary)' }}>
                  {selectedReplacementCostume ? selectedReplacementCostume.displayName : `${selectedTargetCostume?.displayName || ''} (${t('pane1_costume_unchanged')})`}
                </div>
                <div style={{ fontSize: '10px', color: 'var(--accent-violet)' }}>
                  {selectedReplacementCostume ? (selectedReplacementCostume.ownerHero || selectedReplacementCostume.swfFile) : (selectedTargetCostume?.swfFile || '')}
                </div>
              </div>
            </div>
          </div>

          {/* 4. Weapon Cosmetics */}
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: 'bold', color: 'var(--accent-cyan)', display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Sword size={16} />
              <span>{t('pane1_weapons_title')}</span>
            </h3>
            <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '2px 0 8px' }}>
              {t('pane1_weapons_desc')}
            </p>

            {/* Weapon 1 Selection & Visual */}
            <label style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>
              {t('pane1_weapon1_label', { weapon: tWeapon(selectedLegend?.weapon1) })}
            </label>
            <select
              className="select-dropdown"
              style={{ width: '100%', margin: '4px 0 6px' }}
              value={selectedReplacementWeapon1?.weaponSkinName || ''}
              onChange={(e) => {
                if (!e.target.value) {
                  setSelectedReplacementWeapon1(null);
                  return;
                }
                const w = compatibleWeapon1Skins.find((item) => item.weaponSkinName === e.target.value);
                if (w) setSelectedReplacementWeapon1(w);
              }}
            >
              <option value="">{t('pane1_weapon_unchanged', { weapon: tWeapon(selectedLegend?.weapon1) })}</option>
              {compatibleWeapon1Skins.map((w) => (
                <option key={w.weaponSkinName} value={w.weaponSkinName}>
                  {w.displayName} ({w.ownerHero || t('universal_label')})
                </option>
              ))}
            </select>

            <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              marginBottom: '10px',
              padding: '6px 10px',
              background: 'var(--bg-deep)',
              borderRadius: 'var(--radius-sm)',
              border: '1px solid var(--border-subtle)'
            }}>
              <div style={{
                width: '38px',
                height: '38px',
                borderRadius: 'var(--radius-sm)',
                overflow: 'hidden',
                background: '#0a0e17',
                flexShrink: 0,
                border: '1px solid var(--border-subtle)'
              }}>
                <AssetImage
                  type="weapon"
                  item={selectedReplacementWeapon1 || selectedTargetWeapon1}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Sword}
                  fallbackText="W1"
                />
              </div>
              <div style={{ minWidth: 0, flex: 1 }}>
                <div style={{ fontSize: '12px', fontWeight: 'bold', color: selectedReplacementWeapon1 ? 'var(--color-success)' : 'var(--text-primary)' }}>
                  {selectedReplacementWeapon1?.displayName || selectedTargetWeapon1?.displayName || t('pane1_weapon_unchanged', { weapon: tWeapon(selectedLegend?.weapon1) })}
                </div>
                <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>
                  {tWeapon(selectedLegend?.weapon1)} • {selectedReplacementWeapon1?.ownerHero || t('universal_label')}
                </div>
              </div>
            </div>

            {/* Weapon 2 Selection & Visual */}
            <label style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>
              {t('pane1_weapon2_label', { weapon: tWeapon(selectedLegend?.weapon2) })}
            </label>
            <select
              className="select-dropdown"
              style={{ width: '100%', margin: '4px 0 6px' }}
              value={selectedReplacementWeapon2?.weaponSkinName || ''}
              onChange={(e) => {
                if (!e.target.value) {
                  setSelectedReplacementWeapon2(null);
                  return;
                }
                const w = compatibleWeapon2Skins.find((item) => item.weaponSkinName === e.target.value);
                if (w) setSelectedReplacementWeapon2(w);
              }}
            >
              <option value="">{t('pane1_weapon_unchanged', { weapon: tWeapon(selectedLegend?.weapon2) })}</option>
              {compatibleWeapon2Skins.map((w) => (
                <option key={w.weaponSkinName} value={w.weaponSkinName}>
                  {w.displayName} ({w.ownerHero || t('universal_label')})
                </option>
              ))}
            </select>

            <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              padding: '6px 10px',
              background: 'var(--bg-deep)',
              borderRadius: 'var(--radius-sm)',
              border: '1px solid var(--border-subtle)'
            }}>
              <div style={{
                width: '38px',
                height: '38px',
                borderRadius: 'var(--radius-sm)',
                overflow: 'hidden',
                background: '#0a0e17',
                flexShrink: 0,
                border: '1px solid var(--border-subtle)'
              }}>
                <AssetImage
                  type="weapon"
                  item={selectedReplacementWeapon2 || selectedTargetWeapon2}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Sword}
                  fallbackText="W2"
                />
              </div>
              <div style={{ minWidth: 0, flex: 1 }}>
                <div style={{ fontSize: '12px', fontWeight: 'bold', color: selectedReplacementWeapon2 ? 'var(--color-success)' : 'var(--text-primary)' }}>
                  {selectedReplacementWeapon2?.displayName || selectedTargetWeapon2?.displayName || t('pane1_weapon_unchanged', { weapon: tWeapon(selectedLegend?.weapon2) })}
                </div>
                <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>
                  {tWeapon(selectedLegend?.weapon2)} • {selectedReplacementWeapon2?.ownerHero || t('universal_label')}
                </div>
              </div>
            </div>
          </div>

          {/* 5. Color Scheme */}
          <div>
            <h3 style={{ fontSize: '14px', fontWeight: 'bold', color: 'var(--accent-violet)', display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Palette size={16} />
              <span>{t('pane1_color_title')}</span>
            </h3>
            <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '2px 0 8px' }}>
              {t('pane1_color_desc')}
            </p>
            <select
              className="select-dropdown"
              style={{ width: '100%' }}
              value={selectedColorScheme?.schemeName || ''}
              onChange={(e) => {
                const scheme = catalog?.colorSchemes?.find((s) => s.schemeName === e.target.value);
                setSelectedColorScheme(scheme || null);
              }}
            >
              <option value="">{t('pane1_color_default')}</option>
              {catalog?.colorSchemes?.map((s) => (
                <option key={s.schemeId} value={s.schemeName}>
                  {s.displayName} ({s.rarity})
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* ================= PANE 2: STORE SHOWCASE & VISUAL MANIFEST ================= */}
        <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '16px', overflowY: 'auto' }}>
          
          {/* Header */}
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div>
              <h3 style={{ fontSize: '16px', fontWeight: 'bold', display: 'flex', alignItems: 'center', gap: '8px' }}>
                <ShoppingBag size={18} color="var(--accent-cyan)" />
                <span>{t('store_view_title')}</span>
              </h3>
              <p style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                {t('pane2_desc')}
              </p>
            </div>
            <span className="badge badge-cyan" style={{ fontSize: '11px' }}>
              {selectedLegend?.displayName}
            </span>
          </div>

          {/* Store Showcase Visual Cards Grid */}
          <div style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
            gap: '12px'
          }}>
            {/* Store Card 1: Target Costume in Inventory */}
            <div className="card-elevated" style={{
              display: 'flex',
              flexDirection: 'column',
              padding: '12px',
              border: '1px solid var(--border-subtle)',
              position: 'relative'
            }}>
              <span className="badge badge-cyan" style={{
                position: 'absolute',
                top: '8px',
                insetInlineEnd: '8px',
                fontSize: '10px'
              }}>
                {t('store_target_badge')}
              </span>
              <div style={{
                height: '110px',
                borderRadius: 'var(--radius-sm)',
                overflow: 'hidden',
                background: '#080c14',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                marginBottom: '8px',
                border: '1px solid rgba(255, 255, 255, 0.05)'
              }}>
                <AssetImage
                  type="costume"
                  item={selectedTargetCostume}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Layers}
                  fallbackText={selectedTargetCostume?.displayName}
                />
              </div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>{t('pane2_host_costume')}</div>
              <div style={{ fontSize: '14px', fontWeight: 'bold', color: 'var(--text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {selectedTargetCostume?.displayName || 'Host Costume'}
              </div>
              <div style={{ fontSize: '11px', color: 'var(--accent-cyan)', marginTop: '2px' }}>
                {selectedTargetCostume?.swfFile || selectedLegend?.primarySwf}
              </div>
            </div>

            {/* Store Card 2: Visual Replacement Skin */}
            <div className="card-elevated" style={{
              display: 'flex',
              flexDirection: 'column',
              padding: '12px',
              border: selectedReplacementCostume ? '1px solid rgba(16, 185, 129, 0.4)' : '1px solid var(--border-subtle)',
              position: 'relative'
            }}>
              <span className={`badge ${selectedReplacementCostume ? 'badge-green' : 'badge-violet'}`} style={{
                position: 'absolute',
                top: '8px',
                insetInlineEnd: '8px',
                fontSize: '10px'
              }}>
                {t('store_repl_badge')}
              </span>
              <div style={{
                height: '110px',
                borderRadius: 'var(--radius-sm)',
                overflow: 'hidden',
                background: '#080c14',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                marginBottom: '8px',
                border: '1px solid rgba(255, 255, 255, 0.05)'
              }}>
                <AssetImage
                  type="costume"
                  item={selectedReplacementCostume || selectedTargetCostume}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Sparkles}
                  fallbackText={selectedReplacementCostume?.displayName || selectedTargetCostume?.displayName}
                />
              </div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>{t('pane2_visual_repl')}</div>
              <div style={{ fontSize: '14px', fontWeight: 'bold', color: selectedReplacementCostume ? 'var(--color-success)' : 'var(--text-secondary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {selectedReplacementCostume?.displayName || `${selectedTargetCostume?.displayName || 'Host'} (${t('pane1_costume_unchanged')})`}
              </div>
              <div style={{ fontSize: '11px', color: 'var(--accent-violet)', marginTop: '2px' }}>
                {selectedReplacementCostume?.swfFile || selectedTargetCostume?.swfFile}
              </div>
            </div>

            {/* Store Card 3: Weapon 1 Visual */}
            <div className="card-elevated" style={{
              display: 'flex',
              flexDirection: 'column',
              padding: '12px',
              border: selectedReplacementWeapon1 ? '1px solid rgba(16, 185, 129, 0.4)' : '1px solid var(--border-subtle)',
              position: 'relative'
            }}>
              <span className="badge badge-cyan" style={{
                position: 'absolute',
                top: '8px',
                insetInlineEnd: '8px',
                fontSize: '10px'
              }}>
                {tWeapon(selectedLegend?.weapon1)}
              </span>
              <div style={{
                height: '90px',
                borderRadius: 'var(--radius-sm)',
                overflow: 'hidden',
                background: '#080c14',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                marginBottom: '8px',
                border: '1px solid rgba(255, 255, 255, 0.05)'
              }}>
                <AssetImage
                  type="weapon"
                  item={selectedReplacementWeapon1 || selectedTargetWeapon1}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Sword}
                  fallbackText="W1"
                />
              </div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                {t('pane2_visual_weapon')}
              </div>
              <div style={{ fontSize: '13px', fontWeight: 'bold', color: selectedReplacementWeapon1 ? 'var(--color-success)' : 'var(--text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {selectedReplacementWeapon1?.displayName || selectedTargetWeapon1?.displayName || t('pane1_weapon_unchanged', { weapon: tWeapon(selectedLegend?.weapon1) })}
              </div>
              <div style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '2px' }}>
                {selectedReplacementWeapon1?.ownerHero || t('universal_label')}
              </div>
            </div>

            {/* Store Card 4: Weapon 2 Visual */}
            <div className="card-elevated" style={{
              display: 'flex',
              flexDirection: 'column',
              padding: '12px',
              border: selectedReplacementWeapon2 ? '1px solid rgba(16, 185, 129, 0.4)' : '1px solid var(--border-subtle)',
              position: 'relative'
            }}>
              <span className="badge badge-violet" style={{
                position: 'absolute',
                top: '8px',
                insetInlineEnd: '8px',
                fontSize: '10px'
              }}>
                {tWeapon(selectedLegend?.weapon2)}
              </span>
              <div style={{
                height: '90px',
                borderRadius: 'var(--radius-sm)',
                overflow: 'hidden',
                background: '#080c14',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                marginBottom: '8px',
                border: '1px solid rgba(255, 255, 255, 0.05)'
              }}>
                <AssetImage
                  type="weapon"
                  item={selectedReplacementWeapon2 || selectedTargetWeapon2}
                  style={{ width: '100%', height: '100%' }}
                  fallbackIcon={Sword}
                  fallbackText="W2"
                />
              </div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                {t('pane2_visual_weapon')}
              </div>
              <div style={{ fontSize: '13px', fontWeight: 'bold', color: selectedReplacementWeapon2 ? 'var(--color-success)' : 'var(--text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {selectedReplacementWeapon2?.displayName || selectedTargetWeapon2?.displayName || t('pane1_weapon_unchanged', { weapon: tWeapon(selectedLegend?.weapon2) })}
              </div>
              <div style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '2px' }}>
                {selectedReplacementWeapon2?.ownerHero || t('universal_label')}
              </div>
            </div>
          </div>

          {/* Color Scheme Banner if active */}
          {selectedColorScheme && (
            <div className="card-elevated" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '10px 14px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <Palette size={18} color="var(--accent-violet)" />
                <div>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>{t('pane2_palette_trans')}</div>
                  <div style={{ fontSize: '13px', fontWeight: 'bold', color: 'var(--accent-violet)' }}>
                    {selectedColorScheme.displayName} ({selectedColorScheme.rarity})
                  </div>
                </div>
              </div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                {t('pane2_palette_bakes')}
              </div>
            </div>
          )}

          {/* High-Res Vector SVG Preview */}
          <div className="card-elevated" style={{ flex: 1, minHeight: '180px', display: 'flex', flexDirection: 'column' }}>
            <div style={{ fontSize: '12px', fontWeight: 'bold', color: 'var(--text-secondary)', marginBottom: '8px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span>{t('pane2_preview_title')}</span>
              {isLoadingPreview && <span style={{ fontSize: '11px', color: 'var(--accent-blue)' }}>{t('pane2_rendering')}</span>}
            </div>

            <div style={{
              flex: 1,
              backgroundColor: 'var(--bg-deep)',
              borderRadius: 'var(--radius-sm)',
              border: '1px solid var(--border-subtle)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '16px',
              overflow: 'hidden',
              minHeight: '130px'
            }}>
              {isLoadingPreview ? (
                <div className="spinner" />
              ) : previewSvg ? (
                <div
                  dangerouslySetInnerHTML={{ __html: previewSvg }}
                  style={{ width: '100%', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                />
              ) : (
                <div style={{ textAlign: 'center', color: 'var(--text-muted)', fontSize: '12px' }}>
                  <Sword size={28} color="var(--border-light)" style={{ margin: '0 auto 8px', display: 'block' }} />
                  <div>{t('pane2_preview_placeholder')}</div>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ================= PANE 3: COMPATIBILITY REPORT ================= */}
        <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '14px', overflowY: 'auto' }}>
          <div>
            <h3 style={{ fontSize: '15px', fontWeight: 'bold' }}>{t('pane3_title')}</h3>
            <p style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
              {t('pane3_desc')}
            </p>
          </div>

          {/* Status Badge */}
          <div style={{
            padding: '10px 14px',
            borderRadius: 'var(--radius-sm)',
            backgroundColor:
              isLevelIncompatible
                ? 'var(--color-danger-bg)'
                : isLevelExperimental
                  ? 'var(--color-warning-bg)'
                  : 'var(--color-success-bg)',
            border: `1px solid ${
              isLevelIncompatible
                ? 'var(--color-danger-border)'
                : isLevelExperimental
                  ? 'var(--color-warning-border)'
                  : 'var(--color-success-border)'
            }`,
            color:
              isLevelIncompatible
                ? '#F87171'
                : isLevelExperimental
                  ? '#FBBF24'
                  : '#34D399',
            fontWeight: 'bold',
            fontSize: '13px',
            display: 'flex',
            alignItems: 'center',
            gap: '8px'
          }}>
            {isValidating ? (
              <RefreshCw size={16} className="animate-spin" />
            ) : isLevelIncompatible ? (
              <AlertTriangle size={16} />
            ) : (
              <CheckCircle2 size={16} />
            )}
            <span>
              {t('status_label')}{' '}
              {isValidating
                ? t('status_validating')
                : isLevelIncompatible
                  ? t('status_incompatible')
                  : isLevelExperimental
                    ? t('status_cross_experimental')
                    : (compatibility?.isCompatible ? t('status_compatible') : t('status_ready'))}
            </span>
          </div>

          {/* Diagnostics Log */}
          <div>
            <div style={{ fontSize: '12px', fontWeight: '600', color: 'var(--text-secondary)', marginBottom: '6px' }}>
              {t('pane3_analysis_title')}
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              {compatibility?.diagnostics && compatibility.diagnostics.length > 0 ? (
                compatibility.diagnostics.map((diag, index) => (
                  <div key={index} style={{ fontSize: '11px', color: 'var(--text-secondary)', lineHeight: '1.4' }}>
                    • {diag}
                  </div>
                ))
              ) : (
                <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                  • {t('pane3_select_prompt')}
                </div>
              )}
            </div>
          </div>

          {/* Asset Property Inspect */}
          <div className="card-elevated" style={{ fontSize: '11px', display: 'flex', flexDirection: 'column', gap: '6px' }}>
            <div>
              <span style={{ color: 'var(--text-muted)' }}>{t('pane3_target_pkg')} </span>
              <span style={{ color: 'var(--text-primary)', fontFamily: 'monospace' }}>
                {selectedTargetCostume?.swfFile || selectedLegend?.primarySwf || 'None'}
              </span>
            </div>
            <div>
              <span style={{ color: 'var(--text-muted)' }}>{t('pane3_target_suffix')} </span>
              <span style={{ color: 'var(--text-primary)' }}>
                {selectedTargetCostume?.symbolSuffix || 'Default'}
              </span>
            </div>
            <div>
              <span style={{ color: 'var(--text-muted)' }}>{t('pane3_cross_port')} </span>
              <span style={{ color: compatibility?.requiresCrossSwfImport ? 'var(--accent-cyan)' : 'var(--text-primary)' }}>
                {compatibility?.requiresCrossSwfImport ? t('pane3_cross_yes') : t('pane3_cross_no')}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* ================= BOTTOM ACTION BAR ================= */}
      <div className="card" style={{
        padding: '12px 20px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        flexShrink: 0,
        gap: '16px',
        flexWrap: 'wrap'
      }}>
        <div style={{ display: 'flex', gap: '10px' }}>
          <button
            className="btn btn-danger"
            onClick={onRestoreTargetCostume}
            disabled={isBusy}
          >
            <RotateCcw size={14} />
            <span>{t('btn_restore_target')}</span>
          </button>
        </div>

        {/* Center: Live Auto-Restart Option */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer', fontSize: '12px', userSelect: 'none' }}>
            <input
              type="checkbox"
              checked={autoRestart}
              onChange={(e) => setAutoRestart(e.target.checked)}
              style={{ cursor: 'pointer', accentColor: 'var(--accent-blue)', width: '16px', height: '16px' }}
            />
            <span style={{ color: 'var(--text-secondary)' }}>
              {t('opt_auto_restart')}
            </span>
          </label>
        </div>

        <div style={{ display: 'flex', gap: '10px' }}>
          {systemInfo?.isGameRunning && (
            <button
              className="btn btn-secondary"
              onClick={onRestartGame}
              disabled={isBusy}
              title={t('btn_restart_game_tooltip')}
            >
              <RefreshCw size={14} />
              <span>{t('btn_restart_game')}</span>
            </button>
          )}

          <button
            className="btn btn-secondary"
            onClick={onLaunchGame}
            disabled={isBusy}
            title={systemInfo?.isGameRunning ? "Bring running Brawlhalla window to foreground" : "Launch Brawlhalla via Steam"}
          >
            <Play size={14} fill="currentColor" />
            <span>{systemInfo?.isGameRunning ? t('btn_focus_brawlhalla') : t('btn_launch_brawlhalla')}</span>
          </button>

          <button
            className="btn btn-primary"
            style={{ padding: '10px 24px', fontSize: '14px' }}
            onClick={() => onApplyModification(autoRestart)}
            disabled={isBusy || isLevelIncompatible}
          >
            {isBusy ? <div className="spinner-xs" /> : <CheckCircle2 size={16} />}
            <span>{t('btn_apply_mod')}</span>
          </button>
        </div>
      </div>
    </div>
  );
}
