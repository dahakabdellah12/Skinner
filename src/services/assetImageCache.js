// Shared in-memory SVG cache for asset previews with bounded capacity to prevent memory leaks.
// Kept outside the component module so fast-refresh stays component-only.

const memoryCache = new Map();
const subscribers = new Set();
const MAX_CACHE_SIZE = 5000;

export function getCachedSvg(key) {
  return memoryCache.get(key);
}

export function hasCachedSvg(key) {
  return memoryCache.has(key);
}

export function getCacheStats() {
  return {
    size: memoryCache.size,
    capacity: MAX_CACHE_SIZE,
    isPreloaded: memoryCache.size >= 500
  };
}

export function setCachedSvg(key, svg) {
  if (memoryCache.size >= MAX_CACHE_SIZE && !memoryCache.has(key)) {
    // Evict oldest entry (FIFO) to keep memory usage bounded during long sessions
    const oldestKey = memoryCache.keys().next().value;
    if (oldestKey) {
      memoryCache.delete(oldestKey);
    }
  }
  memoryCache.set(key, svg);
}

export function setBatchCachedSvgs(entriesMap) {
  if (!entriesMap) return;
  for (const [key, svg] of Object.entries(entriesMap)) {
    if (key && svg) {
      setCachedSvg(key, svg);
    }
  }
}

export function subscribeToCacheInvalidation(notify) {
  subscribers.add(notify);
  return () => {
    subscribers.delete(notify);
  };
}

export function clearAssetImageCache() {
  memoryCache.clear();
  subscribers.forEach((notify) => notify());
}

const STD_WEAPON = {
  weapon: '#8D8D95',
  weaponlt: '#ACB1B7',
  weapondk: '#65626D',
  weaponvl: '#CAD1D7',
  special: '#3E5466',
  speciallt: '#5D7B93',
  specialdk: '#2A3B49',
  specialvl: '#82A4BF',
  body1: '#485868',
  body2: '#82A4BF',
};

function toHex6(val) {
  if (val == null) return null;
  const n = Number(val);
  if (isNaN(n)) return null;
  return '#' + (n & 0xFFFFFF).toString(16).padStart(6, '0').toUpperCase();
}

/**
 * Ultra-fast client-side SVG palette swapping.
 * Instantly recolors cached base SVG in < 0.5ms so changing colors never shows loading spinners or delay.
 */
export function instantRecolorSvg(baseSvg, colorScheme, baseColors = {}) {
  if (!baseSvg || typeof baseSvg !== 'string' || !colorScheme) return null;

  const swapColors = colorScheme.swapColors;
  if (!swapColors) return null;

  let hasKeys = false;
  for (const _ in swapColors) {
    hasKeys = true;
    break;
  }
  if (!hasKeys) return null;

  const hexMap = new Map();
  const normBaseColors = {};
  if (baseColors && typeof baseColors === 'object') {
    for (const [k, v] of Object.entries(baseColors)) {
      if (k && v != null) {
        normBaseColors[k.toLowerCase()] = v;
      }
    }
  }

  for (const [key, targetVal] of Object.entries(swapColors)) {
    const targetHex = toHex6(targetVal);
    if (!targetHex) continue;
    const kLow = key.toLowerCase();

    if (STD_WEAPON[kLow]) {
      hexMap.set(STD_WEAPON[kLow].toUpperCase(), targetHex);
      hexMap.set(STD_WEAPON[kLow].toLowerCase(), targetHex);
    }

    const baseVal = normBaseColors[kLow] ?? baseColors[key];
    if (baseVal != null) {
      const baseHex = toHex6(baseVal);
      if (baseHex) {
        hexMap.set(baseHex.toUpperCase(), targetHex);
        hexMap.set(baseHex.toLowerCase(), targetHex);
      }
    }
  }

  if (hexMap.size === 0) return null;

  let replacedCount = 0;
  const result = baseSvg.replace(/(fill|stroke|stop-color)="([^"]+)"/gi, (match, attr, color) => {
    const cleanColor = color.trim().toUpperCase();
    if (hexMap.has(cleanColor)) {
      const target = hexMap.get(cleanColor);
      if (cleanColor !== target) {
        replacedCount++;
      }
      return `${attr}="${target}"`;
    }
    return match;
  });

  return replacedCount > 0 ? result : null;
}
