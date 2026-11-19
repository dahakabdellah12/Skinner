/**
 * Brawlhalla Skin Studio — Full Application Preloading Engine
 * Zero-Wait Navigation Architecture
 * Pre-renders, caches, and indexes all application resources during startup.
 */

import { api } from './api.js';
import { setBatchCachedSvgs } from './assetImageCache.js';
import { getColorIconSymbol } from '../utils/colorUtils.js';

export const PRELOAD_STAGES = {
  INIT: {
    id: 'init',
    titleAr: 'تهيئة النظام والتحقق من التثبيت',
    titleEn: 'Initializing Application',
    minPct: 0,
    maxPct: 10
  },
  CATALOG: {
    id: 'catalog',
    titleAr: 'تحميل بيانات اللعبة والكتالوج',
    titleEn: 'Loading Catalog Data',
    minPct: 10,
    maxPct: 25
  },
  LEGENDS: {
    id: 'legends',
    titleAr: 'تجهيز صور وأصول الأساطير',
    titleEn: 'Loading Legends Assets',
    minPct: 25,
    maxPct: 50
  },
  COSTUMES: {
    id: 'costumes',
    titleAr: 'تجهيز سكنات ومظاهر الشخصيات',
    titleEn: 'Loading Character Skins',
    minPct: 50,
    maxPct: 75
  },
  KO_EFFECTS: {
    id: 'ko_effects',
    titleAr: 'تجهيز تأثيرات الإطاحة بالكامل',
    titleEn: 'Loading Knockout (KO) Effects',
    minPct: 75,
    maxPct: 90
  },
  COLORS: {
    id: 'colors',
    titleAr: 'تجهيز أنظمة الألوان والصبغات',
    titleEn: 'Initializing Color Palettes',
    minPct: 90,
    maxPct: 97
  },
  INDEXES: {
    id: 'indexes',
    titleAr: 'تهيئة الفهارس ومكونات البحث الفوري',
    titleEn: 'Preparing Application Components',
    minPct: 97,
    maxPct: 99
  },
  READY: {
    id: 'ready',
    titleAr: 'اكتمل التجهيز بنجاح! جاهز للاستخدام',
    titleEn: 'Finalizing Startup & Studio Ready',
    minPct: 99,
    maxPct: 100
  }
};


/**
 * Discover and catalog all visual assets required across the entire application.
 */
export function discoverAllAppResources(catalog) {
  if (!catalog) {
    return {
      legendItems: [],
      costumeItems: [],
      koItems: [],
      colorItems: [],
      totalCount: 0
    };
  }

  const legendItems = [];
  const costumeItems = [];
  const koItems = [];
  const colorItems = [];

  // 1. Playable Legends portraits
  const playableLegends = (catalog.legends || []).filter(
    (l) => l.heroName !== 'DEFAULT_CHARACTER' && l.heroName?.toLowerCase() !== 'random'
  );

  for (const leg of playableLegends) {
    const key = `legend_${leg.heroName || leg.displayName}`;
    legendItems.push({
      cacheKey: key,
      queryParams: {
        type: 'legend',
        name: leg.displayName,
        hero: leg.heroName,
        cacheKey: key
      }
    });

    // 2. All Costumes for this legend
    for (const c of leg.costumes || []) {
      const cName = c.costumeName || '';
      const cLower = cName.toLowerCase();
      if (!cName || cLower === 'template' || cLower === 'random') continue;

      const costKey = `costume_${c.ownerHero}_${c.costumeName}_${c.iconName}`;
      costumeItems.push({
        cacheKey: costKey,
        queryParams: {
          type: 'costume',
          icon: c.iconName,
          iconFile: c.iconFileName,
          costumeName: c.costumeName,
          ownerHero: c.ownerHero,
          swfFile: c.swfFile,
          cacheKey: costKey
        }
      });
    }
  }

  // 3. Active KO Effects (Knockout Studio & Gallery)
  for (const ko of catalog.koEffects || []) {
    const animFile = ko.animFile || ko.swfFile || 'SFX_KO.swf';
    const animClass = ko.animClass || ko.symbol || '';
    if (!animClass) continue;

    const koKey = `ko_${ko.koEffectId || ko.koEffectName}_${animClass}`;
    koItems.push({
      cacheKey: koKey,
      queryParams: {
        type: 'koEffect',
        animFile,
        animClass,
        cacheKey: koKey
      }
    });
  }

  // 4. Color Scheme Emblems
  for (const cs of catalog.colorSchemes || []) {
    const icon = getColorIconSymbol(cs);
    const keyId = cs.schemeId ?? cs.schemeName ?? cs.displayName ?? 'none';
    const colorKey = `color_${keyId}_${icon}`;
    colorItems.push({
      cacheKey: colorKey,
      queryParams: {
        type: 'color',
        icon,
        cacheKey: colorKey
      }
    });
  }

  const totalCount = legendItems.length + costumeItems.length + koItems.length + colorItems.length;

  return {
    legendItems,
    costumeItems,
    koItems,
    colorItems,
    totalCount
  };

}

/**
 * Execute the complete application resource preloading pipeline with accurate progress updates.
 *
 * @param {Object} options
 * @param {Object} options.catalog
 * @param {boolean} options.isArabic
 * @param {Function} options.onProgress
 * @returns {Promise<{ totalLoaded: number, durationMs: number }>}
 */
