import React, { useState, useEffect, useMemo } from 'react';
import api from '../services/api';
import {
  getCachedSvg,
  hasCachedSvg,
  setCachedSvg,
  subscribeToCacheInvalidation,
  instantRecolorSvg
} from '../services/assetImageCache';

function AssetImage({
  type,
  item,
  colorScheme = null,
  costumeName = '',
  ownerHero = '',
  style = {},
  className = '',
  fallbackIcon: FallbackIcon = null,
  fallbackText = ''
}) {
  const assetInfo = useMemo(() => {
    let cacheKey = '';
    let queryParams = null;

    const csId = colorScheme
      ? (typeof colorScheme === 'object'
          ? (colorScheme.schemeId ?? colorScheme.schemeName ?? '')
          : colorScheme)
      : '';
    const csKey = csId ? `_cs_${csId}` : '';

    if (type === 'legend' && item) {
      cacheKey = `legend_${item.heroName || item.displayName}`;
      queryParams = {
        type: 'legend',
        name: item.displayName,
        hero: item.heroName,
        cacheKey
      };
    } else if (type === 'costume' && item) {
      cacheKey = `costume_${item.ownerHero}_${item.costumeName}_${item.iconName}${csKey}`;
      queryParams = {
        type: 'costume',
        icon: item.iconName,
        iconFile: item.iconFileName,
        costumeName: item.costumeName,
        ownerHero: item.ownerHero,
        swfFile: item.swfFile,
        colorSchemeId: csId ? String(csId) : undefined,
        cacheKey
      };
    } else if (type === 'weapon' && item) {
      const hero = item.ownerHero || ownerHero || '';
      const costKey = costumeName ? `_${costumeName}` : '';
      const isPaired = ['Katar', 'Katars', 'Boots', 'BattleBoots', 'Fists', 'Gauntlet', 'Gauntlets', 'Chakram'].includes(item.baseWeapon);
      const pairedSuffix = isPaired ? '_paired_v2' : '';
      cacheKey = `weapon_${item.baseWeapon}_${item.weaponSkinName}_${item.symbolSuffix}_${hero}${costKey}${csKey}${pairedSuffix}`;
      queryParams = {
        type: 'weapon',
        baseWeapon: item.baseWeapon,
        symbol: item.symbolSuffix,
        weaponSkinName: item.weaponSkinName || item.displayName || '',
        swfFile: item.swfFile,
        ownerHero: hero,
        costumeName: costumeName || '',
        colorSchemeId: csId ? String(csId) : undefined,
        cacheKey
      };
    } else if (type === 'color' && item) {
      const icon = item.iconName || item.icon || 'a_ColorIcon_None';
      const keyId = item.schemeId ?? item.schemeName ?? item.displayName ?? 'none';
      cacheKey = `color_${keyId}_${icon}`;
      queryParams = {
        type: 'color',
        icon,
        cacheKey
      };
    } else if ((type === 'koEffect' || type === 'ko') && item) {
      const ko = item.rawItem || item;
      const animFile = ko.animFile || ko.swfFile || 'SFX_KO.swf';
      const animClass = ko.animClass || ko.symbol || '';
      cacheKey = `ko_${ko.koEffectId || ko.koEffectName}_${animClass}`;
      queryParams = {
        type: 'koEffect',
        animFile,
        animClass,
        cacheKey
      };
    }

    return { cacheKey, queryParams };
  }, [type, item, colorScheme, costumeName, ownerHero]);

  const { cacheKey, queryParams } = assetInfo;
  const [cacheEpoch, setCacheEpoch] = useState(0);
  const [svg, setSvg] = useState(() => (cacheKey && hasCachedSvg(cacheKey) ? getCachedSvg(cacheKey) : ''));

  useEffect(() => {
    return subscribeToCacheInvalidation(() => {
      setSvg('');
      setCacheEpoch((prev) => prev + 1);
    });
  }, []);

  useEffect(() => {
    if (!cacheKey || !queryParams) {
      setSvg('');
      return;
    }

    if (hasCachedSvg(cacheKey)) {
      setSvg(getCachedSvg(cacheKey));
      return;
    }

    // High performance instant client-side palette recoloring
    let instantFound = false;
    if (colorScheme) {
      const baseKey = cacheKey.replace(/_cs_[^_]+$/, '');
      if (hasCachedSvg(baseKey)) {
        const baseSvg = getCachedSvg(baseKey);
        if (typeof baseSvg === 'string' && baseSvg.trim().startsWith('<svg')) {
          const instantSvg = instantRecolorSvg(baseSvg, colorScheme, item?.baseColors || {});
          if (instantSvg) {
            setCachedSvg(cacheKey, instantSvg);
            setSvg(instantSvg);
            instantFound = true;
          }
        }
      }
    }

    let isCancelled = false;
    if (!instantFound) {
      // Retain current SVG if transitioning between color variations instead of flashing blank
      if (!cacheKey.includes('_cs_')) {
        setSvg('');
      }
    }

    api.getAssetSvg(queryParams)
      .then((result) => {
        if (!isCancelled) {
          if (result && typeof result === 'string' && result.trim().startsWith('<svg')) {
            setCachedSvg(cacheKey, result);
            setSvg(result);
          } else if (!instantFound) {
            setCachedSvg(cacheKey, '__FAILED__');
            setSvg('');
          }
        }
      })
      .catch(() => {
        if (!isCancelled && !instantFound) {
          setCachedSvg(cacheKey, '__FAILED__');
          setSvg('');
        }
      });

    return () => {
      isCancelled = true;
    };
  }, [cacheKey, queryParams, colorScheme, item, cacheEpoch]);

  const currentSvg = (cacheKey && hasCachedSvg(cacheKey)) ? getCachedSvg(cacheKey) : svg;
  const isSvgValid = typeof currentSvg === 'string' && currentSvg.trim().startsWith('<svg');

  return (
    <div
      className={`asset-image-container ${className}`}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        overflow: 'hidden',
        position: 'relative',
        ...style
      }}
    >
      {!isSvgValid && Boolean(cacheKey) && (
        <div style={{
          position: 'absolute',
          inset: 0,
          background: 'var(--bg-card)'
        }} />
      )}

      {isSvgValid ? (
        <div
          dangerouslySetInnerHTML={{ __html: currentSvg }}
          style={{
            width: '100%',
            height: '100%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center'
          }}
        />
      ) : (
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          width: '100%',
          height: '100%',
          color: 'var(--text-muted)',
          fontSize: '11px',
          padding: '8px'
        }}>
          {FallbackIcon && typeof FallbackIcon === 'function' ? (
            <FallbackIcon size={28} style={{ opacity: 0.7, marginBottom: '4px' }} />
          ) : (
            <div style={{
              width: '46px',
              height: '46px',
              borderRadius: '50%',
              background: 'linear-gradient(135deg, rgba(56, 189, 248, 0.15), rgba(99, 102, 241, 0.05))',
              border: '1px solid rgba(56, 189, 248, 0.3)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: '20px',
              fontWeight: 800,
              color: '#38bdf8',
              boxShadow: '0 4px 14px rgba(0,0,0,0.4)',
              textShadow: '0 0 10px rgba(56, 189, 248, 0.5)'
            }}>
              {(item?.displayName || item?.heroName || fallbackText || '?').charAt(0).toUpperCase()}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default React.memo(AssetImage);
