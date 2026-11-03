import React, { useState, useEffect, useMemo } from 'react';
import api from '../services/api';
import {
  getCachedSvg,
  hasCachedSvg,
  setCachedSvg,
  subscribeToCacheInvalidation
} from '../services/assetImageCache';

export default function AssetImage({
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
      cacheKey = `weapon_${item.baseWeapon}_${item.weaponSkinName}_${item.symbolSuffix}_${hero}${costKey}${csKey}`;
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

    let isCancelled = false;
    setSvg('');

    api.getAssetSvg(queryParams)
      .then((result) => {
        if (!isCancelled) {
          if (result && typeof result === 'string' && result.trim().startsWith('<svg')) {
            setCachedSvg(cacheKey, result);
            setSvg(result);
          } else {
            setCachedSvg(cacheKey, '__FAILED__');
            setSvg('');
          }
        }
      })
      .catch(() => {
        if (!isCancelled) {
          setCachedSvg(cacheKey, '__FAILED__');
          setSvg('');
        }
      });

    return () => {
      isCancelled = true;
    };
  }, [cacheKey, queryParams, cacheEpoch]);

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
