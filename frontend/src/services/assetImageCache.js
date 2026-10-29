// Shared in-memory SVG cache for asset previews.
// Kept outside the component module so fast-refresh stays component-only.

const memoryCache = new Map();
const subscribers = new Set();

export function getCachedSvg(key) {
  return memoryCache.get(key);
}

export function hasCachedSvg(key) {
  return memoryCache.has(key);
}

export function setCachedSvg(key, svg) {
  memoryCache.set(key, svg);
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
