import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import AssetImage from './AssetImage';
import CardPickerModal, { ColorEmblem } from './CardPickerModal';
import { api } from '../services/api';

// Cross-legend character borrowing is disabled by default to focus 100% on high-quality, authentic legend skins.
// Kept modular so it can easily be re-enabled in the future when adding new characters.
const ENABLE_CROSS_LEGEND_PICKER = false;

export default function LegendCustomizer({
  legend,
  catalog,
  systemInfo,
  savedCustomization,
  onSaveCustomization,
  onBack,
  onApplyModification,
  onRestoreTargetCostume,
  onRestartGame,
  onLaunchGame,
  showToast,
  isBusy
}) {
  const { tWeapon, isArabic } = useLanguage();

  // All costumes available for this legend
  const legendCostumes = useMemo(() => {
    if (!legend) return [];
    if (legend.costumes && legend.costumes.length > 0) {
      return legend.costumes;
    }
    return catalog?.costumes?.filter((c) => c.ownerHero === legend.heroName) || [];
  }, [legend, catalog]);

  // All catalog costumes across all legends for Cross-Legend visual swapping
  const allCatalogCostumes = useMemo(() => {
    if (catalog?.costumes && catalog.costumes.length > 0) {
      return catalog.costumes;
    }
    return legendCostumes || [];
  }, [catalog, legendCostumes]);

  // Helper matchers for robust persistence comparison
  const matchCostume = (c, id) => {
    if (!c || id == null || id === '' || id === '__NONE__') return false;
    return String(c.costumeId) === String(id) || c.costumeName === String(id);
  };

  const matchWeapon = (w, id) => {
    if (!w || id == null || id === '' || id === '__NONE__') return false;
    if (w.weaponSkinId != null && w.weaponSkinId !== 0 && String(w.weaponSkinId) === String(id)) {
      return true;
    }
    const idStr = String(id);
    if (w.weaponSkinName === idStr) return true;
    return !idStr.match(/^\d+$/) && w.displayName === idStr;
  };

  const matchColorScheme = (cs, id) => {
    if (!cs || id == null || id === '' || id === '__NONE__') return false;
    return String(cs.schemeId) === String(id) || cs.schemeName === String(id);
  };

  // Crossover weapon mapping table ensuring authentic crossover weapon pairings
  const CROSSOVER_WEAPONS_MAP = [
    {
      matcher: (c) => {
        const s = `${c.costumeName || ''} ${c.displayName || ''} ${c.bio || ''}`.toLowerCase();
        return s.includes('xavier');
      },
      weapon1: 'HammerXavier',
      weapon2: 'SwordXavier'
    },
    {
      matcher: (c) => {
        const s = `${c.costumeName || ''} ${c.displayName || ''} ${c.bio || ''}`.toLowerCase();
        return s.includes('snakeeyes') || s.includes('snake eyes') || s.includes('snake_eyes');
      },
      weapon1: 'SwordSnakeEyes',
      weapon2: 'PistolSnakeEyes'
    },
    {
      matcher: (c) => {
        const s = `${c.costumeName || ''} ${c.displayName || ''} ${c.bio || ''}`.toLowerCase();
        return s.includes('chewbacca');
      },
      weapon1: 'SwordChewbacca',
      weapon2: 'PistolChewbacca'
    },
    {
      matcher: (c) => {
        const s = `${c.costumeName || ''} ${c.displayName || ''} ${c.bio || ''}`.toLowerCase();
        return s.includes('machoman') || s.includes('macho man') || s.includes('macho_man') || s.includes('macho');
      },
      weapon1: 'AxeMachoMan',
      weapon2: 'RocketLanceMachoMan'
    },
    {
      matcher: (c) => {
        const s = `${c.costumeName || ''} ${c.displayName || ''} ${c.bio || ''}`.toLowerCase();
        return s.includes('destinytitan') || s.includes('destiny titan') || s.includes('titan');
      },
      weapon1: 'FistsDestinyTitan',
      weapon2: 'CannonDestinyTitan'
    },
    {
      matcher: (c) => {
        const s = `${c.costumeName || ''} ${c.displayName || ''} ${c.bio || ''}`.toLowerCase();
        return s.includes('negan');
      },
      weapon1: 'SwordNegan',
      weapon2: 'GreatswordNegan'
    }
  ];

  const resolveCrossoverWeapons = (costume, w1Skins, w2Skins) => {
    if (!costume) return null;
    const entry = CROSSOVER_WEAPONS_MAP.find((m) => m.matcher(costume));
    if (!entry) return null;

    const findInList = (list, targetName) => {
      const lower = targetName.toLowerCase();
      return (list || []).find((w) => {
        const n = (w?.weaponSkinName || '').toLowerCase();
        const s = (w?.symbolSuffix || '').toLowerCase();
        return n === lower || s === lower || n.includes(lower);
      });
    };

    let w1 = findInList(w1Skins, entry.weapon1) || findInList(w1Skins, entry.weapon2);
    let w2 = findInList(w2Skins, entry.weapon2) || findInList(w2Skins, entry.weapon1);
    return { w1, w2 };
  };


  // Helper: Find matching weapon skin for a given costume (Strictly matches legend's weapon skins)
  const findMatchingWeaponSkin = (skins, costume, targetLegend = legend) => {
    if (!skins || skins.length === 0 || !costume) return null;
    const wSet = (costume.weaponSet || '').trim().toLowerCase();
    const cName = (costume.costumeName || '').trim().toLowerCase();
    const dName = (costume.displayName || '').trim().toLowerCase();
    const oHero = (costume.ownerHero || '').trim().toLowerCase();
    const legHero = (targetLegend?.heroName || '').trim().toLowerCase();

    // Verify owner hero does not conflict with another character
    const isValidOwner = (w) => {
      const wHero = (w?.ownerHero || '').trim().toLowerCase();
      if (!wHero) return true;
      if (oHero && (wHero === oHero || wHero.includes(oHero) || oHero.includes(wHero))) return true;
      if (legHero && (wHero === legHero || wHero.includes(legHero) || legHero.includes(wHero))) return true;
      if (wSet && (wHero === wSet || wHero.includes(wSet) || wSet.includes(wHero))) return true;
      return false;
    };

    // 1. Exact match with weaponSet
    if (wSet && wSet !== '--') {
      const exact = skins.find((w) => {
        const wName = (w.weaponSkinName || '').toLowerCase();
        const sSuffix = (w.symbolSuffix || '').toLowerCase();
        return (wName.endsWith(wSet) || sSuffix === wSet || wName === wSet) && isValidOwner(w);
      });
      if (exact) return exact;
    }

    // 2. Exact match with costumeName
    if (cName) {
      const exact = skins.find((w) => {
        const wName = (w.weaponSkinName || '').toLowerCase();
        const sSuffix = (w.symbolSuffix || '').toLowerCase();
        return (wName.endsWith(cName) || sSuffix === cName || wName === cName) && isValidOwner(w);
      });
      if (exact) return exact;
    }

    // 3. Substring match with weaponSet
    if (wSet && wSet !== '--') {
      const foundWithHero = skins.find((w) => {
        const wName = (w.weaponSkinName || '').toLowerCase();
        const sSuffix = (w.symbolSuffix || '').toLowerCase();
        return (wName.includes(wSet) || sSuffix.includes(wSet)) && isValidOwner(w);
      });
      if (foundWithHero) return foundWithHero;

      const foundAny = skins.find((w) => {
        const wName = (w.weaponSkinName || '').toLowerCase();
        const sSuffix = (w.symbolSuffix || '').toLowerCase();
        return wName.includes(wSet) || sSuffix.includes(wSet);
      });
      if (foundAny) return foundAny;
    }

    // 4. Substring match with costumeName
    if (cName) {
      const found = skins.find((w) => {
        const wName = (w.weaponSkinName || '').toLowerCase();
        return wName.includes(cName) && isValidOwner(w);
      });
      if (found) return found;
    }

    // 5. Token-based match with displayName
    if (dName) {
      const found = skins.find((w) => {
        const wDisp = (w.displayName || '').toLowerCase();
        return (wDisp === dName || wDisp.endsWith(' ' + dName) || wDisp.startsWith(dName + ' ')) && isValidOwner(w);
      });
      if (found) return found;
    }

    // 6. Default weapon of owner hero
    if (costume.isDefault) {
      const found = skins.find((w) => {
        const wHero = (w?.ownerHero || '').trim().toLowerCase();
        return (wHero === oHero || wHero === legHero) && w.isDefault;
      }) || skins.find((w) => {
        const wHero = (w?.ownerHero || '').trim().toLowerCase();
        return (wHero === oHero || wHero === legHero);
      });
      if (found) return found;
    }

    return null;
  };

  // Target costume (costume to be replaced in user inventory, defaults to Classic/Default)
  const [selectedTargetCostume, setSelectedTargetCostume] = useState(() => {
    if (savedCustomization?.targetCostumeId != null) {
      const found = legendCostumes.find((c) => matchCostume(c, savedCustomization.targetCostumeId));
      if (found) return found;
    }
    return legendCostumes.find((c) => c.isDefault) || legendCostumes[0] || null;
  });

  // Replacement costume (costume whose visual look will be applied)
  const initialReplacementCostume = useMemo(() => {
    if (savedCustomization?.replacementCostumeId === '__NONE__') return null;
    if (savedCustomization?.replacementCostumeId != null) {
      const found = legendCostumes.find((c) => matchCostume(c, savedCustomization.replacementCostumeId))
        || (ENABLE_CROSS_LEGEND_PICKER ? allCatalogCostumes.find((c) => matchCostume(c, savedCustomization.replacementCostumeId)) : null);
      if (found) return found;
    }
    return legendCostumes.find((c) => !c.isDefault) || null;
  }, [savedCustomization, legendCostumes, allCatalogCostumes]);

  const [selectedReplacementCostume, setSelectedReplacementCostume] = useState(initialReplacementCostume);

  // Available weapon skins matching Weapon 1 and Weapon 2 (Strictly isolated by baseWeapon)
  const weapon1Skins = useMemo(() => {
    if (!catalog?.weaponSkins || !legend?.weapon1) return [];
    const base = legend.weapon1.trim().toLowerCase();
    return catalog.weaponSkins.filter(
      (w) => (w.baseWeapon || '').trim().toLowerCase() === base
    );
  }, [catalog, legend]);

  const weapon2Skins = useMemo(() => {
    if (!catalog?.weaponSkins || !legend?.weapon2) return [];
    const base = legend.weapon2.trim().toLowerCase();
    return catalog.weaponSkins.filter(
      (w) => (w.baseWeapon || '').trim().toLowerCase() === base
    );
  }, [catalog, legend]);

  // Target weapons (strictly authentic defaults for this legend, never falls back to another legend)
  const defaultWeapon1 = useMemo(() => {
    const heroNameLower = (legend?.heroName || '').trim().toLowerCase();
    const found = weapon1Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === heroNameLower && w?.isDefault)
      || weapon1Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === heroNameLower);
    return found || {
      weaponSkinId: 0,
      weaponSkinName: `${legend?.weapon1 || 'Weapon'}Default`,
      displayName: isArabic ? `${tWeapon(legend?.weapon1 || '')} (الافتراضي)` : `${tWeapon(legend?.weapon1 || '')} (Stock)`,
      baseWeapon: legend?.weapon1 || '',
      ownerHero: legend?.heroName || '',
      symbolSuffix: legend?.heroName || '',
      swfFile: `Gfx_${legend?.heroName || 'Weapons'}.swf`,
      weaponIcon: '',
      soundBank: '',
      isDefault: true,
      baseColors: {}
    };
  }, [weapon1Skins, legend, tWeapon, isArabic]);

  const defaultWeapon2 = useMemo(() => {
    const heroNameLower = (legend?.heroName || '').trim().toLowerCase();
    const found = weapon2Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === heroNameLower && w?.isDefault)
      || weapon2Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === heroNameLower);
    return found || {
      weaponSkinId: 0,
      weaponSkinName: `${legend?.weapon2 || 'Weapon'}Default`,
      displayName: isArabic ? `${tWeapon(legend?.weapon2 || '')} (الافتراضي)` : `${tWeapon(legend?.weapon2 || '')} (Stock)`,
      baseWeapon: legend?.weapon2 || '',
      ownerHero: legend?.heroName || '',
      symbolSuffix: legend?.heroName || '',
      swfFile: `Gfx_${legend?.heroName || 'Weapons'}.swf`,
      weaponIcon: '',
      soundBank: '',
      isDefault: true,
      baseColors: {}
    };
  }, [weapon2Skins, legend, tWeapon, isArabic]);

  const selectedTargetWeapon1 = defaultWeapon1;
  const [selectedReplacementWeapon1, setSelectedReplacementWeapon1] = useState(() => {
    if (savedCustomization?.weapon1Id === '__NONE__') return null;
    if (savedCustomization?.weapon1Id != null) {
      const found = weapon1Skins.find((w) => matchWeapon(w, savedCustomization.weapon1Id));
      if (found) return found;
    }
    if (initialReplacementCostume) {
      const crossMatch = resolveCrossoverWeapons(initialReplacementCostume, weapon1Skins, weapon2Skins);
      if (crossMatch?.w1) return crossMatch.w1;
      const autoMatch = findMatchingWeaponSkin(weapon1Skins, initialReplacementCostume, legend);
      if (autoMatch) return autoMatch;
    }
    return null;
  });

  const selectedTargetWeapon2 = defaultWeapon2;
  const [selectedReplacementWeapon2, setSelectedReplacementWeapon2] = useState(() => {
    if (savedCustomization?.weapon2Id === '__NONE__') return null;
    if (savedCustomization?.weapon2Id != null) {
      const found = weapon2Skins.find((w) => matchWeapon(w, savedCustomization.weapon2Id));
      if (found) return found;
    }
    if (initialReplacementCostume) {
      const crossMatch = resolveCrossoverWeapons(initialReplacementCostume, weapon1Skins, weapon2Skins);
      if (crossMatch?.w2) return crossMatch.w2;
      const autoMatch = findMatchingWeaponSkin(weapon2Skins, initialReplacementCostume, legend);
      if (autoMatch) return autoMatch;
    }
    return null;
  });

  const [selectedColorScheme, setSelectedColorScheme] = useState(() => {
    if (savedCustomization?.colorSchemeId === '__NONE__') return null;
    if (savedCustomization?.colorSchemeId != null) {
      const found = catalog?.colorSchemes?.find((cs) => matchColorScheme(cs, savedCustomization.colorSchemeId));
      if (found) return found;
    }
    return null;
  });


  const [autoRestart, setAutoRestart] = useState(() => {
    return savedCustomization?.autoRestart !== undefined ? savedCustomization.autoRestart : true;
  });

  // Character/legend selected as source for replacement visual skin.
  // Locked to active studio legend to focus on quality; kept extensible for future character additions.
  const [selectedReplacementHero, setSelectedReplacementHero] = useState(() => {
    if (ENABLE_CROSS_LEGEND_PICKER && savedCustomization?.replacementHero) {
      return savedCustomization.replacementHero;
    }
    return legend?.heroName || '';
  });

  useEffect(() => {
    if (!ENABLE_CROSS_LEGEND_PICKER && legend?.heroName) {
      setSelectedReplacementHero(legend.heroName);
    }
  }, [legend?.heroName]);

  // Visual Character Cards for Cross-Legend selection
  const heroCards = useMemo(() => {
    const currentHeroLower = (legend?.heroName || '').trim().toLowerCase();

    return (catalog?.legends || [])
      .filter((l) => {
        const hName = (l.heroName || '').toLowerCase();
        const dName = (l.displayName || '').toLowerCase();
        return hName !== 'random' && dName !== 'random';
      })
      .map((l) => {
        const isCurrent = (l.heroName || '').toLowerCase() === currentHeroLower;
        const w1 = tWeapon(l.weapon1 || '');
        const w2 = tWeapon(l.weapon2 || '');
        const weaponsSubtitle = l.weapon1 && l.weapon2 ? `${w1} • ${w2}` : (l.bio || '');

        return {
          id: l.heroName,
          title: l.displayName || l.heroName,
          subtitle: isCurrent
            ? `${weaponsSubtitle} (${isArabic ? 'الأسطورة الحالية' : 'Current'})`
            : weaponsSubtitle,
          badge: isCurrent ? (isArabic ? 'الحالية' : 'Current') : null,
          rawItem: l,
          searchKey: `${l.displayName || ''} ${l.heroName || ''} ${l.weapon1 || ''} ${l.weapon2 || ''} ${w1} ${w2}`
        };
      })
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));
  }, [catalog?.legends, legend, tWeapon, isArabic]);

  // Selected hero object for thumbnail image
  const selectedHeroObject = useMemo(() => {
    if (!selectedReplacementHero) return null;
    return (catalog?.legends || []).find(
      (l) => (l.heroName || '').toLowerCase() === selectedReplacementHero.toLowerCase()
    ) || null;
  }, [catalog?.legends, selectedReplacementHero]);

  // Label for the selected replacement character
  const currentSelectedHeroDisplayName = useMemo(() => {
    if (!selectedReplacementHero) {
      return '';
    }
    const found = (catalog?.legends || []).find(
      (l) => (l.heroName || '').toLowerCase() === selectedReplacementHero.toLowerCase()
    );
    return found ? (found.displayName || found.heroName) : selectedReplacementHero;
  }, [selectedReplacementHero, catalog?.legends]);

  // Handler for picking a hero from the character cards modal
  const handleSelectHeroFilter = useCallback((hero) => {
    setSelectedReplacementHero(hero);

    // A specific hero was selected (e.g. Cassidy, Bodvar, Random, etc.)
    const heroCostumes = (allCatalogCostumes || []).filter(
      (c) => (c.ownerHero || '').trim().toLowerCase() === hero.trim().toLowerCase()
    );
    const newCostume = heroCostumes.find((c) => c.isDefault) || heroCostumes[0] || null;
    if (newCostume) {
      setSelectedReplacementCostume(newCostume);
      const crossMatch = resolveCrossoverWeapons(newCostume, weapon1Skins, weapon2Skins);
      const w1 = crossMatch?.w1 || findMatchingWeaponSkin(weapon1Skins, newCostume, legend);
      const w2 = crossMatch?.w2 || findMatchingWeaponSkin(weapon2Skins, newCostume, legend);
      if (w1) setSelectedReplacementWeapon1(w1);
      if (w2) setSelectedReplacementWeapon2(w2);
    }
  }, [allCatalogCostumes, weapon1Skins, weapon2Skins, legend]);

  const [isValidating, setIsValidating] = useState(false);
  const [compatibility, setCompatibility] = useState(null);
  const [activePicker, setActivePicker] = useState(null);

  // Sync state to parent for persistence whenever any option changes
  const isFirstMount = useRef(true);
  const lastSavedPayloadRef = useRef(null);

  const getCurrentCustomizationPayload = useCallback(() => {
    return {
      targetCostumeId: selectedTargetCostume?.costumeId ?? selectedTargetCostume?.costumeName,
      replacementHero: selectedReplacementHero,
      replacementCostumeId: selectedReplacementCostume ? (selectedReplacementCostume.costumeId ?? selectedReplacementCostume.costumeName) : '__NONE__',
      weapon1Id: selectedReplacementWeapon1 ? (selectedReplacementWeapon1.weaponSkinId ?? selectedReplacementWeapon1.weaponSkinName) : '__NONE__',
      weapon2Id: selectedReplacementWeapon2 ? (selectedReplacementWeapon2.weaponSkinId ?? selectedReplacementWeapon2.weaponSkinName) : '__NONE__',
      colorSchemeId: selectedColorScheme ? (selectedColorScheme.schemeId ?? selectedColorScheme.schemeName) : '__NONE__',
      autoRestart
    };
  }, [
    selectedTargetCostume,
    selectedReplacementHero,
    selectedReplacementCostume,
    selectedReplacementWeapon1,
    selectedReplacementWeapon2,
    selectedColorScheme,
    autoRestart
  ]);

  useEffect(() => {
    if (isFirstMount.current) {
      isFirstMount.current = false;
      const initial = getCurrentCustomizationPayload();
      lastSavedPayloadRef.current = JSON.stringify(initial);
      return;
    }

    const payload = getCurrentCustomizationPayload();
    const payloadStr = JSON.stringify(payload);
    if (lastSavedPayloadRef.current === payloadStr) {
      return;
    }
    lastSavedPayloadRef.current = payloadStr;
    onSaveCustomization?.(payload);
  }, [getCurrentCustomizationPayload, onSaveCustomization, legend?.heroName]);

  const handleBack = () => {
    const payload = getCurrentCustomizationPayload();
    const payloadStr = JSON.stringify(payload);
    if (lastSavedPayloadRef.current !== payloadStr) {
      lastSavedPayloadRef.current = payloadStr;
      onSaveCustomization?.(payload);
    }
    onBack();
  };

  // Revalidate compatibility whenever selections change
  useEffect(() => {
    async function validate() {
      if (!legend || !selectedTargetCostume) {
        setCompatibility(null);
        return;
      }
      setIsValidating(true);
      try {
        const req = {
          targetLegend: legend,
          targetCostume: selectedTargetCostume,
          replacementCostume: selectedReplacementCostume,
          targetWeapon1: selectedTargetWeapon1,
          replacementWeapon1: selectedReplacementWeapon1,
          targetWeapon2: selectedTargetWeapon2,
          replacementWeapon2: selectedReplacementWeapon2,
          selectedColorScheme: selectedColorScheme
        };
        const res = await api.validateModification(req);
        setCompatibility(res);
      } catch (err) {
        setCompatibility({
          isCompatible: false,
          level: 'Incompatible',
          diagnostics: [err.message]
        });
      } finally {
        setIsValidating(false);
      }
    }
    validate();
  }, [
    legend,
    selectedTargetCostume,
    selectedReplacementCostume,
    selectedTargetWeapon1,
    selectedReplacementWeapon1,
    selectedTargetWeapon2,
    selectedReplacementWeapon2
  ]);

  const legWeapon1 = legend?.weapon1 || '';
  const legWeapon2 = legend?.weapon2 || '';

  // Color scheme cards with authentic in-game color emblems (Sorted A to Z)
  const colorSchemeCards = useMemo(() => {
    const noneCard = {
      id: '__NONE__',
      title: isArabic ? 'ألوان الأسطورة الأصلية (بدون تعديل)' : 'Default Legend Colors (Stock)',
      subtitle: isArabic ? 'الصبغة الأساسية للأصل' : 'Stock Legend Palette',
      isNone: true
    };

    const rest = (catalog?.colorSchemes || [])
      .map((cs) => ({
        id: cs?.schemeId ?? cs?.schemeName ?? '__SCHEME__',
        title: cs?.displayName || cs?.schemeName || 'Color',
        subtitle: cs?.rarity ? `Rarity: ${cs.rarity}` : (isArabic ? 'نظام ألوان' : 'Color Scheme'),
        badge: cs?.displayName === 'Community Colors' ? 'Special' : null,
        rawItem: cs,
        searchKey: `${cs?.displayName || ''} ${cs?.schemeName || ''}`
      }))
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));

    return [noneCard, ...rest];
  }, [catalog?.colorSchemes, isArabic]);

  // --- Visual Card Gallery Items for Card Picker Modals ---
  const targetCostumeCards = useMemo(() => {
    return (legendCostumes || [])
      .filter((c) => {
        const cName = (c.costumeName || '').toLowerCase();
        const dName = (c.displayName || '').toLowerCase();
        return cName !== 'random' && dName !== 'random';
      })
      .map((c) => ({
        id: c.costumeId || c.costumeName,
        title: c.displayName || c.costumeName,
        subtitle: c.isDefault
          ? (isArabic ? 'الزي الافتراضي للأصل' : 'Default Costume')
          : (c.isCrossover ? `${c.ownerHero || ''} [${isArabic ? 'كروس أوفر' : 'Crossover'}]` : (c.ownerHero || '')),
        badge: c.isDefault ? (isArabic ? 'افتراضي' : 'Default') : null,
        rawItem: c,
        searchKey: `${c.displayName || ''} ${c.costumeName || ''} ${c.ownerHero || ''}`
      }))
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));
  }, [legendCostumes, isArabic]);

  const legDisplayName = legend?.displayName || '';
  const replacementCostumeCards = useMemo(() => {
    const noneCard = {
      id: '__NONE__',
      title: isArabic ? 'بدون تغيير (الاحتفاظ بمظهر الزي المستهدف)' : 'Unchanged (Keep Target Look)',
      subtitle: isArabic ? 'لا يتم تعديل مظهر الزي' : 'Stock Target Visual',
      isNone: true,
      ownerHero: legend?.heroName || ''
    };

    // Hero name to display name mapping
    const heroMap = new Map();
    (catalog?.legends || []).forEach((l) => {
      if (l.heroName) heroMap.set(l.heroName.toLowerCase(), l.displayName || l.heroName);
    });

    const currentHeroLower = (legend?.heroName || '').trim().toLowerCase();
    const selHeroLower = ENABLE_CROSS_LEGEND_PICKER
      ? (selectedReplacementHero || '').trim().toLowerCase()
      : currentHeroLower;

    // Filter costumes: strictly current studio legend's costumes if cross-legend is disabled
    let sourceCostumes = ENABLE_CROSS_LEGEND_PICKER ? (allCatalogCostumes || []) : (legendCostumes || []);
    sourceCostumes = sourceCostumes.filter((c) => {
      const cName = (c.costumeName || '').toLowerCase();
      const dName = (c.displayName || '').toLowerCase();
      return cName !== 'random' && dName !== 'random';
    });
    if (selHeroLower) {
      sourceCostumes = sourceCostumes.filter(
        (c) => (c.ownerHero || '').trim().toLowerCase() === selHeroLower
      );
    }

    const skins = sourceCostumes.map((c) => {
      const cHero = (c.ownerHero || '').trim();
      const cHeroLower = cHero.toLowerCase();
      const isCurrentLegend = cHeroLower === currentHeroLower;
      const heroDisp = heroMap.get(cHeroLower) || cHero || legDisplayName;

      let subtitle = heroDisp;
      if (c.isDefault) {
        subtitle = isArabic ? `${heroDisp} (الافتراضي)` : `${heroDisp} (Default / Stock)`;
      } else if (c.isCrossover) {
        subtitle = isArabic ? `${heroDisp} [كروس أوفر]` : `${heroDisp} [Crossover]`;
      }

      let badge = null;
      if (c.isDefault) {
        badge = isArabic ? 'افتراضي' : 'Default';
      } else if (!isCurrentLegend) {
        badge = heroDisp;
      }

      return {
        id: c.costumeId != null ? c.costumeId : c.costumeName,
        title: c.displayName || c.costumeName,
        subtitle,
        badge,
        rawItem: c,
        ownerHero: cHero,
        isCurrentLegend,
        isDefault: !!c.isDefault,
        searchKey: `${c.displayName || ''} ${c.costumeName || ''} ${heroDisp} ${cHero}`
      };
    });

    skins.sort((a, b) => {
      if (a.isCurrentLegend && !b.isCurrentLegend) return -1;
      if (!a.isCurrentLegend && b.isCurrentLegend) return 1;

      const heroCmp = (a.ownerHero || '').localeCompare(b.ownerHero || '');
      if (heroCmp !== 0) return heroCmp;

      if (a.isDefault && !b.isDefault) return -1;
      if (!a.isDefault && b.isDefault) return 1;

      return (a.title || '').localeCompare(b.title || '');
    });

    return [noneCard, ...skins];
  }, [legendCostumes, allCatalogCostumes, catalog?.legends, legend, legDisplayName, selectedReplacementHero, isArabic]);

  const weapon1Cards = useMemo(() => {
    const weaponLabel = tWeapon(legWeapon1);
    const noneCard = {
      id: '__NONE__',
      title: isArabic ? `الافتراضي لـ ${weaponLabel} (بدون تعديل)` : `Default ${weaponLabel} (Stock)`,
      subtitle: isArabic ? 'المظهر الأصلي للسلاح' : 'Stock Weapon Skin',
      isNone: true
    };
    const skins = (weapon1Skins || [])
      .map((w) => {
        const ownerLegend = catalog?.legends?.find(
          (l) => (l?.heroName || '').trim().toLowerCase() === (w?.ownerHero || '').trim().toLowerCase()
        );
        const ownerDisp = ownerLegend ? ownerLegend.displayName : (w?.ownerHero ? w.ownerHero : (isArabic ? 'عام' : 'Universal'));
        return {
          id: w.weaponSkinId != null && w.weaponSkinId !== 0 ? w.weaponSkinId : w.weaponSkinName,
          title: w.displayName || w.weaponSkinName,
          subtitle: ownerDisp,
          badge: null,
          rawItem: w,
          searchKey: `${w.displayName || ''} ${ownerDisp} ${w.weaponSkinName || ''}`
        };
      })
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));
    return [noneCard, ...skins];
  }, [weapon1Skins, legWeapon1, isArabic, tWeapon, catalog]);

  const weapon2Cards = useMemo(() => {
    const weaponLabel = tWeapon(legWeapon2);
    const noneCard = {
      id: '__NONE__',
      title: isArabic ? `الافتراضي لـ ${weaponLabel} (بدون تعديل)` : `Default ${weaponLabel} (Stock)`,
      subtitle: isArabic ? 'المظهر الأصلي للسلاح' : 'Stock Weapon Skin',
      isNone: true
    };
    const skins = (weapon2Skins || [])
      .map((w) => {
        const ownerLegend = catalog?.legends?.find(
          (l) => (l?.heroName || '').trim().toLowerCase() === (w?.ownerHero || '').trim().toLowerCase()
        );
        const ownerDisp = ownerLegend ? ownerLegend.displayName : (w?.ownerHero ? w.ownerHero : (isArabic ? 'عام' : 'Universal'));
        return {
          id: w.weaponSkinId != null && w.weaponSkinId !== 0 ? w.weaponSkinId : w.weaponSkinName,
          title: w.displayName || w.weaponSkinName,
          subtitle: ownerDisp,
          badge: null,
          rawItem: w,
          searchKey: `${w.displayName || ''} ${ownerDisp} ${w.weaponSkinName || ''}`
        };
      })
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));
    return [noneCard, ...skins];
  }, [weapon2Skins, legWeapon2, isArabic, tWeapon, catalog]);

  // Handle target costume select
  const handleSelectTargetCostume = (val) => {
    const found = legendCostumes.find((c) => matchCostume(c, val));
    setSelectedTargetCostume(found || null);
  };

  // Handle replacement costume select (auto-assigns matching weapons strictly for this costume)
  const handleSelectReplacementCostume = (val) => {
    if (val === '__NONE__') {
      setSelectedReplacementCostume(null);
      return;
    }
    const found = allCatalogCostumes.find((c) => matchCostume(c, val))
      || legendCostumes.find((c) => matchCostume(c, val));
    setSelectedReplacementCostume(found || null);

    if (found) {
      if (found.ownerHero && selectedReplacementHero !== 'Random' && selectedReplacementHero !== 'all') {
        setSelectedReplacementHero(found.ownerHero);
      }

      // Check if returning to default costume
      if (found.isDefault) {
        // Restores default weapon behavior for this legend
        const defW1 = weapon1Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === (legend?.heroName || '').trim().toLowerCase() && w?.isDefault)
          || weapon1Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === (legend?.heroName || '').trim().toLowerCase())
          || weapon1Skins.find((w) => w?.isDefault)
          || weapon1Skins[0];
        const defW2 = weapon2Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === (legend?.heroName || '').trim().toLowerCase() && w?.isDefault)
          || weapon2Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === (legend?.heroName || '').trim().toLowerCase())
          || weapon2Skins.find((w) => w?.isDefault)
          || weapon2Skins[0];
        setSelectedReplacementWeapon1(defW1 || null);
        setSelectedReplacementWeapon2(defW2 || null);
      } else {
        // 1. Try crossover weapon mapping first
        const crossMatch = resolveCrossoverWeapons(found, weapon1Skins, weapon2Skins);
        let w1 = crossMatch?.w1 || findMatchingWeaponSkin(weapon1Skins, found, legend);
        let w2 = crossMatch?.w2 || findMatchingWeaponSkin(weapon2Skins, found, legend);

        // Fallback to default weapons if no dedicated skin exists for this weapon type
        if (!w1) {
          w1 = weapon1Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === (legend?.heroName || '').trim().toLowerCase() && w?.isDefault)
            || weapon1Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === (legend?.heroName || '').trim().toLowerCase())
            || weapon1Skins[0];
        }
        if (!w2) {
          w2 = weapon2Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === (legend?.heroName || '').trim().toLowerCase() && w?.isDefault)
            || weapon2Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === (legend?.heroName || '').trim().toLowerCase())
            || weapon2Skins[0];
        }

        setSelectedReplacementWeapon1(w1 || null);
        setSelectedReplacementWeapon2(w2 || null);
      }
    }
  };

  // Handle weapon 1 select
  const handleSelectWeapon1 = (val) => {
    if (val === '__NONE__') {
      setSelectedReplacementWeapon1(null);
      return;
    }
    const found = weapon1Skins.find((w) => matchWeapon(w, val));
    setSelectedReplacementWeapon1(found || null);
  };

  // Handle weapon 2 select
  const handleSelectWeapon2 = (val) => {
    if (val === '__NONE__') {
      setSelectedReplacementWeapon2(null);
      return;
    }
    const found = weapon2Skins.find((w) => matchWeapon(w, val));
    setSelectedReplacementWeapon2(found || null);
  };

  // Handle color scheme select
  const handleSelectColorScheme = (val) => {
    if (val === '__NONE__') {
      setSelectedColorScheme(null);
      return;
    }
    const found = catalog?.colorSchemes?.find((cs) => matchColorScheme(cs, val));
    setSelectedColorScheme(found || null);
  };

  // Apply
  const handleApply = () => {
    if (!selectedTargetCostume) {
      showToast(isArabic ? 'يرجى تحديد الزي المستهدف.' : 'Target costume is required.', 'error');
      return;
    }
    onApplyModification({
      targetLegend: legend,
      targetCostume: selectedTargetCostume,
      replacementCostume: selectedReplacementCostume,
      targetWeapon1: selectedTargetWeapon1,
      replacementWeapon1: selectedReplacementWeapon1,
      targetWeapon2: selectedTargetWeapon2,
      replacementWeapon2: selectedReplacementWeapon2,
      selectedColorScheme: selectedColorScheme,
      autoRestart: autoRestart
    });
  };

  // Active items to preview
  const displayedTargetCostume = selectedTargetCostume;
  const displayedReplacementCostume = selectedReplacementCostume || selectedTargetCostume;
  const displayedWeapon1 = selectedReplacementWeapon1 || selectedTargetWeapon1;
  const displayedWeapon2 = selectedReplacementWeapon2 || selectedTargetWeapon2;

  return (
    <div className="customizer-workbench">
      {/* Workbench Navigation Header */}
      <div className="workbench-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
          <button
            className="button-secondary"
            onClick={handleBack}
            style={{ padding: '8px 14px', fontSize: '13px' }}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M19 12H5M12 19l-7-7 7-7" />
            </svg>
            <span>{isArabic ? 'العودة إلى قائمة الأساطير' : 'Back to Legends'}</span>
          </button>

          <div>
            <h2 style={{ fontSize: '18px', fontWeight: 800 }}>{legend.displayName}</h2>
          </div>
        </div>
      </div>

      {/* 4 Authentic Showcase Cards */}
      <div className="workbench-cards-grid">
        {/* Card 1: Target Costume (A) */}
        <div className="store-card">
          <div className="card-image-wrap">
            <div className="card-image">
              <AssetImage
                type="costume"
                item={displayedTargetCostume}
                style={{ width: '100%', height: '100%' }}
                fallbackText=""
              />
            </div>
            <span style={{
              position: 'absolute',
              top: '10px',
              right: '10px',
              color: 'var(--text-primary)',
              fontSize: '14px',
              fontWeight: 800,
              zIndex: 2,
              background: 'transparent',
              padding: 0
            }}>
              A
            </span>
          </div>
          <div className="card-content">
            <div className="card-heading">
              <h3 className="product-title" title={displayedTargetCostume?.displayName || displayedTargetCostume?.costumeName || ''}>
                {displayedTargetCostume?.displayName || displayedTargetCostume?.costumeName || legend?.displayName || 'Costume'}
              </h3>
            </div>
            <p className="product-description">
              {isArabic ? 'الزي الذي تملكه وسيتم استبدال شكله في اللعبة' : 'Costume in your game inventory to be cosmetically replaced.'}
            </p>
          </div>
        </div>

        {/* Card 2: Replacement Visual Skin (B) */}
        <div className="store-card">
          <div className="card-image-wrap">
            <div className="card-image">
              <AssetImage
                type="costume"
                item={displayedReplacementCostume}
                colorScheme={selectedColorScheme}
                style={{ width: '100%', height: '100%' }}
                fallbackText=""
              />
            </div>
            <span style={{
              position: 'absolute',
              top: '10px',
              right: '10px',
              color: 'var(--text-primary)',
              fontSize: '14px',
              fontWeight: 800,
              zIndex: 2,
              background: 'transparent',
              padding: 0
            }}>
              B
            </span>
          </div>
          <div className="card-content">
            <div className="card-heading">
              <h3 className="product-title" title={displayedReplacementCostume?.displayName || displayedReplacementCostume?.costumeName || ''}>
                {displayedReplacementCostume?.displayName || displayedReplacementCostume?.costumeName || (isArabic ? 'نفس الزي' : 'Same Costume')}
              </h3>
            </div>
            <p className="product-description">
              {displayedReplacementCostume?.ownerHero && displayedReplacementCostume.ownerHero.toLowerCase() !== (legend?.heroName || '').toLowerCase()
                ? (isArabic
                    ? `مظهر منسوخ من ${displayedReplacementCostume.ownerHero} • ${displayedReplacementCostume.isDefault ? 'الزي الافتراضي' : 'سكن'}`
                    : `Cross-Legend skin from ${displayedReplacementCostume.ownerHero} • ${displayedReplacementCostume.isDefault ? 'Stock' : 'Skin'}`)
                : (isArabic ? 'المظهر الذي سيظهر في اللعبة أثناء المعارك' : 'The visual look you will see during combat and animations.')}
            </p>
          </div>
        </div>

        {/* Card 3: Weapon 1 Skin */}
        <div className="store-card">
          <div className="card-image-wrap">
            <div className={`card-image weapon-preview weapon-${(displayedWeapon1?.baseWeapon || 'generic').toLowerCase().replace(/[\s_]+/g, '')}`}>
              <AssetImage
                type="weapon"
                item={displayedWeapon1}
                colorScheme={selectedColorScheme}
                ownerHero={displayedWeapon1?.ownerHero || ''}
                costumeName=""
                style={{ width: '100%', height: '100%' }}
                fallbackText={tWeapon(legend?.weapon1 || '')}
              />
            </div>
          </div>
          <div className="card-content">
            <div className="card-heading">
              <h3 className="product-title" title={displayedWeapon1?.displayName || displayedWeapon1?.weaponSkinName || ''}>
                {displayedWeapon1?.displayName || displayedWeapon1?.weaponSkinName || tWeapon(legend?.weapon1 || '')}
              </h3>
            </div>
            <p className="product-description">
              {isArabic ? `مظهر السلاح الأول (${tWeapon(legend?.weapon1 || '')})` : `Equipped skin for ${legend?.weapon1 || ''}`}
            </p>
          </div>
        </div>

        {/* Card 4: Weapon 2 Skin */}
        <div className="store-card">
          <div className="card-image-wrap">
            <div className={`card-image weapon-preview weapon-${(displayedWeapon2?.baseWeapon || 'generic').toLowerCase().replace(/[\s_]+/g, '')}`}>
              <AssetImage
                type="weapon"
                item={displayedWeapon2}
                colorScheme={selectedColorScheme}
                ownerHero={displayedWeapon2?.ownerHero || ''}
                costumeName=""
                style={{ width: '100%', height: '100%' }}
                fallbackText={tWeapon(legend?.weapon2 || '')}
              />
            </div>
          </div>
          <div className="card-content">
            <div className="card-heading">
              <h3 className="product-title" title={displayedWeapon2?.displayName || displayedWeapon2?.weaponSkinName || ''}>
                {displayedWeapon2?.displayName || displayedWeapon2?.weaponSkinName || tWeapon(legend?.weapon2 || '')}
              </h3>
            </div>
            <p className="product-description">
              {isArabic ? `مظهر السلاح الثاني (${tWeapon(legend?.weapon2 || '')})` : `Equipped skin for ${legend?.weapon2 || ''}`}
            </p>
          </div>
        </div>
      </div>

      {/* Configuration Controls Panel */}
      <div className="config-panel">
        <h3 style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '4px' }}>
          {isArabic ? 'خيارات التخصيص والأزياء' : 'Customization Configuration'}
        </h3>

        {/* Skin Character Selector (Cross-Legend) - Kept modular, currently locked to active studio legend */}
        {ENABLE_CROSS_LEGEND_PICKER && (
          <div className="config-field" style={{ marginBottom: '16px' }}>
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                <circle cx="9" cy="7" r="4" />
                <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                <path d="M16 3.13a4 4 0 0 1 0 7.75" />
              </svg>
              <span>{isArabic ? 'شخصية السكن (اختيار شخصية لاستعارة سكناتها):' : 'Skin Character (Borrow Skins From):'}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('replacementHero')}
              title={isArabic ? 'اضغط لاختيار شخصية السكن من البطاقات' : 'Click to select skin character from cards'}
            >
              <div className="card-selector-trigger-thumb">
                <AssetImage
                  type="legend"
                  item={selectedHeroObject || { displayName: currentSelectedHeroDisplayName, heroName: selectedReplacementHero }}
                  style={{ width: '100%', height: '100%' }}
                />
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {currentSelectedHeroDisplayName}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedReplacementHero === 'Random'
                    ? (isArabic ? 'استعارة سكنات عشوائية من كافة الأساطير' : 'Random skins from all legends')
                    : (isArabic ? `عرض سكنات ${currentSelectedHeroDisplayName} المتاحة للاستبدال` : `Showing skins from ${currentSelectedHeroDisplayName}`)}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
              </div>
            </div>
          </div>
        )}

        {/* Costumes Row: Target Costume & Replacement Costume in ONE horizontal level */}
        <div className="config-row">
          <div className="config-field">
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                <circle cx="12" cy="7" r="4" />
              </svg>
              <span>{isArabic ? 'الزي المستهدف في مخزونك (Target Costume)' : 'Target Costume in Inventory'}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('targetCostume')}
              title={isArabic ? 'اضغط لفتح واجهة البطاقات واختيار الزي' : 'Click to open visual cards picker'}
            >
              <div className="card-selector-trigger-thumb">
                <AssetImage type="costume" item={selectedTargetCostume} style={{ width: '100%', height: '100%' }} />
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {selectedTargetCostume?.displayName || selectedTargetCostume?.costumeName || legend?.displayName}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedTargetCostume?.isDefault ? (isArabic ? 'الزي الافتراضي (الأصلي)' : 'Default Costume') : (selectedTargetCostume?.ownerHero || '')}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
              </div>
            </div>
          </div>

          <div className="config-field">
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
              </svg>
              <span>{isArabic ? 'زي الاستبدال المرئي الجديد (Visual Skin)' : 'New Visual Replacement Skin'}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('replacementCostume')}
              title={isArabic ? 'اضغط لفتح واجهة البطاقات واختيار السكن' : 'Click to pick visual skin from cards'}
            >
              <div className="card-selector-trigger-thumb">
                {selectedReplacementCostume ? (
                  <AssetImage type="costume" item={selectedReplacementCostume} colorScheme={selectedColorScheme} style={{ width: '100%', height: '100%' }} />
                ) : (
                  <AssetImage type="costume" item={selectedTargetCostume} colorScheme={selectedColorScheme} style={{ width: '100%', height: '100%' }} />
                )}
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {selectedReplacementCostume
                    ? (selectedReplacementCostume.displayName || selectedReplacementCostume.costumeName)
                    : (isArabic ? 'بدون تغيير (الاحتفاظ بمظهر الزي المستهدف)' : 'Unchanged (Keep Target Look)')}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedReplacementCostume
                    ? `${selectedReplacementCostume.ownerHero || ''} ${selectedReplacementCostume.isDefault ? (isArabic ? '• الافتراضي' : '• Stock') : ''}`
                    : (isArabic ? 'المظهر الأصلي للزي المستهدف' : 'Stock target costume')}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
              </div>
            </div>
          </div>
        </div>

        {/* Weapons Row */}
        <div className="config-row">
          <div className="config-field">
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M14.5 17.5L3 6V3h3l11.5 11.5-3 3z" />
              </svg>
              <span>{isArabic ? `مظهر سلاح: ${tWeapon(legend.weapon1)} (خاص بـ ${tWeapon(legend.weapon1)} فقط)` : `Weapon Skin: ${tWeapon(legend.weapon1)} (${tWeapon(legend.weapon1)} Only)`}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('weapon1')}
              title={isArabic ? 'اضغط لفتح واجهة البطاقات واختيار مظهر السلاح' : 'Click to pick weapon skin from cards'}
            >
              <div className="card-selector-trigger-thumb">
                <div className={`weapon-preview weapon-${(legend?.weapon1 || '').toLowerCase().replace(/[\s_]+/g, '')}`} style={{ width: '100%', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <AssetImage type="weapon" item={selectedReplacementWeapon1 || selectedTargetWeapon1} colorScheme={selectedColorScheme} style={{ width: '100%', height: '100%' }} />
                </div>
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {selectedReplacementWeapon1
                    ? (selectedReplacementWeapon1.displayName || selectedReplacementWeapon1.weaponSkinName)
                    : (isArabic ? `الافتراضي لـ ${tWeapon(legend.weapon1)} (بدون تعديل)` : `Default ${tWeapon(legend.weapon1)} (Stock)`)}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedReplacementWeapon1?.ownerHero ? `${selectedReplacementWeapon1.ownerHero}` : (isArabic ? `${weapon1Skins.length} مظهر متاح للـ ${tWeapon(legend.weapon1)}` : `${weapon1Skins.length} skins available`)}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
              </div>
            </div>
          </div>

          <div className="config-field">
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M14.5 17.5L3 6V3h3l11.5 11.5-3 3z" />
              </svg>
              <span>{isArabic ? `مظهر سلاح: ${tWeapon(legend.weapon2)} (خاص بـ ${tWeapon(legend.weapon2)} فقط)` : `Weapon Skin: ${tWeapon(legend.weapon2)} (${tWeapon(legend.weapon2)} Only)`}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('weapon2')}
              title={isArabic ? 'اضغط لفتح واجهة البطاقات واختيار مظهر السلاح' : 'Click to pick weapon skin from cards'}
            >
              <div className="card-selector-trigger-thumb">
                <div className={`weapon-preview weapon-${(legend?.weapon2 || '').toLowerCase().replace(/[\s_]+/g, '')}`} style={{ width: '100%', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                  <AssetImage type="weapon" item={selectedReplacementWeapon2 || selectedTargetWeapon2} colorScheme={selectedColorScheme} style={{ width: '100%', height: '100%' }} />
                </div>
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {selectedReplacementWeapon2
                    ? (selectedReplacementWeapon2.displayName || selectedReplacementWeapon2.weaponSkinName)
                    : (isArabic ? `الافتراضي لـ ${tWeapon(legend.weapon2)} (بدون تعديل)` : `Default ${tWeapon(legend.weapon2)} (Stock)`)}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedReplacementWeapon2?.ownerHero ? `${selectedReplacementWeapon2.ownerHero}` : (isArabic ? `${weapon2Skins.length} مظهر متاح للـ ${tWeapon(legend.weapon2)}` : `${weapon2Skins.length} skins available`)}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
              </div>
            </div>
          </div>
        </div>

        {/* Color Scheme Row */}
        <div className="config-field">
          <label className="config-label">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="10" />
              <path d="M12 2a10 10 0 0 1 10 10c0 5.523-4.477 10-10 10" />
            </svg>
            <span>{isArabic ? 'نظام الألوان والصبغة (Color Palette)' : 'Color Scheme / Palette'}</span>
          </label>
          <div
            className="card-selector-trigger"
            onClick={() => setActivePicker('colorScheme')}
            title={isArabic ? 'اضغط لفتح واجهة البطاقات واختيار نظام الألوان' : 'Click to pick color palette from visual cards'}
          >
            <div className="card-selector-trigger-thumb" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <ColorEmblem colorData={selectedColorScheme} size={32} />
            </div>
            <div className="card-selector-trigger-info">
              <div className="card-selector-trigger-title">
                {selectedColorScheme ? (selectedColorScheme.displayName || selectedColorScheme.schemeName) : (isArabic ? 'ألوان الأسطورة الأصلية (بدون تعديل)' : 'Default Legend Colors')}
              </div>
              <div className="card-selector-trigger-sub">
                {selectedColorScheme ? (isArabic ? 'أيقونة اللعبة الرسمية • صبغة معتمدة' : 'Official in-game color palette') : (isArabic ? `${catalog?.colorSchemes?.length || 0} نظام ألوان متاح مع أيقونات اللعبة` : `${catalog?.colorSchemes?.length || 0} palettes available`)}
              </div>
            </div>
            <div className="card-selector-trigger-badge">
              <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="7" height="7" />
                <rect x="14" y="3" width="7" height="7" />
                <rect x="14" y="14" width="7" height="7" />
                <rect x="3" y="14" width="7" height="7" />
              </svg>
            </div>
          </div>
        </div>
      </div>

      {/* Action Footer Bar */}
      <div className="workbench-footer">
        <label className="checkbox-label" style={{ cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '8px' }}>
          <input
            type="checkbox"
            checked={autoRestart}
            onChange={(e) => setAutoRestart(e.target.checked)}
            style={{ accentColor: '#3b82f6', width: '16px', height: '16px', cursor: 'pointer' }}
          />
          <span>{isArabic ? 'إعادة تشغيل اللعبة تلقائياً لتطبيق المظهر فوراً' : 'Auto restart game instantly to apply visual change'}</span>
        </label>

        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <button
            className="button-secondary"
            onClick={onRestoreTargetCostume}
            disabled={isBusy}
            title={isArabic ? 'استعادة ملف الزي الحالي لأصله' : 'Restore target costume to original SWF'}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
            <span>{isArabic ? 'استعادة الأصلية' : 'Restore Original'}</span>
          </button>

          <button
            className="button-secondary"
            onClick={onRestartGame}
            disabled={isBusy}
            title={isArabic ? 'إغلاق اللعبة بالكامل من جذورها وتطبيق السكنات وإعادة تشغيلها' : 'Terminate game, apply queued modifications, and restart'}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.19"/>
            </svg>
            <span>{isArabic ? 'تحديث اللعبة' : 'Restart Game'}</span>
          </button>

          <button
            className="button-primary"
            onClick={handleApply}
            disabled={isBusy || !selectedTargetCostume}
          >
            <span>{isBusy ? (isArabic ? 'جاري التطبيق...' : 'Applying...') : (isArabic ? 'تطبيق التعديل على اللعبة' : 'Apply Modification to Game')}</span>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M7 17L17 7M17 7H7M17 7V17" />
            </svg>
          </button>
        </div>
      </div>

      {/* Visual Card Gallery Modals */}
      {ENABLE_CROSS_LEGEND_PICKER && activePicker === 'replacementHero' && (
        <CardPickerModal
          isOpen={true}
          onClose={() => setActivePicker(null)}
          title={isArabic ? 'اختيار شخصية السكن' : 'Select Skin Character'}
          subtitle={isArabic ? `اختر أسطورة لاستعارة سكناتها أو اختر Random (${heroCards.length} خيار متاح)` : `Pick a character to borrow skins from or choose Random (${heroCards.length} options)`}
          items={heroCards}
          selectedId={selectedReplacementHero}
          onSelect={handleSelectHeroFilter}
          type="hero"
        />
      )}

      {activePicker === 'targetCostume' && (
        <CardPickerModal
          isOpen={true}
          onClose={() => setActivePicker(null)}
          title={isArabic ? 'اختيار الزي المستهدف في المخزون' : 'Select Target Costume'}
          subtitle={isArabic ? `اختر الزي الذي تملكه لـ ${legend.displayName} وسيتم استبدال شكله` : `Costume in inventory to replace`}
          items={targetCostumeCards}
          selectedId={selectedTargetCostume?.costumeId || selectedTargetCostume?.costumeName}
          onSelect={handleSelectTargetCostume}
          type="costume"
        />
      )}

      {activePicker === 'replacementCostume' && (
        <CardPickerModal
          isOpen={true}
          onClose={() => setActivePicker(null)}
          title={isArabic ? 'اختيار زي ومظهر الاستبدال' : 'Select Visual Replacement Skin'}
          subtitle={
            selectedReplacementHero === 'Random'
              ? (isArabic ? 'مظاهر عشوائية من جميع الأساطير' : 'Random skins across all legends')
              : (isArabic ? `مظاهر أسطورة ${currentSelectedHeroDisplayName} (${replacementCostumeCards.length - 1} مظهر متاح)` : `Skins for ${currentSelectedHeroDisplayName} (${replacementCostumeCards.length - 1} skins)`)
          }
          items={replacementCostumeCards}
          selectedId={selectedReplacementCostume ? (selectedReplacementCostume.costumeId != null && selectedReplacementCostume.costumeId !== 0 ? selectedReplacementCostume.costumeId : selectedReplacementCostume.costumeName) : '__NONE__'}
          onSelect={handleSelectReplacementCostume}
          type="costume"
          colorScheme={selectedColorScheme}
        />
      )}

      {activePicker === 'colorScheme' && (
        <CardPickerModal
          isOpen={true}
          onClose={() => setActivePicker(null)}
          title={isArabic ? 'اختيار نظام الألوان والصبغة' : 'Select Color Scheme'}
          subtitle={isArabic ? `اختر الصبغة بأيقونتها الأصلية كما في اللعبة (${colorSchemeCards.length} نظام ألوان)` : `Choose palette with authentic in-game emblems (${colorSchemeCards.length} palettes)`}
          items={colorSchemeCards}
          selectedId={selectedColorScheme ? (selectedColorScheme.schemeId || selectedColorScheme.schemeName) : '__NONE__'}
          onSelect={handleSelectColorScheme}
          type="color"
        />
      )}

      {activePicker === 'weapon1' && (
        <CardPickerModal
          isOpen={true}
          onClose={() => setActivePicker(null)}
          title={isArabic ? `اختيار مظهر سلاح: ${tWeapon(legend.weapon1)}` : `Select ${tWeapon(legend.weapon1)} Skin`}
          subtitle={isArabic ? `قائمة حصرية لمظاهر سلاح الـ ${tWeapon(legend.weapon1)} فقط (${weapon1Skins.length} مظهر متاح)` : `${weapon1Skins.length} skins for ${tWeapon(legend.weapon1)} only`}
          items={weapon1Cards}
          selectedId={selectedReplacementWeapon1 ? (selectedReplacementWeapon1.weaponSkinId != null && selectedReplacementWeapon1.weaponSkinId !== 0 ? selectedReplacementWeapon1.weaponSkinId : selectedReplacementWeapon1.weaponSkinName) : '__NONE__'}
          onSelect={handleSelectWeapon1}
          type="weapon"
          baseWeapon={legend.weapon1}
          colorScheme={selectedColorScheme}
        />
      )}

      {activePicker === 'weapon2' && (
        <CardPickerModal
          isOpen={true}
          onClose={() => setActivePicker(null)}
          title={isArabic ? `اختيار مظهر سلاح: ${tWeapon(legend.weapon2)}` : `Select ${tWeapon(legend.weapon2)} Skin`}
          subtitle={isArabic ? `قائمة حصرية لمظاهر سلاح الـ ${tWeapon(legend.weapon2)} فقط (${weapon2Skins.length} مظهر متاح)` : `${weapon2Skins.length} skins for ${tWeapon(legend.weapon2)} only`}
          items={weapon2Cards}
          selectedId={selectedReplacementWeapon2 ? (selectedReplacementWeapon2.weaponSkinId != null && selectedReplacementWeapon2.weaponSkinId !== 0 ? selectedReplacementWeapon2.weaponSkinId : selectedReplacementWeapon2.weaponSkinName) : '__NONE__'}
          onSelect={handleSelectWeapon2}
          type="weapon"
          baseWeapon={legend.weapon2}
          colorScheme={selectedColorScheme}
        />
      )}
    </div>
  );
}
