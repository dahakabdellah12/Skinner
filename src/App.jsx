import React, { useState, useEffect, useCallback, useRef } from 'react';
import TitleBar from './components/TitleBar';
import LegendsPage from './pages/LegendsPage';
import LegendCustomizer from './components/LegendCustomizer';
import ErrorBoundary from './components/ErrorBoundary';
import SettingsModal from './components/SettingsModal';
import UpdateModal from './components/UpdateModal';
import Toast from './components/Toast';
import { api } from './services/api';
import { setBatchCachedSvgs } from './services/assetImageCache';
import { useLanguage } from './i18n/LanguageContext';

export default function App() {
  const { isArabic } = useLanguage();
  const [catalog, setCatalog] = useState(null);
  const [systemInfo, setSystemInfo] = useState(null);
  const [loading, setLoading] = useState(true);
  const [loadingProgress, setLoadingProgress] = useState(0);
  const [loadingStatus, setLoadingStatus] = useState('');
  const [isBusy, setIsBusy] = useState(false);
  const [toast, setToast] = useState(null);
  const [activeLegend, setActiveLegend] = useState(null);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [rosterWeaponFilter, setRosterWeaponFilter] = useState('All');
  const [rosterSearch, setRosterSearch] = useState('');
  const pageContainerRef = useRef(null);
  const rosterScrollPos = useRef(0);

  // Update Modal & Progress State
  const [pendingUpdate, setPendingUpdate] = useState(null);
  const [isUpdateModalOpen, setIsUpdateModalOpen] = useState(false);
  const [updateProgress, setUpdateProgress] = useState(null);
  const [isUpdatePaused, setIsUpdatePaused] = useState(false);

  // Persistent customizations map per legend
  const [customizations, setCustomizations] = useState(() => {
    try {
      const saved = localStorage.getItem('brawlhalla_legend_customizations');
      return saved ? JSON.parse(saved) : {};
    } catch {
      return {};
    }
  });

  const handleSaveCustomization = useCallback((heroName, data) => {
    if (!heroName || !data) return;
    setCustomizations((prev) => {
      const existing = prev[heroName];
      if (existing && JSON.stringify(existing) === JSON.stringify(data)) {
        return prev;
      }
      const updated = { ...prev, [heroName]: data };
      try {
        localStorage.setItem('brawlhalla_legend_customizations', JSON.stringify(updated));
      } catch { }
      return updated;
    });
  }, []);

  const handleSaveActiveCustomization = useCallback((data) => {
    if (activeLegend?.heroName) {
      handleSaveCustomization(activeLegend.heroName, data);
    }
  }, [activeLegend?.heroName, handleSaveCustomization]);

  const showToast = useCallback((message, type = 'info') => {
    setToast({
      message,
      isSuccess: type === 'success',
      type
    });
  }, []);

  const dismissToast = useCallback(() => {
    setToast(null);
  }, []);

  // Poll or fetch system info
  const fetchSystemInfo = useCallback(async () => {
    try {
      const info = await api.getSystemInfo();
      setSystemInfo(info);
    } catch (err) {
      console.warn('Failed to fetch system info:', err);
    }
  }, []);

  // Initialize catalog and system info on boot (runs strictly once)
  useEffect(() => {
    async function init() {
      setLoading(true);
      setLoadingProgress(5);
      setLoadingStatus(isArabic ? 'جاري فحص وتحديد مسار اللعبة...' : 'Detecting game installation...');
      try {
        const [sys, cat] = await Promise.all([
          api.getSystemInfo(),
          api.getCatalog()
        ]);
        setSystemInfo(sys);
        setCatalog(cat);
        setLoadingProgress(25);

        // Pre-caching Algorithm (Warm up cache for all playable legends and default costumes)
        if (cat && cat.legends && cat.legends.length > 0) {
          setLoadingStatus(isArabic ? 'جاري استخراج صور وأصول الأساطير المتاحة...' : 'Gathering legend portraits & assets...');
          const itemsToPreload = [];

          // 1. Playable Legend portraits
          for (const leg of cat.legends) {
            if (leg.heroName === 'DEFAULT_CHARACTER' || leg.heroName?.toLowerCase() === 'random') continue;
            itemsToPreload.push({
              cacheKey: `legend_${leg.heroName || leg.displayName}`,
              queryParams: {
                type: 'legend',
                name: leg.displayName,
                hero: leg.heroName
              }
            });
          }

          // 2. Default costumes for each legend to ensure instant card rendering
          for (const leg of cat.legends) {
            if (leg.heroName === 'DEFAULT_CHARACTER' || leg.heroName?.toLowerCase() === 'random') continue;
            const def = (leg.costumes || []).find((c) => c.isDefault) || (leg.costumes || [])[0];
            if (def) {
              itemsToPreload.push({
                cacheKey: `costume_${def.ownerHero}_${def.costumeName}_${def.iconName}`,
                queryParams: {
                  type: 'costume',
                  icon: def.iconName,
                  iconFile: def.iconFileName,
                  costumeName: def.costumeName,
                  ownerHero: def.ownerHero,
                  swfFile: def.swfFile
                }
              });
            }
          }

          const total = itemsToPreload.length;
          const chunkSize = 25;
          let processed = 0;

          for (let i = 0; i < total; i += chunkSize) {
            const chunk = itemsToPreload.slice(i, i + chunkSize);
            const percent = Math.min(95, Math.round(25 + ((processed / total) * 70)));
            setLoadingProgress(percent);
            setLoadingStatus(
              isArabic
                ? `جاري التحميل السريع والتخزين الفوري للأساطير والسكنات... (${Math.round((processed / total) * 100)}%)`
                : `Instant pre-caching for legends & skins... (${Math.round((processed / total) * 100)}%)`
            );

            try {
              const batchRes = await api.batchGetAssetSvgs(chunk);
              if (batchRes) {
                setBatchCachedSvgs(batchRes);
              }
            } catch (batchErr) {
              console.warn('Batch preload warning:', batchErr);
            }

            processed += chunk.length;
          }

          setLoadingProgress(100);
          setLoadingStatus(isArabic ? 'اكتمل التخزين السريع بنجاح! التطبيق جاهز.' : 'Pre-caching complete! Studio ready.');
        }
      } catch (err) {
        console.error('Initialization error:', err);
        showToast('Failed to load game catalog / فشل تحميل بيانات اللعبة: ' + err.message, 'error');
      } finally {
        setTimeout(() => {
          setLoading(false);
        }, 300);
      }
    }
    init();
  }, []); // Run only once on mount, language changes seamlessly without re-triggering loader

  // Listen for background update progress events from WebView bridge
  useEffect(() => {
    const unsubscribe = api.onEvent((event, data) => {
      if (event === 'updateProgress') {
        if (data.status === 'paused') {
          setIsUpdatePaused(true);
        } else if (data.status === 'downloading') {
          setIsUpdatePaused(false);
          setUpdateProgress(data);
        } else if (data.status === 'installing') {
          setIsUpdatePaused(false);
          setUpdateProgress({ ...data, progress: 100 });
        } else if (data.status === 'cancelled') {
          setUpdateProgress(null);
          setIsUpdatePaused(false);
        }
      }
    });
    return () => unsubscribe();
  }, []);

  // Startup auto-update check — asks user first via UpdateModal
  useEffect(() => {
    const autoUpdateEnabled = (() => {
      try {
        const saved = localStorage.getItem('skinner_auto_update') ?? localStorage.getItem('brawlcraft_auto_update');
        return saved !== 'false';
      } catch {
        return true;
      }
    })();

    if (!autoUpdateEnabled) return;

    // Delay slightly so the UI is fully ready before the network check
    const timer = setTimeout(async () => {
      try {
        const res = await api.checkForUpdates();
        if (res && res.hasUpdate && res.downloadUrl) {
          setPendingUpdate(res);
          const dismissed = sessionStorage.getItem('skinner_dismissed_update');
          if (dismissed !== res.latestVersion) {
            setIsUpdateModalOpen(true);
          }
        }
      } catch (err) {
        console.warn('Auto-update check encountered:', err);
      }
    }, 3500);

    return () => clearTimeout(timer);
  }, []);

  const handleProceedUpdate = useCallback(async () => {
    if (!pendingUpdate || !pendingUpdate.downloadUrl) return;
    setUpdateProgress({ progress: 0, status: 'starting' });
    setIsUpdatePaused(false);

    try {
      await api.installUpdate(
        pendingUpdate.downloadUrl,
        pendingUpdate.latestVersion,
        pendingUpdate.sha256,
        pendingUpdate.assetName,
        pendingUpdate.manifestUrl,
        pendingUpdate.signatureUrl
      );
    } catch (err) {
      console.error('Install update failed:', err);
      showToast(
        (isArabic ? 'فشل تثبيت التحديث: ' : 'Update installation failed: ') + err.message,
        'error'
      );
      setUpdateProgress(null);
    }
  }, [pendingUpdate, isArabic, showToast]);

  const handlePauseUpdate = useCallback(async () => {
    try {
      await api.pauseUpdate();
      setIsUpdatePaused(true);
    } catch (err) {
      console.warn(err);
    }
  }, []);

  const handleResumeUpdate = useCallback(async () => {
    try {
      await api.resumeUpdate();
      setIsUpdatePaused(false);
    } catch (err) {
      console.warn(err);
    }
  }, []);

  const handleCloseUpdateModal = useCallback(() => {
    if (pendingUpdate?.latestVersion) {
      try {
        sessionStorage.setItem('skinner_dismissed_update', pendingUpdate.latestVersion);
      } catch {}
    }
    setIsUpdateModalOpen(false);
  }, [pendingUpdate]);

  const handleViewChangelog = useCallback(() => {
    if (pendingUpdate?.htmlUrl) {
      api.openExternalUrl(pendingUpdate.htmlUrl);
    }
  }, [pendingUpdate]);

  const reloadCatalog = useCallback(async () => {
    try {
      const [sys, cat] = await Promise.all([
        api.getSystemInfo(),
        api.getCatalog()
      ]);
      setSystemInfo(sys);
      setCatalog(cat);
    } catch (err) {
      console.error('Failed to reload catalog:', err);
    }
  }, []);

  // Periodic polling for game state (e.g. detect game launch/close)
  useEffect(() => {
    const timer = setInterval(() => {
      fetchSystemInfo();
    }, 3500);
    return () => clearInterval(timer);
  }, [fetchSystemInfo]);

  // Handler: Select Legend from roster (saves current scroll offset)
  const handleSelectLegend = useCallback((legend) => {
    if (pageContainerRef.current) {
      rosterScrollPos.current = pageContainerRef.current.scrollTop;
    }
    setActiveLegend(legend);
  }, []);

  // Handler: Return to Legends roster
  const handleBackToLegends = useCallback(() => {
    setActiveLegend(null);
  }, []);

  // Track scroll position while browsing legends roster
  const handleScroll = useCallback((e) => {
    if (!activeLegend) {
      rosterScrollPos.current = e.currentTarget.scrollTop;
    }
  }, [activeLegend]);

  // Restore scroll position when returning to legends roster
  useEffect(() => {
    if (!activeLegend && pageContainerRef.current) {
      const savedPos = rosterScrollPos.current;
      const animId = requestAnimationFrame(() => {
        if (pageContainerRef.current) {
          pageContainerRef.current.scrollTop = savedPos;
        }
      });
      const timeoutId = setTimeout(() => {
        if (pageContainerRef.current) {
          pageContainerRef.current.scrollTop = savedPos;
        }
      }, 40);
      return () => {
        cancelAnimationFrame(animId);
        clearTimeout(timeoutId);
      };
    }
  }, [activeLegend]);

  // Handler: Apply cosmetic modification
  const handleApplyModification = useCallback(async (req) => {
    setIsBusy(true);
    try {
      const result = await api.applyModification(req);
      if (result.success) {
        if (result.backupId === 'QUEUED') {
          showToast(
            result.message || (isArabic
              ? 'تم وضع السكن في قائمة الانتظار! سيتم تطبيقه فور إغلاق اللعبة أو عند الضغط على تحديث اللعبة.'
              : 'Skin queued! It will be applied when the game is closed or restarted.'),
            'info'
          );
        } else if (req.autoRestart) {
          showToast(
            isArabic
              ? 'تم إغلاق اللعبة بالكامل وتطبيق السكن وإعادة تشغيلها بنجاح!'
              : 'Game closed, skin applied, and restarted cleanly from roots!',
            'success'
          );
        } else {
          showToast(
            isArabic
              ? 'تم تطبيق التعديل بنجاح على ملفات اللعبة.'
              : 'Modification successfully written to game files.',
            'success'
          );
        }
        await fetchSystemInfo();
      } else {
        showToast(result.message || (isArabic ? 'فشل تطبيق التعديل.' : 'Modification failed.'), 'error');
      }
    } catch (err) {
      console.error('Apply error:', err);
      showToast((isArabic ? 'خطأ أثناء تطبيق التعديل: ' : 'Error applying modification: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [isArabic, showToast, systemInfo, fetchSystemInfo]);

  // Handler: Restore Target Costume's SWF
  const handleRestoreTargetCostume = useCallback(async () => {
    const swf = activeLegend?.primarySwf;
    if (!swf) {
      showToast(isArabic ? 'لا يوجد ملف مرتبط بهذه الأسطورة.' : 'No SWF file linked.', 'error');
      return;
    }

    setIsBusy(true);
    try {
      const success = await api.restoreCostume(swf);
      if (success) {
        showToast(
          isArabic
            ? `تمت استعادة ملف "${swf}" لحالته الأصلية بنجاح.`
            : `Successfully restored "${swf}" to original pristine state.`,
          'success'
        );
        await fetchSystemInfo();
      } else {
        showToast(
          isArabic
            ? `الملف "${swf}" بحالته الأصلية بالفعل.`
            : `File "${swf}" is already original or has no backup.`,
          'info'
        );
      }
    } catch (err) {
      showToast((isArabic ? 'فشلت الاستعادة: ' : 'Restore failed: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [activeLegend, isArabic, showToast, fetchSystemInfo]);

  // Poll isGameRunning every 3.5 seconds to dynamically update Launch/Restart button
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const running = await api.isGameRunning();
        setSystemInfo((prev) => {
          if (!prev || prev.isGameRunning === running) return prev;
          return { ...prev, isGameRunning: running };
        });
      } catch { }
    }, 3500);
    return () => clearInterval(interval);
  }, []);

  // Handler: Launch Game
  const handleLaunchGame = useCallback(async () => {
    setIsBusy(true);
    try {
      const ok = await api.launchGame();
      if (ok) {
        showToast(
          isArabic ? 'جاري تشغيل Brawlhalla...' : 'Launching Brawlhalla...',
          'success'
        );
      } else {
        showToast(isArabic ? 'فشل تشغيل اللعبة.' : 'Failed to launch Brawlhalla.', 'error');
      }
      await fetchSystemInfo();
    } catch (err) {
      showToast((isArabic ? 'خطأ في تشغيل اللعبة: ' : 'Launch error: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [isArabic, showToast, fetchSystemInfo]);

  // Handler: Instant Restart Game
  const handleRestartGame = useCallback(async () => {
    setIsBusy(true);
    try {
      const res = await api.restartGame();
      const ok = res && (typeof res === 'boolean' ? res : res.success);
      if (ok) {
        showToast(
          isArabic ? 'تم إغلاق اللعبة بالكامل وتطبيق السكنات وإعادة تشغيلها بنجاح!' : 'Brawlhalla terminated and restarted cleanly with skins active!',
          'success'
        );
      } else {
        showToast(isArabic ? 'فشلت إعادة تشغيل اللعبة.' : 'Failed to restart Brawlhalla.', 'error');
      }
      await fetchSystemInfo();
    } catch (err) {
      showToast((isArabic ? 'خطأ في إعادة التشغيل: ' : 'Restart error: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [isArabic, showToast, fetchSystemInfo]);

  // Handler: Factory Reset & Revert All to Original
  const handleRestoreEverything = useCallback(async () => {
    setIsBusy(true);
    try {
      const restoredCount = await api.restoreAll();
      try {
        localStorage.removeItem('brawlhalla_legend_customizations');
      } catch { }

      // Reset all states and menus to original default
      setCustomizations({});
      setRosterWeaponFilter('All');
      setRosterSearch('');
      setActiveLegend(null);
      await fetchSystemInfo();
      setIsSettingsOpen(false);

      const msg = isArabic
        ? (restoredCount > 0
            ? `تمت استعادة كافة ملفات اللعبة للأصل (${restoredCount} ملف) وإعادة ضبط جميع القوائم بنجاح!`
            : 'كافة ملفات اللعبة بحالتها الأصلية بالفعل، وتمت إعادة ضبط جميع القوائم والتخصيصات للأصل!')
        : (restoredCount > 0
            ? `All game files restored (${restoredCount} files) and all menus reset successfully!`
            : 'All game files are already in pristine state, and all menus & customizations have been reset!');

      showToast(msg, 'success');
    } catch (err) {
      console.error('Factory restore failed:', err);
      showToast((isArabic ? 'فشلت استعادة الملفات الأصلية: ' : 'Failed to restore original files: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [isArabic, showToast, fetchSystemInfo]);

  // Handler: Select custom Brawlhalla game path
  const handleSelectGamePath = useCallback(async (pathOrAction) => {
    try {
      let targetPath = pathOrAction;
      if (pathOrAction === 'browse') {
        const browsed = await api.browseGameExe();
        if (!browsed) return;
        targetPath = browsed;
      }

      setLoading(true);
      const res = await api.selectGamePath(targetPath);
      if (res && res.success) {
        setSystemInfo(res.systemInfo);
        setCatalog(res.catalog);
        showToast(
          isArabic
            ? `تم التعرف على مسار اللعبة بنجاح! تم تحميل ${res.catalog?.legends?.length || 0} أسطورة.`
            : `Game path detected! Loaded ${res.catalog?.legends?.length || 0} legends.`,
          'success'
        );
      }
    } catch (err) {
      console.error('Failed to set game path:', err);
      showToast(
        (isArabic ? 'فشل تحديد مسار اللعبة: ' : 'Failed to set game path: ') + err.message,
        'error'
      );
    } finally {
      setLoading(false);
    }
  }, [isArabic, showToast]);

  return (
    <div className="app-container">
      {/* Windows Custom Title Bar */}
      <TitleBar
        onOpenSettings={() => setIsSettingsOpen(true)}
        hasPendingUpdate={Boolean(pendingUpdate && !isUpdateModalOpen)}
        onOpenUpdate={() => setIsUpdateModalOpen(true)}
      />

      {/* Main Content Area */}
      <main className="main-content">
        <div className="page-container" ref={pageContainerRef} onScroll={handleScroll}>
          {loading ? (
            <div style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              minHeight: '65vh',
              gap: '24px',
              padding: '24px',
              maxWidth: '520px',
              margin: '0 auto',
              textAlign: 'center'
            }}>
              <div>
                <img
                  src="/logo.png"
                  alt="Skinner"
                  style={{
                    width: '84px',
                    height: '84px',
                    objectFit: 'contain'
                  }}
                />
              </div>

              {/* Progress Bar Container */}
              <div style={{ width: '100%', display: 'flex', flexDirection: 'column', gap: '8px' }}>
                <div style={{
                  width: '100%',
                  height: '8px',
                  backgroundColor: 'rgba(255, 255, 255, 0.08)',
                  borderRadius: '999px',
                  overflow: 'hidden',
                  border: '1px solid rgba(255, 255, 255, 0.1)'
                }}>
                  <div style={{
                    width: `${loadingProgress}%`,
                    height: '100%',
                    backgroundColor: 'var(--accent, #3b82f6)',
                    borderRadius: '999px',
                    transition: 'width 0.25s ease-out'
                  }} />
                </div>

                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '12px', color: 'var(--text-secondary)' }}>
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: '85%' }}>{loadingStatus}</span>
                  <span style={{ fontWeight: 700, color: 'var(--accent, #3b82f6)' }}>{loadingProgress}%</span>
                </div>
              </div>
            </div>
          ) : activeLegend ? (
            <ErrorBoundary onReset={handleBackToLegends}>
              <LegendCustomizer
                key={activeLegend.heroName}
                legend={activeLegend}
                catalog={catalog}
                systemInfo={systemInfo}
                savedCustomization={customizations[activeLegend.heroName]}
                onSaveCustomization={handleSaveActiveCustomization}
                onBack={handleBackToLegends}
                onApplyModification={handleApplyModification}
                onRestoreTargetCostume={handleRestoreTargetCostume}
                onRestartGame={handleRestartGame}
                onLaunchGame={handleLaunchGame}
                showToast={showToast}
                isBusy={isBusy}
              />
            </ErrorBoundary>
          ) : (
            <LegendsPage
              catalog={catalog}
              systemInfo={systemInfo}
              onSelectLegend={handleSelectLegend}
              onSelectGamePath={handleSelectGamePath}
              search={rosterSearch}
              setSearch={setRosterSearch}
              weaponFilter={rosterWeaponFilter}
              setWeaponFilter={setRosterWeaponFilter}
            />
          )}
        </div>
      </main>

      {/* Settings Modal */}
      <SettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        onRestoreEverything={handleRestoreEverything}
        onReloadCatalog={reloadCatalog}
        onTriggerUpdate={(updateData) => {
          setPendingUpdate(updateData);
          setIsSettingsOpen(false);
          setIsUpdateModalOpen(true);
        }}
        showToast={showToast}
        isBusy={isBusy}
      />

      {/* Interactive Update Modal with Question, Progress Bar & Pause Option */}
      <UpdateModal
        isOpen={isUpdateModalOpen}
        updateInfo={pendingUpdate}
        updateProgress={updateProgress}
        isPaused={isUpdatePaused}
        onProceed={handleProceedUpdate}
        onPause={handlePauseUpdate}
        onResume={handleResumeUpdate}
        onClose={handleCloseUpdateModal}
        onViewChangelog={handleViewChangelog}
      />

      {/* Toast Notification */}
      {toast && <Toast toast={toast} onDismiss={dismissToast} />}
    </div>
  );
}
