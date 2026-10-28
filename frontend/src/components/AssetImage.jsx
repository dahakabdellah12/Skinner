import React, { useState, useEffect, useMemo } from 'react';
import api from '../services/api';

const memoryCache = new Map();

export function clearAssetImageCache() {
  memoryCache.clear();
}

export default function AssetImage({
  type,
  item,
  colorScheme = null,
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
        hero: item.heroName
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
        colorSchemeId: csId ? String(csId) : undefined
      };
    } else if (type === 'weapon' && item) {
      cacheKey = `weapon_${item.baseWeapon}_${item.weaponSkinName}_${item.symbolSuffix}_${item.ownerHero || ''}${csKey}`;
      queryParams = {
        type: 'weapon',
        baseWeapon: item.baseWeapon,
        symbol: item.symbolSuffix,
        swfFile: item.swfFile,
        ownerHero: item.ownerHero,
        colorSchemeId: csId ? String(csId) : undefined
      };
    }

    return { cacheKey, queryParams };
  }, [type, item, colorScheme]);

  const { cacheKey, queryParams } = assetInfo;
  const [svg, setSvg] = useState(() => (cacheKey && memoryCache.has(cacheKey) ? memoryCache.get(cacheKey) : ''));

  useEffect(() => {
    if (!cacheKey || !queryParams) {
      return;
    }

    if (memoryCache.has(cacheKey)) {
      return;
    }

    let isCancelled = false;

    api.getAssetSvg(queryParams)
      .then((result) => {
        if (!isCancelled) {
          if (result) {
            memoryCache.set(cacheKey, result);
            setSvg(result);
          } else {
            setSvg('');
          }
        }
      })
      .catch(() => {
        if (!isCancelled) setSvg('');
      });

    return () => {
      isCancelled = true;
    };
  }, [cacheKey, queryParams]);

  const currentSvg = (cacheKey && memoryCache.has(cacheKey)) ? memoryCache.get(cacheKey) : svg;
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
          background: 'linear-gradient(90deg, rgba(255,255,255,0.02) 25%, rgba(255,255,255,0.07) 50%, rgba(255,255,255,0.02) 75%)',
          backgroundSize: '200% 100%',
          animation: 'shimmer 1.5s infinite'
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
        <div style={{ textAlign: 'center', color: 'var(--text-muted)', fontSize: '11px', padding: '4px' }}>
          {FallbackIcon && typeof FallbackIcon === 'function' && (
            <FallbackIcon size={24} style={{ opacity: 0.6, margin: '0 auto 4px', display: 'block' }} />
          )}
          {fallbackText && <span>{String(fallbackText)}</span>}
        </div>
      )}
    </div>
  );
}