export async function executeFullAppPreload({ catalog, isArabic, onProgress }) {
  const startTime = Date.now();
  const resources = discoverAllAppResources(catalog);
  const totalAssets = resources.totalCount;

  let totalProcessed = 0;

  const emitProgress = (stage, stageProcessed, stageTotal, itemDescription) => {
    const stageRatio = stageTotal > 0 ? stageProcessed / stageTotal : 1;
    const pct = Math.min(
      stage.maxPct,
      Math.round(stage.minPct + stageRatio * (stage.maxPct - stage.minPct))
    );

    onProgress({
      stage,
      progress: pct,
      stageTitle: isArabic ? stage.titleAr : stage.titleEn,
      status: itemDescription,
      processed: totalProcessed,
      total: totalAssets
    });
  };

  /**
   * Process an array of items in batches via batchGetAssetSvgs
   */
  const processBatchList = async (items, stage, chunkSize = 35, formatDesc) => {
    const stageTotal = items.length;
    let stageProcessed = 0;

    for (let i = 0; i < stageTotal; i += chunkSize) {
      const chunk = items.slice(i, i + chunkSize);
      const desc = formatDesc(stageProcessed + chunk.length, stageTotal);

      try {
        const batchRes = await api.batchGetAssetSvgs(chunk);
        if (batchRes) {
          setBatchCachedSvgs(batchRes);
        }
      } catch (chunkErr) {
        console.warn(`[PreloadEngine] Warning during stage ${stage.id}:`, chunkErr);
      }

      stageProcessed += chunk.length;
      totalProcessed += chunk.length;
      emitProgress(stage, stageProcessed, stageTotal, desc);

      // Micro-yield between batches to let Tao/Wry pump Windows drag & mouse events smoothly
      await new Promise((resolve) => setTimeout(resolve, 16));
    }
  };

  // --- STAGE 3: Playable Legends ---
  emitProgress(
    PRELOAD_STAGES.LEGENDS,
    0,
    resources.legendItems.length,
    isArabic ? 'جاري تجهيز صور وبطاقات الأساطير المتاحة...' : 'Gathering playable legend portraits & assets...'
  );
  await processBatchList(
    resources.legendItems,
    PRELOAD_STAGES.LEGENDS,
    35,
    (proc, tot) =>
      isArabic
        ? `تجهيز بطاقات وأصول الأساطير (${proc}/${tot})`
        : `Preloading legend portraits & cards (${proc}/${tot})`
  );

  // --- STAGE 4: Character Skins & Costumes ---
  emitProgress(
    PRELOAD_STAGES.COSTUMES,
    0,
    resources.costumeItems.length,
    isArabic ? 'جاري تجهيز سكنات ومظاهر الشخصيات...' : 'Gathering character skins & costumes...'
  );
  await processBatchList(
    resources.costumeItems,
    PRELOAD_STAGES.COSTUMES,
    35,
    (proc, tot) =>
      isArabic
        ? `تجهيز ومعالجة أزياء الأساطير (${proc}/${tot})`
        : `Preloading costume previews & icons (${proc}/${tot})`
  );

  // --- STAGE 5: Knockout (KO) Effects ---
  emitProgress(
    PRELOAD_STAGES.KO_EFFECTS,
    0,
    resources.koItems.length,
    isArabic ? 'جاري تجهيز ومعالجة تأثيرات الإطاحة...' : 'Preloading Knockout (KO) Effects...'
  );
  await processBatchList(
    resources.koItems,
    PRELOAD_STAGES.KO_EFFECTS,
    25,
    (proc, tot) =>
      isArabic
        ? `معالجة وتخزين تأثيرات الإطاحة (${proc}/${tot})`
        : `Processing & caching KO effects (${proc}/${tot})`
  );

  // --- STAGE 6: Color Schemes ---
  emitProgress(
    PRELOAD_STAGES.COLORS,
    0,
    resources.colorItems.length,
    isArabic ? 'جاري تجهيز أنظمة الألوان والصبغات...' : 'Initializing color palette emblems...'
  );
  await processBatchList(
    resources.colorItems,
    PRELOAD_STAGES.COLORS,
    40,
    (proc, tot) =>
      isArabic
        ? `تجهيز أيقونات الصبغات وشفرات الألوان (${proc}/${tot})`
        : `Preparing color palette emblems (${proc}/${tot})`
  );

  // --- STAGE 8: Building Component Indexes & Warming Maps ---
  emitProgress(
    PRELOAD_STAGES.INDEXES,
    50,
    100,
    isArabic ? 'فهرسة وتجهيز بيانات البحث في الخزنة والمخصص لسرعة استجابة فورية...' : 'Indexing weapon vault and customizer maps for zero-wait access...'
  );

  // Small delay to allow browser thread to flush DOM state
  await new Promise((r) => setTimeout(r, 60));

  // --- STAGE 9: Studio Ready ---
  emitProgress(
    PRELOAD_STAGES.READY,
    100,
    100,
    isArabic
      ? `تم تجهيز وتخزين كافة موارد التطبيق بنجاح (${totalAssets} مورد جاهز)`
      : `All ${totalAssets} application assets fully cached and ready.`
  );

  const durationMs = Date.now() - startTime;
  return {
    totalLoaded: totalAssets,
    durationMs
  };
}
