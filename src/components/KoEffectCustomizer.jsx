import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import CardPickerModal from './CardPickerModal';
import AssetImage from './AssetImage';
import { api } from '../services/api';

/**
 * KoEffectCustomizer - Studio for KO Effects.
 * Matches LegendCustomizer design, card layout, and button styling 1:1.
 * Strict Rule: Target (A) and Replacement (B) must never be the same effect.
 */
export default function KoEffectCustomizer({
  koEffect,
  catalog,
  systemInfo,
  onApplyModification,
  onBack,
  onRestartGame,
  onLaunchGame,
  showToast,
  isBusy
}) {
  const { isArabic } = useLanguage();

  const allKoEffects = useMemo(() => {
    return catalog?.koEffects || [];
  }, [catalog?.koEffects]);

  // Default / Stock KO Effect in Brawlhalla
  const defaultKoEffect = useMemo(() => {
    return allKoEffects.find((k) => k.isDefault) || allKoEffects[0] || null;
  }, [allKoEffects]);

  // First non-default KO effect in catalog as an alternative
  const firstNonDefaultKoEffect = useMemo(() => {
    return allKoEffects.find((k) => !k.isDefault) || allKoEffects[1] || allKoEffects[0] || null;
  }, [allKoEffects]);

  // The exact KO effect selected by the user to be customized (Target A)
  const currentTarget = useMemo(() => {
    return koEffect || defaultKoEffect;
  }, [koEffect, defaultKoEffect]);

  // Persistent customizations map per target KO effect
  const [koCustomizations, setKoCustomizations] = useState(() => {
    try {
      const raw = localStorage.getItem('brawlcraft_ko_customizations');
      if (raw) return JSON.parse(raw);
      // Migrate legacy single customization if present
      const old = localStorage.getItem('brawlcraft_ko_effect_customization');
      if (old) {
        const parsed = JSON.parse(old);
        if (parsed?.targetKoEffectName) {
          return { [parsed.targetKoEffectName]: parsed };
        }
      }
    } catch {}
    return {};
  });

  // Helper to look up saved replacement for target, strictly ensuring A !== B
  const getSavedReplacement = useCallback(
    (target) => {
      if (!target) return null;
      const key = target.koEffectName || target.displayName;
      const saved = koCustomizations[key];
      if (saved?.replacementKoEffectName && saved.replacementKoEffectName !== key) {
        const found = allKoEffects.find(
          (k) =>
            (k.koEffectName === saved.replacementKoEffectName ||
              k.displayName === saved.replacementKoEffectName ||
              (k.koEffectId && String(k.koEffectId) === String(saved.replacementKoEffectName))) &&
            k.koEffectName !== target.koEffectName &&
            k.displayName !== target.displayName
        );
        if (found) return found;
      }

      // A cannot be B! If target is default, pick the first non-default; if target is non-default, pick default!
      if (target.isDefault || target.koEffectName === defaultKoEffect?.koEffectName) {
        return firstNonDefaultKoEffect;
      } else {
        return defaultKoEffect;
      }
    },
    [koCustomizations, allKoEffects, defaultKoEffect, firstNonDefaultKoEffect]
  );

  // Desired Replacement KO Effect (Selected visual + sound - strictly different from A)
  const [replacementKoEffect, setReplacementKoEffect] = useState(() => {
    return getSavedReplacement(currentTarget);
  });

  // Synchronize replacement when currentTarget changes or saved customizations update
  useEffect(() => {
    setReplacementKoEffect(getSavedReplacement(currentTarget));
  }, [currentTarget, getSavedReplacement]);

  // Persist replacement effect whenever changed (blocking identical selections)
  const updateReplacement = useCallback(
    (newReplacement) => {
      if (!newReplacement || !currentTarget) return;

      // Strict enforcement: A and B cannot be the same
      if (
        newReplacement.koEffectName === currentTarget.koEffectName ||
        newReplacement.displayName === currentTarget.displayName
      ) {
        showToast(
          isArabic
            ? 'لا يمكن اختيار نفس مظهر الإطاحة الأصلي كبديل (A لا يمكن أن يساوي B)'
            : 'Replacement cannot be the same as the original effect (A cannot equal B)',
          'warning'
        );
        return;
      }

      setReplacementKoEffect(newReplacement);

      const key = currentTarget.koEffectName || currentTarget.displayName;
      setKoCustomizations((prev) => {
        const updated = {
          ...prev,
          [key]: {
            targetKoEffectName: key,
            replacementKoEffectName: newReplacement.koEffectName || newReplacement.displayName
          }
        };
        try {
          localStorage.setItem('brawlcraft_ko_customizations', JSON.stringify(updated));
        } catch {}
        return updated;
      });
    },
    [currentTarget, showToast, isArabic]
  );

  // Picker modal state
  const [showPicker, setShowPicker] = useState(false);

  // Auto-restart toggle
  const [autoRestart, setAutoRestart] = useState(() => {
    try {
      const saved = localStorage.getItem('brawlcraft_auto_restart');
      return saved !== null ? JSON.parse(saved) : true;
    } catch {
      return true;
    }
  });

  const isGameRunning = Boolean(systemInfo?.isGameRunning);

  // Fallback ensures currentReplacement is valid and never equals currentTarget
  const currentReplacement = useMemo(() => {
    if (
      replacementKoEffect &&
      replacementKoEffect.koEffectName !== currentTarget?.koEffectName &&
      replacementKoEffect.displayName !== currentTarget?.displayName
    ) {
      return replacementKoEffect;
    }
    return getSavedReplacement(currentTarget);
  }, [replacementKoEffect, currentTarget, getSavedReplacement]);

  const isSameEffect =
    !currentTarget ||
    !currentReplacement ||
    currentTarget.koEffectName === currentReplacement.koEffectName ||
    currentTarget.displayName === currentReplacement.displayName;

  // Fully formatted visual cards for CardPickerModal (Excluding currentTarget A so user cannot pick the same effect)
  const pickerCards = useMemo(() => {
    return allKoEffects
      .filter(
        (k) =>
          (k.koEffectName || k.koEffectId) !== (currentTarget?.koEffectName || currentTarget?.koEffectId) &&
          k.displayName !== currentTarget?.displayName
      )
      .map((k) => ({
        id: k.koEffectName || k.koEffectId,
        title: k.displayName,
        subtitle: `${k.category} • ${k.swfFile}`,
        badge: k.isDefault ? (isArabic ? 'افتراضي' : 'Default') : k.category,
        rawItem: k
      }));
  }, [allKoEffects, currentTarget, isArabic]);

  // Apply visual + audio modification
  const handleApply = useCallback(async () => {
    if (isSameEffect) {
      showToast(
        isArabic
          ? 'لا يمكن تطبيق التعديل: المظهر البديل هو نفسه المظهر الأصلي (A و B متطابقان)'
          : 'Cannot apply: Replacement is identical to original (A and B are identical)',
        'error'
      );
      return;
    }

    // Save customization state immediately
    const key = currentTarget.koEffectName || currentTarget.displayName;
    try {
      const updated = {
        ...koCustomizations,
        [key]: {
          targetKoEffectName: key,
          replacementKoEffectName: currentReplacement.koEffectName || currentReplacement.displayName
        }
      };
      localStorage.setItem('brawlcraft_ko_customizations', JSON.stringify(updated));
    } catch {}

    const payload = {
      targetKoEffect: currentTarget,
      replacementKoEffect: currentReplacement,
      targetKoEffectName: currentTarget.koEffectName,
      replacementKoEffectName: currentReplacement.koEffectName,
      autoRestart
    };

    if (onApplyModification) {
      onApplyModification(payload);
      return;
    }

    try {
      const res = await api.applyModification(payload);
      if (res && res.success) {
        showToast(
          isArabic
            ? `تم تطبيق تأثير الإطاحة "${currentReplacement.displayName}" بنجاح!`
            : `KO Effect "${currentReplacement.displayName}" applied successfully!`,
          'success'
        );
        if (autoRestart && isGameRunning && onRestartGame) {
          onRestartGame();
        }
      } else {
        showToast(res?.message || (isArabic ? 'فشل تطبيق تأثير الإطاحة' : 'Failed to apply KO Effect'), 'error');
      }
    } catch (err) {
      showToast(err?.message || (isArabic ? 'حدث خطأ أثناء التطبيق' : 'Error applying modification'), 'error');
    }
  }, [currentTarget, currentReplacement, isSameEffect, koCustomizations, autoRestart, isGameRunning, onApplyModification, onRestartGame, showToast, isArabic]);

  // Restore pristine KO effect files for current target
  const handleRestore = useCallback(async () => {
    try {
      const key = currentTarget?.koEffectName || currentTarget?.displayName;
      if (key) {
        setKoCustomizations((prev) => {
          const updated = { ...prev };
          delete updated[key];
          try {
            localStorage.setItem('brawlcraft_ko_customizations', JSON.stringify(updated));
          } catch {}
          return updated;
        });
      }

      // Reset replacement to distinct alternative
      const alt =
        currentTarget?.isDefault || currentTarget?.koEffectName === defaultKoEffect?.koEffectName
          ? firstNonDefaultKoEffect
          : defaultKoEffect;
      setReplacementKoEffect(alt);

      let count = 0;
      if (currentTarget?.swfFile) {
        try {
          count = await api.restoreKoEffect(currentTarget.swfFile);
        } catch {
          count = await api.restoreKoEffects();
        }
      } else {
        count = await api.restoreKoEffects();
      }

      showToast(
        isArabic
          ? `تمت استعادة تأثير الإطاحة "${currentTarget?.displayName || ''}" بنجاح (${count} ملفات)`
          : `Restored original KO effect "${currentTarget?.displayName || ''}" successfully (${count} files)`,
        'success'
      );
      if (autoRestart && isGameRunning && onRestartGame) {
        onRestartGame();
      }
    } catch (err) {
      showToast(err?.message || (isArabic ? 'فشلت الاستعادة' : 'Restore failed'), 'error');
    }
  }, [currentTarget, defaultKoEffect, firstNonDefaultKoEffect, autoRestart, isGameRunning, onRestartGame, showToast, isArabic]);

  return (
    <div className="customizer-workbench">
      {/* Workbench Navigation Header */}
      <div className="workbench-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
          <button
            type="button"
            className="button-secondary"
            onClick={onBack}
            style={{ padding: '8px 14px', fontSize: '13px' }}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M19 12H5M12 19l-7-7 7-7" />
            </svg>
            <span>{isArabic ? 'العودة إلى قائمة تأثيرات الإطاحة' : 'Back to KO Effects'}</span>
          </button>

          <div>
            <h2 style={{ fontSize: '18px', fontWeight: 800 }}>
              {currentTarget?.displayName || (isArabic ? 'تأثير الإطاحة' : 'KO Effect')}
            </h2>
          </div>
        </div>
      </div>

      {/* 2 Authentic Showcase Cards (Matching Legends Studio Cards Grid) */}
      <div className="workbench-cards-grid" style={{ gridTemplateColumns: 'repeat(2, 1fr)' }}>
        {/* Card 1: Target KO Effect (A) - المظهر الأصلي */}
        <div className="store-card">
          <div className="card-image-wrap">
            <div className="card-image">
              <AssetImage
                type="koEffect"
                item={currentTarget}
                style={{ width: '100%', height: '100%' }}
                fallbackText={currentTarget?.displayName}
              />
            </div>
            <span
              style={{
                position: 'absolute',
                top: '10px',
                right: '10px',
                color: 'var(--text-primary)',
                fontSize: '14px',
                fontWeight: 800,
                zIndex: 2,
                background: 'transparent',
                padding: 0
              }}
            >
              A
            </span>
          </div>
          <div className="card-content">
            <div className="card-heading">
              <h3 className="product-title" title={currentTarget?.displayName || ''}>
                {currentTarget?.displayName || (isArabic ? 'تأثير الإطاحة الأصلي' : 'Original KO Effect')}
              </h3>
            </div>
            <p className="product-description">
              {isArabic
                ? 'تأثير الإطاحة الذي تملكه وسيتم استبدال شكله وصوته في اللعبة'
                : 'Original KO effect in your inventory to be cosmetically replaced.'}
            </p>
          </div>
        </div>

        {/* Card 2: Replacement Visual KO Effect (B) - المظهر المطلوب (Never equal to A) */}
        <div className="store-card">
          <div className="card-image-wrap">
            <div className="card-image">
              <AssetImage
                type="koEffect"
                item={currentReplacement}
                style={{ width: '100%', height: '100%' }}
                fallbackText={currentReplacement?.displayName}
              />
            </div>
            <span
              style={{
                position: 'absolute',
                top: '10px',
                right: '10px',
                color: 'var(--text-primary)',
                fontSize: '14px',
                fontWeight: 800,
                zIndex: 2,
                background: 'transparent',
                padding: 0
              }}
            >
              B
            </span>
          </div>
          <div className="card-content">
            <div className="card-heading">
              <h3 className="product-title" title={currentReplacement?.displayName || ''}>
                {currentReplacement?.displayName || (isArabic ? 'المظهر البديل' : 'Replacement Effect')}
              </h3>
            </div>
            <p className="product-description">
              {isArabic
                ? 'المظهر والتأثير الصوتي الذي سيظهر في اللعبة أثناء المعارك'
                : 'The visual look and sound effect you will see during combat.'}
            </p>
          </div>
        </div>
      </div>

      {/* Configuration Controls Panel (Matching Legends Studio Config Panel) */}
      <div className="config-panel">
        <h3 style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '14px' }}>
          {isArabic ? 'خيارات التخصيص وتأثير الإطاحة' : 'KO Effect Customization Options'}
        </h3>

        {/* Costumes Row: Target (A) & Desired Replacement (B) in ONE horizontal level */}
        <div className="config-row">
          {/* Card 1: المظهر الأصلي (Target in inventory - display only, no edit button) */}
          <div className="config-field">
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                <circle cx="12" cy="7" r="4" />
              </svg>
              <span>{isArabic ? 'المظهر الأصلي في مخزونك (Target KO Effect)' : 'Target KO Effect in Inventory'}</span>
            </label>
            <div
              className="card-selector-trigger"
              style={{ cursor: 'default' }}
            >
              <div className="card-selector-trigger-thumb">
                <AssetImage type="koEffect" item={currentTarget} style={{ width: '100%', height: '100%' }} fallbackText={currentTarget?.displayName} />
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {currentTarget?.displayName || (isArabic ? 'تأثير الإطاحة' : 'KO Effect')}
                </div>
                <div className="card-selector-trigger-sub">
                  {currentTarget?.category} • {currentTarget?.swfFile}
                </div>
              </div>
            </div>
          </div>

          {/* Card 2: المظهر المطلوب (Desired replacement - click opens picker modal) */}
          <div className="config-field">
            <label className="config-label">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
              </svg>
              <span>{isArabic ? 'تأثير الاستبدال المرئي الجديد (Visual KO Effect)' : 'New Visual Replacement KO Effect'}</span>
            </label>
            <div
              className="card-selector-trigger"
              onClick={() => setShowPicker(true)}
              title={isArabic ? 'اضغط لفتح واجهة البطاقات واختيار التأثير' : 'Click to pick KO effect from cards'}
            >
              <div className="card-selector-trigger-thumb">
                <AssetImage type="koEffect" item={currentReplacement} style={{ width: '100%', height: '100%' }} fallbackText={currentReplacement?.displayName} />
              </div>
              <div className="card-selector-trigger-info">
                <div className="card-selector-trigger-title">
                  {currentReplacement?.displayName}
                </div>
                <div className="card-selector-trigger-sub">
                  {currentReplacement?.category} • {currentReplacement?.swfFile}
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
      </div>

      {/* Action Footer Bar (Matching Legends Studio Action Footer Exactly) */}
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
            type="button"
            className="button-secondary"
            onClick={handleRestore}
            disabled={isBusy}
            title={isArabic ? 'استعادة تأثيرات الإطاحة الأصلية' : 'Restore original KO effects'}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
            <span>{isArabic ? 'استعادة الأصلية' : 'Restore Original'}</span>
          </button>

          {isGameRunning && onRestartGame && (
            <button
              type="button"
              className="button-secondary"
              onClick={onRestartGame}
              disabled={isBusy}
              title={isArabic ? 'إعادة تشغيل اللعبة وتطبيق التعديل' : 'Restart game and apply modifications'}
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.19" />
              </svg>
              <span>{isArabic ? 'تحديث اللعبة' : 'Restart Game'}</span>
            </button>
          )}

          {!isGameRunning && onLaunchGame && (
            <button
              type="button"
              className="button-secondary"
              onClick={onLaunchGame}
              disabled={isBusy}
              title={isArabic ? 'تشغيل لعبة Brawlhalla' : 'Launch Brawlhalla'}
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
                <polygon points="5 3 19 12 5 21 5 3" />
              </svg>
              <span>{isArabic ? 'تشغيل اللعبة' : 'Launch Game'}</span>
            </button>
          )}

          <button
            type="button"
            className="button-primary"
            onClick={handleApply}
            disabled={isBusy || isSameEffect}
          >
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2.5">
              <path d="M20 6L9 17l-5-5" />
            </svg>
            <span>
              {isBusy
                ? (isArabic ? 'جاري التطبيق...' : 'Applying...')
                : (isArabic ? 'تطبيق التعديل على اللعبة' : 'Apply Modification to Game')}
            </span>
          </button>
        </div>
      </div>

      {/* Replacement Picker Modal (Excluding currentTarget A) */}
      <CardPickerModal
        isOpen={showPicker}
        onClose={() => setShowPicker(false)}
        title={isArabic ? 'اختر البديل المطلوب' : 'Select Desired Replacement'}
        type="koEffect"
        items={pickerCards}
        selectedId={currentReplacement?.koEffectName || currentReplacement?.displayName}
        onSelect={(id) => {
          const found = allKoEffects.find((k) => (k.koEffectName || k.koEffectId) === id || k.displayName === id);
          if (found) {
            updateReplacement(found);
          }
          setShowPicker(false);
        }}
      />
    </div>
  );
}
