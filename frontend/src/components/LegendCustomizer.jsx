import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import AssetImage from './AssetImage';
import CardPickerModal, { ColorEmblem } from './CardPickerModal';
import { api } from '../services/api';

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
  showToast,
  isBusy
}) {
  const { tWeapon, isRTL } = useLanguage();

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
    if (!c || !id || id === '__NONE__') return false;
    return String(c.costumeId) === String(id) || c.costumeName === String(id);
  };

  const matchWeapon = (w, id) => {
    if (!w || !id || id === '__NONE__') return false;
    return String(w.weaponSkinId) === String(id) ||
           w.weaponSkinName === String(id) ||
           w.displayName === String(id);
  };

  const matchColorScheme = (cs, id) => {
    if (!cs || !id || id === '__NONE__') return false;
    return String(cs.schemeId) === String(id) || cs.schemeName === String(id);
  };

  // Helper: Find matching weapon skin for a given costume (Strictly matches legend's weapon skins)
  const findMatchingWeaponSkin = (skins, costume) => {
    if (!skins || skins.length === 0 || !costume) return null;
    const wSet = (costume.weaponSet || '').trim().toLowerCase();
    const cName = (costume.costumeName || '').trim().toLowerCase();
    const dName = (costume.displayName || '').trim().toLowerCase();
    const oHero = (costume.ownerHero || '').trim().toLowerCase();

    // Verify owner hero does not conflict with another character
    const isValidOwner = (w) => {
      const wHero = (w?.ownerHero || '').trim().toLowerCase();
      return !wHero || !oHero || wHero === oHero;
    };

    // 1. Exact match with weaponSet (ends with weaponSet)
    if (wSet && wSet !== '--') {
      const exact = skins.find((w) => {
        if (!isValidOwner(w)) return false;
        const wName = (w.weaponSkinName || '').toLowerCase();
        const sSuffix = (w.symbolSuffix || '').toLowerCase();
        return wName.endsWith(wSet) || sSuffix === wSet;
      });
      if (exact) return exact;
    }

    // 2. Exact match with costumeName (ends with costumeName)
    if (cName) {
      const exact = skins.find((w) => {
        if (!isValidOwner(w)) return false;
        const wName = (w.weaponSkinName || '').toLowerCase();
        const sSuffix = (w.symbolSuffix || '').toLowerCase();
        return wName.endsWith(cName) || sSuffix === cName;
      });
      if (exact) return exact;
    }

    // 3. Substring match with weaponSet
    if (wSet && wSet !== '--') {
      const found = skins.find((w) => {
        if (!isValidOwner(w)) return false;
        const wName = (w.weaponSkinName || '').toLowerCase();
        return wName.includes(wSet);
      });
      if (found) return found;
    }

    // 4. Substring match with costumeName
    if (cName) {
      const found = skins.find((w) => {
        if (!isValidOwner(w)) return false;
        const wName = (w.weaponSkinName || '').toLowerCase();
        return wName.includes(cName);
      });
      if (found) return found;
    }

    // 5. Token-based match with displayName
    if (dName) {
      const found = skins.find((w) => {
        if (!isValidOwner(w)) return false;
        const wDisp = (w.displayName || '').toLowerCase();
        return wDisp === dName || wDisp.endsWith(' ' + dName) || wDisp.startsWith(dName + ' ');
      });
      if (found) return found;
    }

    // 6. Default weapon of owner hero
    if (costume.isDefault && oHero) {
      const found = skins.find((w) => (w.ownerHero || '').toLowerCase() === oHero && w.isDefault);
      if (found) return found;
    }

    return null;
  };

  // Target costume (costume to be replaced in user inventory, defaults to Classic/Default)
  const [selectedTargetCostume, setSelectedTargetCostume] = useState(() => {
    if (savedCustomization?.targetCostumeId) {
      const found = legendCostumes.find((c) => matchCostume(c, savedCustomization.targetCostumeId));
      if (found) return found;
    }
    return legendCostumes.find((c) => c.isDefault) || legendCostumes[0] || null;
  });

  // Replacement costume (costume whose visual look will be applied; supports cross-legend costumes)
  const initialReplacementCostume = useMemo(() => {
    if (savedCustomization?.replacementCostumeId === '__NONE__') return null;
    if (savedCustomization?.replacementCostumeId) {
      const found = allCatalogCostumes.find((c) => matchCostume(c, savedCustomization.replacementCostumeId))
        || legendCostumes.find((c) => matchCostume(c, savedCustomization.replacementCostumeId));
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
      displayName: isRTL ? `${tWeapon(legend?.weapon1 || '')} (الافتراضي)` : `${tWeapon(legend?.weapon1 || '')} (Stock)`,
      baseWeapon: legend?.weapon1 || '',
      ownerHero: legend?.heroName || '',
      symbolSuffix: legend?.heroName || '',
      isDefault: true
    };
  }, [weapon1Skins, legend, tWeapon, isRTL]);

  const defaultWeapon2 = useMemo(() => {
    const heroNameLower = (legend?.heroName || '').trim().toLowerCase();
    const found = weapon2Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === heroNameLower && w?.isDefault)
      || weapon2Skins.find((w) => (w?.ownerHero || '').trim().toLowerCase() === heroNameLower);
    return found || {
      weaponSkinId: 0,
      weaponSkinName: `${legend?.weapon2 || 'Weapon'}Default`,
      displayName: isRTL ? `${tWeapon(legend?.weapon2 || '')} (الافتراضي)` : `${tWeapon(legend?.weapon2 || '')} (Stock)`,
      baseWeapon: legend?.weapon2 || '',
      ownerHero: legend?.heroName || '',
      symbolSuffix: legend?.heroName || '',
      isDefault: true
    };
  }, [weapon2Skins, legend, tWeapon, isRTL]);

  const selectedTargetWeapon1 = defaultWeapon1;
  const [selectedReplacementWeapon1, setSelectedReplacementWeapon1] = useState(() => {
    if (savedCustomization?.weapon1Id === '__NONE__') return null;
    if (savedCustomization?.weapon1Id) {
      const found = weapon1Skins.find((w) => matchWeapon(w, savedCustomization.weapon1Id));
      if (found) return found;
    }
    if (initialReplacementCostume) {
      const autoMatch = findMatchingWeaponSkin(weapon1Skins, initialReplacementCostume);
      if (autoMatch) return autoMatch;
    }
    return null;
  });

  const selectedTargetWeapon2 = defaultWeapon2;
  const [selectedReplacementWeapon2, setSelectedReplacementWeapon2] = useState(() => {
    if (savedCustomization?.weapon2Id === '__NONE__') return null;
    if (savedCustomization?.weapon2Id) {
      const found = weapon2Skins.find((w) => matchWeapon(w, savedCustomization.weapon2Id));
      if (found) return found;
    }
    if (initialReplacementCostume) {
      const autoMatch = findMatchingWeaponSkin(weapon2Skins, initialReplacementCostume);
      if (autoMatch) return autoMatch;
    }
    return null;
  });

  const [selectedColorScheme, setSelectedColorScheme] = useState(() => {
    if (savedCustomization?.colorSchemeId === '__NONE__') return null;
    if (savedCustomization?.colorSchemeId) {
      const found = catalog?.colorSchemes?.find((cs) => matchColorScheme(cs, savedCustomization.colorSchemeId));
      if (found) return found;
    }
    return null;
  });

  const [autoRestart, setAutoRestart] = useState(() => {
    return savedCustomization?.autoRestart !== undefined ? savedCustomization.autoRestart : false;
  });

  // Character/legend selected as source for replacement visual skin ('Random' | heroName)
  const [selectedReplacementHero, setSelectedReplacementHero] = useState(() => {
    if (savedCustomization?.replacementHero) {
      return savedCustomization.replacementHero;
    }
    if (initialReplacementCostume?.ownerHero) {
      return initialReplacementCostume.ownerHero;
    }
    return legend?.heroName || (catalog?.legends?.[0]?.heroName || '');
  });

  // Visual Character Cards for Cross-Legend selection
  const heroCards = useMemo(() => {
    const currentHeroLower = (legend?.heroName || '').trim().toLowerCase();

    return (catalog?.legends || [])
      .map((l) => {
        const isCurrent = (l.heroName || '').toLowerCase() === currentHeroLower;
        const w1 = tWeapon(l.weapon1 || '');
        const w2 = tWeapon(l.weapon2 || '');
        const weaponsSubtitle = l.weapon1 && l.weapon2 ? `${w1} • ${w2}` : (l.bio || '');

        return {
          id: l.heroName,
          title: l.displayName || l.heroName,
          subtitle: isCurrent
            ? `${weaponsSubtitle} (${isRTL ? 'الأسطورة الحالية' : 'Current'})`
            : weaponsSubtitle,
          badge: isCurrent ? (isRTL ? 'الحالية' : 'Current') : null,
          rawItem: l,
          searchKey: `${l.displayName || ''} ${l.heroName || ''} ${l.weapon1 || ''} ${l.weapon2 || ''} ${w1} ${w2}`
        };
      })
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));
  }, [catalog?.legends, legend, tWeapon, isRTL]);

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
      const w1 = findMatchingWeaponSkin(weapon1Skins, newCostume);
      const w2 = findMatchingWeaponSkin(weapon2Skins, newCostume);
      if (w1) setSelectedReplacementWeapon1(w1);
      if (w2) setSelectedReplacementWeapon2(w2);
    }
  }, [allCatalogCostumes, weapon1Skins, weapon2Skins]);

  const [isValidating, setIsValidating] = useState(false);
  const [compatibility, setCompatibility] = useState(null);
  const [activePicker, setActivePicker] = useState(null);

  // Sync state to parent for persistence whenever any option changes
  const isFirstMount = useRef(true);
  useEffect(() => {
    if (isFirstMount.current) {
      isFirstMount.current = false;
      return;
    }

    onSaveCustomization?.({
      targetCostumeId: selectedTargetCostume?.costumeId ?? selectedTargetCostume?.costumeName,
      replacementHero: selectedReplacementHero,
      replacementCostumeId: selectedReplacementCostume ? (selectedReplacementCostume.costumeId ?? selectedReplacementCostume.costumeName) : '__NONE__',
      weapon1Id: selectedReplacementWeapon1 ? (selectedReplacementWeapon1.weaponSkinId ?? selectedReplacementWeapon1.weaponSkinName) : '__NONE__',
      weapon2Id: selectedReplacementWeapon2 ? (selectedReplacementWeapon2.weaponSkinId ?? selectedReplacementWeapon2.weaponSkinName) : '__NONE__',
      colorSchemeId: selectedColorScheme ? (selectedColorScheme.schemeId ?? selectedColorScheme.schemeName) : '__NONE__',
      autoRestart
    });
  }, [
    selectedTargetCostume,
    selectedReplacementHero,
    selectedReplacementCostume,
    selectedReplacementWeapon1,
    selectedReplacementWeapon2,
    selectedColorScheme,
    autoRestart,
    onSaveCustomization,
    legend?.heroName
  ]);

  const handleBack = () => {
    onSaveCustomization?.({
      targetCostumeId: selectedTargetCostume?.costumeId ?? selectedTargetCostume?.costumeName,
      replacementHero: selectedReplacementHero,
      replacementCostumeId: selectedReplacementCostume ? (selectedReplacementCostume.costumeId ?? selectedReplacementCostume.costumeName) : '__NONE__',
      weapon1Id: selectedReplacementWeapon1 ? (selectedReplacementWeapon1.weaponSkinId ?? selectedReplacementWeapon1.weaponSkinName) : '__NONE__',
      weapon2Id: selectedReplacementWeapon2 ? (selectedReplacementWeapon2.weaponSkinId ?? selectedReplacementWeapon2.weaponSkinName) : '__NONE__',
      colorSchemeId: selectedColorScheme ? (selectedColorScheme.schemeId ?? selectedColorScheme.schemeName) : '__NONE__',
      autoRestart
    });
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
    selectedReplacementWeapon2,
    selectedColorScheme
  ]);

  const legWeapon1 = legend?.weapon1 || '';
  const legWeapon2 = legend?.weapon2 || '';

  // Color scheme cards with authentic in-game color emblems (Sorted A to Z)
  const colorSchemeCards = useMemo(() => {
    const noneCard = {
      id: '__NONE__',
      title: isRTL ? 'ألوان الأسطورة الأصلية (بدون تعديل)' : 'Default Legend Colors (Stock)',
      subtitle: isRTL ? 'الصبغة الأساسية للأصل' : 'Stock Legend Palette',
      isNone: true
    };

    const rest = (catalog?.colorSchemes || [])
      .map((cs) => ({
        id: cs?.schemeId ?? cs?.schemeName ?? '__SCHEME__',
        title: cs?.displayName || cs?.schemeName || 'Color',
        subtitle: cs?.rarity ? `Rarity: ${cs.rarity}` : (isRTL ? 'نظام ألوان' : 'Color Scheme'),
        badge: cs?.displayName === 'Community Colors' ? 'Special' : null,
        rawItem: cs,
        searchKey: `${cs?.displayName || ''} ${cs?.schemeName || ''}`
      }))
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));

    return [noneCard, ...rest];
  }, [catalog?.colorSchemes, isRTL]);

  // --- Visual Card Gallery Items for Card Picker Modals ---
  const targetCostumeCards = useMemo(() => {
    return (legendCostumes || [])
      .map((c) => ({
        id: c.costumeId || c.costumeName,
        title: c.displayName || c.costumeName,
        subtitle: c.isDefault ? (isRTL ? 'الزي الافتراضي للأصل' : 'Default Costume') : (c.ownerHero || ''),
        badge: c.isDefault ? (isRTL ? 'افتراضي' : 'Default') : (c.isCrossover ? 'Crossover' : null),
        rawItem: c,
        searchKey: `${c.displayName || ''} ${c.costumeName || ''} ${c.ownerHero || ''}`
      }))
      .sort((a, b) => (a.title || '').localeCompare(b.title || ''));
  }, [legendCostumes, isRTL]);

  const legDisplayName = legend?.displayName || '';
  const replacementCostumeCards = useMemo(() => {
    const noneCard = {
      id: '__NONE__',
      title: isRTL ? 'بدون تغيير (الاحتفاظ بمظهر الزي المستهدف)' : 'Unchanged (Keep Target Look)',
      subtitle: isRTL ? 'لا يتم تعديل مظهر الزي' : 'Stock Target Visual',
      isNone: true,
      ownerHero: legend?.heroName || ''
    };

    // Hero name to display name mapping
    const heroMap = new Map();
    (catalog?.legends || []).forEach((l) => {
      if (l.heroName) heroMap.set(l.heroName.toLowerCase(), l.displayName || l.heroName);
    });

    const currentHeroLower = (legend?.heroName || '').trim().toLowerCase();
    const selHeroLower = (selectedReplacementHero || '').trim().toLowerCase();

    // Filter by selected hero if a specific hero is selected
    let sourceCostumes = allCatalogCostumes || [];
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
        subtitle = isRTL ? `${heroDisp} (الافتراضي)` : `${heroDisp} (Default / Stock)`;
      }

      let badge = null;
      if (c.isDefault) {
        badge = isRTL ? 'افتراضي' : 'Default';
      } else if (c.isCrossover) {
        badge = 'Crossover';
      } else if (!isCurrentLegend) {
        badge = heroDisp;
      }

      return {
        id: c.costumeId != null && c.costumeId !== 0 ? c.costumeId : c.costumeName,
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
  }, [allCatalogCostumes, catalog?.legends, legend, legDisplayName, selectedReplacementHero, isRTL]);

  const weapon1Cards = useMemo(() => {
    const weaponLabel = tWeapon(legWeapon1);
    const noneCard = {
      id: '__NONE__',
      title: isRTL ? `الافتراضي لـ ${weaponLabel} (بدون تعديل)` : `Default ${weaponLabel} (Stock)`,
      subtitle: isRTL ? 'المظهر الأصلي للسلاح' : 'Stock Weapon Skin',
      isNone: true
    };
    const skins = (weapon1Skins || [])
      .map((w) => {
        const ownerLegend = catalog?.legends?.find(
          (l) => (l?.heroName || '').trim().toLowerCase() === (w?.ownerHero || '').trim().toLowerCase()
        );
        const ownerDisp = ownerLegend ? ownerLegend.displayName : (w?.ownerHero ? w.ownerHero : (isRTL ? 'عام' : 'Universal'));
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
  }, [weapon1Skins, legWeapon1, isRTL, tWeapon, catalog]);

  const weapon2Cards = useMemo(() => {
    const weaponLabel = tWeapon(legWeapon2);
    const noneCard = {
      id: '__NONE__',
      title: isRTL ? `الافتراضي لـ ${weaponLabel} (بدون تعديل)` : `Default ${weaponLabel} (Stock)`,
      subtitle: isRTL ? 'المظهر الأصلي للسلاح' : 'Stock Weapon Skin',
      isNone: true
    };
    const skins = (weapon2Skins || [])
      .map((w) => {
        const ownerLegend = catalog?.legends?.find(
          (l) => (l?.heroName || '').trim().toLowerCase() === (w?.ownerHero || '').trim().toLowerCase()
        );
        const ownerDisp = ownerLegend ? ownerLegend.displayName : (w?.ownerHero ? w.ownerHero : (isRTL ? 'عام' : 'Universal'));
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
  }, [weapon2Skins, legWeapon2, isRTL, tWeapon, catalog]);

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
    if (val === '__RANDOM_ROLL__') {
      handleRollRandom();
      return;
    }
    const found = allCatalogCostumes.find((c) => matchCostume(c, val))
      || legendCostumes.find((c) => matchCostume(c, val));
    setSelectedReplacementCostume(found || null);

    if (found) {
      if (found.ownerHero && selectedReplacementHero !== 'Random' && selectedReplacementHero !== 'all') {
        setSelectedReplacementHero(found.ownerHero);
      }
      const w1 = findMatchingWeaponSkin(weapon1Skins, found);
      const w2 = findMatchingWeaponSkin(weapon2Skins, found);
      if (w1) setSelectedReplacementWeapon1(w1);
      if (w2) setSelectedReplacementWeapon2(w2);
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
      showToast(isRTL ? 'يرجى تحديد الزي المستهدف.' : 'Target costume is required.', 'error');
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
      autoRestart: autoRestart && systemInfo?.isGameRunning
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
            <span>{isRTL ? 'العودة إلى قائمة الأساطير' : 'Back to Legends'}</span>
          </button>

          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <h2 style={{ fontSize: '18px', fontWeight: 800 }}>{legend.displayName}</h2>
              <span className="product-badge" style={{ position: 'static' }}>
                {tWeapon(legend.weapon1)} • {tWeapon(legend.weapon2)}
              </span>
            </div>
            <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '2px' }}>
              {isRTL
                ? `تخصيص المظهر والأسلحة لـ ${legend.displayName} (${legend.heroName})`
                : `Customizing appearance & weaponry for ${legend.displayName} (${legend.heroName})`}
            </p>
          </div>
        </div>

        {/* Live Compatibility Status Pill */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {isValidating ? (
            <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
              {isRTL ? 'جاري التحقق...' : 'Validating...'}
            </span>
          ) : compatibility?.isCompatible ? (
            <span style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              padding: '5px 12px',
              borderRadius: '9999px',
              background: 'var(--success-soft)',
              color: 'var(--success)',
              fontSize: '12px',
              fontWeight: 600,
              border: '1px solid rgba(34, 197, 94, 0.3)'
            }}>
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2.5">
                <path d="M20 6L9 17l-5-5" />
              </svg>
              <span>
                {compatibility?.level === 'CrossLegendExperimental'
                  ? (isRTL ? 'توافق أزياء متقاطع (Cross-Legend)' : 'Cross-Legend Compatible')
                  : (isRTL ? 'جاهز ومتوافق ١٠٠٪' : '100% Compatible')}
              </span>
            </span>
          ) : (
            <span style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              padding: '5px 12px',
              borderRadius: '9999px',
              background: 'var(--warning-soft)',
              color: 'var(--warning)',
              fontSize: '12px',
              fontWeight: 600
            }}>
              <span>{isRTL ? 'غير متوافق' : 'Incompatible'}</span>
            </span>
          )}
        </div>
      </div>

      {/* 4 Authentic Showcase Cards */}
      <div className="workbench-cards-grid">
        {/* Card 1: Target Costume */}
        <div className="store-card">
          <div className="card-image-wrap">
            <div className="card-image">
              <AssetImage
                type="costume"
                item={displayedTargetCostume}
                style={{ width: '100%', height: '100%' }}
                fallbackText={displayedTargetCostume?.displayName || displayedTargetCostume?.costumeName || legend?.displayName || 'Target Costume'}
              />
            </div>
            <span className="product-badge default-badge">
              {isRTL ? 'في المخزون' : 'Target'}
            </span>
          </div>
          <div className="card-content">
            <div className="card-heading">
              <h3 className="product-title" title={displayedTargetCostume?.displayName || displayedTargetCostume?.costumeName || ''}>
                {displayedTargetCostume?.displayName || displayedTargetCostume?.costumeName || legend?.displayName || 'Costume'}
              </h3>
            </div>
            <p className="product-description">
              {isRTL ? 'الزي الذي تملكه وسيتم استبدال شكله في اللعبة' : 'Costume in your game inventory to be cosmetically replaced.'}
            </p>
          </div>
        </div>

        {/* Card 2: Replacement Visual Skin */}
        <div className="store-card" style={{ borderColor: 'var(--accent-border)' }}>
          <div className="card-image-wrap">
            <div className="card-image">
              <AssetImage
                type="costume"
                item={displayedReplacementCostume}
                colorScheme={selectedColorScheme}
                style={{ width: '100%', height: '100%' }}
                fallbackText={displayedReplacementCostume?.displayName || displayedReplacementCostume?.costumeName || 'Skin Preview'}
              />
            </div>
            <span className="product-badge">
              {displayedReplacementCostume?.ownerHero && displayedReplacementCostume.ownerHero.toLowerCase() !== (legend?.heroName || '').toLowerCase()
                ? (isRTL ? `مظهر من ${displayedReplacementCostume.ownerHero}` : `From ${displayedReplacementCostume.ownerHero}`)
                : (isRTL ? 'المظهر الجديد' : 'New Visual')}
            </span>
          </div>
          <div className="card-content">
            <div className="card-heading">
              <h3 className="product-title" title={displayedReplacementCostume?.displayName || displayedReplacementCostume?.costumeName || ''}>
                {displayedReplacementCostume?.displayName || displayedReplacementCostume?.costumeName || (isRTL ? 'نفس الزي' : 'Same Costume')}
              </h3>
            </div>
            <p className="product-description">
              {displayedReplacementCostume?.ownerHero && displayedReplacementCostume.ownerHero.toLowerCase() !== (legend?.heroName || '').toLowerCase()
                ? (isRTL
                    ? `مظهر منسوخ من ${displayedReplacementCostume.ownerHero} • ${displayedReplacementCostume.isDefault ? 'الزي الافتراضي' : 'سكن'}`
                    : `Cross-Legend skin from ${displayedReplacementCostume.ownerHero} • ${displayedReplacementCostume.isDefault ? 'Stock' : 'Skin'}`)
                : (isRTL ? 'المظهر الذي سيظهر في اللعبة أثناء المعارك' : 'The visual look you will see during combat and animations.')}
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
              {isRTL ? `مظهر السلاح الأول (${tWeapon(legend?.weapon1 || '')})` : `Equipped skin for ${legend?.weapon1 || ''}`}
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
              {isRTL ? `مظهر السلاح الثاني (${tWeapon(legend?.weapon2 || '')})` : `Equipped skin for ${legend?.weapon2 || ''}`}
            </p>
          </div>
        </div>
      </div>

      {/* Configuration Controls Panel */}
      <div className="config-panel">
        <h3 style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '4px' }}>
          {isRTL ? 'خيارات التخصيص والأزياء' : 'Customization Configuration'}
        </h3>

        {/* Costumes Row: Target Costume & New Visual Skin */}
        <div className="config-row">
          <div className="config-field">
            {/* Target Legend info card directly above target costume to keep layout perfectly balanced */}
            <div className="target-legend-info-section" style={{ marginBottom: '8px' }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                <label className="config-label" style={{ marginBottom: 0 }}>
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                    <circle cx="12" cy="12" r="10" />
                    <polyline points="12 6 12 12 16 14" />
                  </svg>
                  <span>{isRTL ? 'الأسطورة الحالية في اللعبة:' : 'Current Legend in Game:'}</span>
                </label>
              </div>
              <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                padding: '8px 12px',
                background: 'var(--bg-input)',
                border: '1px solid var(--border-primary)',
                borderRadius: 'var(--radius-md)',
                fontSize: '13px',
                color: 'var(--text-primary)',
                fontWeight: 600,
                minHeight: '38px'
              }}>
                <span style={{ width: '8px', height: '8px', borderRadius: '50%', background: 'var(--accent)', display: 'inline-block' }} />
                <span>{legend?.displayName || legend?.heroName}</span>
                <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginInlineStart: 'auto', fontWeight: 500 }}>
                  ({isRTL ? 'المستهدفة' : 'Target'})
                </span>
              </div>
            </div>

            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                <circle cx="12" cy="7" r="4" />
              </svg>
              <span>{isRTL ? 'الزي المستهدف في مخزونك (Target Costume)' : 'Target Costume in Inventory'}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('targetCostume')}
              title={isRTL ? 'اضغط لفتح واجهة البطاقات واختيار الزي' : 'Click to open visual cards picker'}
            >
              <div className="card-selector-trigger-thumb">
                <AssetImage type="costume" item={selectedTargetCostume} style={{ width: '100%', height: '100%' }} />
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {selectedTargetCostume?.displayName || selectedTargetCostume?.costumeName || legend?.displayName}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedTargetCostume?.isDefault ? (isRTL ? 'الزي الافتراضي (الأصلي)' : 'Default Costume') : (selectedTargetCostume?.ownerHero || '')}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
                <span>{isRTL ? 'اختيار من البطاقات' : 'Browse Cards'}</span>
              </div>
            </div>
          </div>

          <div className="config-field">
            {/* Character selection placed ABOVE "New Visual Replacement Skin" */}
            <div className="replacement-hero-selector-section" style={{ marginBottom: '8px' }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                <label className="config-label" style={{ marginBottom: 0 }}>
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                    <circle cx="12" cy="7" r="4" />
                  </svg>
                  <span>{isRTL ? 'اختيار شخصية السكن:' : 'Skin Character / Legend:'}</span>
                </label>
              </div>

              {/* Visual Card Trigger for Character Selection */}
              <div
                className="card-selector-trigger"
                onClick={() => setActivePicker('replacementHero')}
                title={isRTL ? 'اضغط لاختيار شخصية السكن من البطاقات' : 'Click to select skin character from cards'}
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
                    {isRTL ? `استعارة سكنات من ${currentSelectedHeroDisplayName}` : `Borrow skins from ${currentSelectedHeroDisplayName}`}
                  </div>
                </div>
                <div className="card-selector-trigger-badge">
                  <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                    <rect x="3" y="3" width="7" height="7" />
                    <rect x="14" y="3" width="7" height="7" />
                    <rect x="14" y="14" width="7" height="7" />
                    <rect x="3" y="14" width="7" height="7" />
                  </svg>
                  <span>{isRTL ? 'اختيار من البطاقات' : 'Browse Cards'}</span>
                </div>
              </div>
            </div>

            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
              </svg>
              <span>{isRTL ? 'زي الاستبدال المرئي الجديد (Visual Skin)' : 'New Visual Replacement Skin'}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('replacementCostume')}
              title={isRTL ? 'اضغط لفتح واجهة البطاقات واختيار السكن' : 'Click to pick visual skin from cards'}
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
                    : (isRTL ? 'بدون تغيير (الاحتفاظ بمظهر الزي المستهدف)' : 'Unchanged (Keep Target Look)')}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedReplacementCostume
                    ? `${selectedReplacementCostume.ownerHero || ''} ${selectedReplacementCostume.isDefault ? (isRTL ? '• الافتراضي' : '• Stock') : ''}`
                    : (isRTL ? 'المظهر الأصلي للزي المستهدف' : 'Stock target costume')}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
                <span>{isRTL ? 'اختيار من البطاقات' : 'Browse Cards'}</span>
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
              <span>{isRTL ? `مظهر سلاح: ${tWeapon(legend.weapon1)} (خاص بـ ${tWeapon(legend.weapon1)} فقط)` : `Weapon Skin: ${tWeapon(legend.weapon1)} (${tWeapon(legend.weapon1)} Only)`}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('weapon1')}
              title={isRTL ? 'اضغط لفتح واجهة البطاقات واختيار مظهر السلاح' : 'Click to pick weapon skin from cards'}
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
                    : (isRTL ? `الافتراضي لـ ${tWeapon(legend.weapon1)} (بدون تعديل)` : `Default ${tWeapon(legend.weapon1)} (Stock)`)}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedReplacementWeapon1?.ownerHero ? `${selectedReplacementWeapon1.ownerHero}` : (isRTL ? `${weapon1Skins.length} مظهر متاح للـ ${tWeapon(legend.weapon1)}` : `${weapon1Skins.length} skins available`)}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
                <span>{isRTL ? 'اختيار من البطاقات' : 'Browse Cards'}</span>
              </div>
            </div>
          </div>

          <div className="config-field">
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M14.5 17.5L3 6V3h3l11.5 11.5-3 3z" />
              </svg>
              <span>{isRTL ? `مظهر سلاح: ${tWeapon(legend.weapon2)} (خاص بـ ${tWeapon(legend.weapon2)} فقط)` : `Weapon Skin: ${tWeapon(legend.weapon2)} (${tWeapon(legend.weapon2)} Only)`}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setActivePicker('weapon2')}
              title={isRTL ? 'اضغط لفتح واجهة البطاقات واختيار مظهر السلاح' : 'Click to pick weapon skin from cards'}
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
                    : (isRTL ? `الافتراضي لـ ${tWeapon(legend.weapon2)} (بدون تعديل)` : `Default ${tWeapon(legend.weapon2)} (Stock)`)}
                </div>
                <div className="card-selector-trigger-sub">
                  {selectedReplacementWeapon2?.ownerHero ? `${selectedReplacementWeapon2.ownerHero}` : (isRTL ? `${weapon2Skins.length} مظهر متاح للـ ${tWeapon(legend.weapon2)}` : `${weapon2Skins.length} skins available`)}
                </div>
              </div>
              <div className="card-selector-trigger-badge">
                <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7" />
                  <rect x="14" y="3" width="7" height="7" />
                  <rect x="14" y="14" width="7" height="7" />
                  <rect x="3" y="14" width="7" height="7" />
                </svg>
                <span>{isRTL ? 'اختيار من البطاقات' : 'Browse Cards'}</span>
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
            <span>{isRTL ? 'نظام الألوان والصبغة (Color Palette)' : 'Color Scheme / Palette'}</span>
          </label>
          <div
            className="card-selector-trigger"
            onClick={() => setActivePicker('colorScheme')}
            title={isRTL ? 'اضغط لفتح واجهة البطاقات واختيار نظام الألوان' : 'Click to pick color palette from visual cards'}
          >
            <div className="card-selector-trigger-thumb" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <ColorEmblem colorData={selectedColorScheme} size={32} />
            </div>
            <div className="card-selector-trigger-info">
              <div className="card-selector-trigger-title">
                {selectedColorScheme ? (selectedColorScheme.displayName || selectedColorScheme.schemeName) : (isRTL ? 'ألوان الأسطورة الأصلية (بدون تعديل)' : 'Default Legend Colors')}
              </div>
              <div className="card-selector-trigger-sub">
                {selectedColorScheme ? (isRTL ? 'أيقونة اللعبة الرسمية • صبغة معتمدة' : 'Official in-game color palette') : (isRTL ? `${catalog?.colorSchemes?.length || 0} نظام ألوان متاح مع أيقونات اللعبة` : `${catalog?.colorSchemes?.length || 0} palettes available`)}
              </div>
            </div>
            <div className="card-selector-trigger-badge">
              <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="7" height="7" />
                <rect x="14" y="3" width="7" height="7" />
                <rect x="14" y="14" width="7" height="7" />
                <rect x="3" y="14" width="7" height="7" />
              </svg>
              <span>{isRTL ? 'اختيار من البطاقات' : 'Browse Colors'}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Action Footer Bar */}
      <div className="workbench-footer">
        <div style={{ display: 'flex', flexDirection: 'column', gap: '3px' }}>
          <label className="checkbox-label" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <input
              type="checkbox"
              checked={autoRestart}
              onChange={(e) => setAutoRestart(e.target.checked)}
            />
            <span style={{ fontWeight: autoRestart ? '500' : 'normal' }}>
              {isRTL ? 'إعادة تشغيل اللعبة تلقائياً عند التطبيق' : 'Auto restart game on apply'}
            </span>
          </label>
          {systemInfo?.isGameRunning && !autoRestart && (
            <span style={{ fontSize: '11px', color: '#10b981', display: 'flex', alignItems: 'center', gap: '4px' }}>
              ⚡ {isRTL ? 'التبديل الفوري مفعّل: التعديل يطبق واللعبة شغالة بدون إعادة تشغيل' : 'Live Hot-Swap: applies instantly while game is running without restarting'}
            </span>
          )}
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <button
            className="button-secondary"
            onClick={onRestoreTargetCostume}
            disabled={isBusy}
            title={isRTL ? 'استعادة ملف الزي الحالي لأصله' : 'Restore target costume to original SWF'}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
            <span>{isRTL ? 'استعادة الأصلية' : 'Restore Original'}</span>
          </button>

          {systemInfo?.isGameRunning && (
            <button
              className="button-secondary"
              onClick={onRestartGame}
              disabled={isBusy}
              title={isRTL ? 'إعادة تشغيل اللعبة فوراً لتحديث المتجهات' : 'Instantly restart Brawlhalla'}
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.19"/>
              </svg>
              <span>{isRTL ? 'تحديث اللعبة' : 'Restart Game'}</span>
            </button>
          )}

          <button
            className="button-primary"
            onClick={handleApply}
            disabled={isBusy || !selectedTargetCostume}
          >
            <span>{isBusy ? (isRTL ? 'جاري التطبيق...' : 'Applying...') : (isRTL ? 'تطبيق التعديل على اللعبة' : 'Apply Modification to Game')}</span>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M7 17L17 7M17 7H7M17 7V17" />
            </svg>
          </button>
        </div>
      </div>

      {/* Visual Card Gallery Modals */}
      {activePicker === 'replacementHero' && (
        <CardPickerModal
          isOpen={true}
          onClose={() => setActivePicker(null)}
          title={isRTL ? 'اختيار شخصية السكن' : 'Select Skin Character'}
          subtitle={isRTL ? `اختر أسطورة لاستعارة سكناتها أو اختر Random (${heroCards.length} خيار متاح)` : `Pick a character to borrow skins from or choose Random (${heroCards.length} options)`}
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
          title={isRTL ? 'اختيار الزي المستهدف في المخزون' : 'Select Target Costume'}
          subtitle={isRTL ? `اختر الزي الذي تملكه لـ ${legend.displayName} وسيتم استبدال شكله` : `Costume in inventory to replace`}
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
          title={isRTL ? 'اختيار زي ومظهر الاستبدال' : 'Select Visual Replacement Skin'}
          subtitle={
            selectedReplacementHero === 'Random'
              ? (isRTL ? 'مظاهر عشوائية من جميع الأساطير' : 'Random skins across all legends')
              : (isRTL ? `مظاهر أسطورة ${currentSelectedHeroDisplayName} (${replacementCostumeCards.length - 1} مظهر متاح)` : `Skins for ${currentSelectedHeroDisplayName} (${replacementCostumeCards.length - 1} skins)`)
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
          title={isRTL ? 'اختيار نظام الألوان والصبغة' : 'Select Color Scheme'}
          subtitle={isRTL ? `اختر الصبغة بأيقونتها الأصلية كما في اللعبة (${colorSchemeCards.length} نظام ألوان)` : `Choose palette with authentic in-game emblems (${colorSchemeCards.length} palettes)`}
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
          title={isRTL ? `اختيار مظهر سلاح: ${tWeapon(legend.weapon1)}` : `Select ${tWeapon(legend.weapon1)} Skin`}
          subtitle={isRTL ? `قائمة حصرية لمظاهر سلاح الـ ${tWeapon(legend.weapon1)} فقط (${weapon1Skins.length} مظهر متاح)` : `${weapon1Skins.length} skins for ${tWeapon(legend.weapon1)} only`}
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
          title={isRTL ? `اختيار مظهر سلاح: ${tWeapon(legend.weapon2)}` : `Select ${tWeapon(legend.weapon2)} Skin`}
          subtitle={isRTL ? `قائمة حصرية لمظاهر سلاح الـ ${tWeapon(legend.weapon2)} فقط (${weapon2Skins.length} مظهر متاح)` : `${weapon2Skins.length} skins for ${tWeapon(legend.weapon2)} only`}
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
