// Shared in-memory SVG cache for asset previews with bounded capacity to prevent memory leaks.
// Kept outside the component module so fast-refresh stays component-only.

const memoryCache = new Map();
const subscribers = new Set();
const MAX_CACHE_SIZE = 600;

export function getCachedSvg(key) {
  return memoryCache.get(key);
}

export function hasCachedSvg(key) {
  return memoryCache.has(key);
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
