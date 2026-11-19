import React, { useState, useEffect, useCallback, useRef } from 'react';
import TitleBar from './components/TitleBar';
import LegendsPage from './pages/LegendsPage';
import KoEffectsPage from './pages/KoEffectsPage';
import Sidebar from './components/Sidebar';
import LegendCustomizer from './components/LegendCustomizer';
import ErrorBoundary from './components/ErrorBoundary';
import SettingsModal from './components/SettingsModal';
import UpdateModal from './components/UpdateModal';
import Toast from './components/Toast';
import { api } from './services/api';
import { executeFullAppPreload } from './services/preloadService';
import { useLanguage } from './i18n/LanguageContext';

export default function App() {
  const { isArabic } = useLanguage();
  const [catalog, setCatalog] = useState(null);
  const [systemInfo, setSystemInfo] = useState(null);
  const [loading, setLoading] = useState(true);
  const [loadingProgress, setLoadingProgress] = useState(0);
  const [loadingStageTitle, setLoadingStageTitle] = useState('');
  const [loadingStatus, setLoadingStatus] = useState('');
  const [loadingItemsCount, setLoadingItemsCount] = useState({ processed: 0, total: 0 });
  const [isBusy, setIsBusy] = useState(false);
  const [toast, setToast] = useState(null);
  const [activeTab, setActiveTab] = useState('legends'); // 'legends' | 'weapons'
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

  const showToast = useCallback((message, type = 'info', force = false) => {
    try {
      const saved = localStorage.getItem('skinner_notifications_enabled');
      if (saved === 'false' && !force) {
        return;
      }
    } catch { }

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

  // Keep current isArabic and showToast in refs for one-time initialization
  const isArabicRef = useRef(isArabic);
  useEffect(() => {
    isArabicRef.current = isArabic;
  }, [isArabic]);

  const showToastRef = useRef(showToast);
  useEffect(() => {
    showToastRef.current = showToast;
  }, [showToast]);

  const hasInitializedRef = useRef(false);

  // Full Application Resource Preloading Engine (Zero-Wait Navigation) — Runs strictly ONCE on startup
  useEffect(() => {
    if (hasInitializedRef.current) return;
    hasInitializedRef.current = true;

    async function init() {
      setLoading(true);
      setLoadingProgress(3);
      const currArabic = isArabicRef.current;
      setLoadingStageTitle(currArabic ? 'تهيئة النظام والتحقق من التثبيت' : 'Initializing Application');
      setLoadingStatus(currArabic ? 'جاري فحص وتحديد مسار اللعبة...' : 'Detecting game installation...');
      try {
        const sys = await api.getSystemInfo();
        setSystemInfo(sys);
        setLoadingProgress(10);
        setLoadingStageTitle(currArabic ? 'تحميل بيانات اللعبة والكتالوج' : 'Loading Catalog Data');
        setLoadingStatus(currArabic ? 'فك تشفير وقراءة سجلات الأساطير والسكنات...' : 'Parsing legends, weapons & costumes catalog...');

        const cat = await api.getCatalog();
        setCatalog(cat);
        setLoadingProgress(18);

        if (cat) {
          await executeFullAppPreload({
            catalog: cat,
            isArabic: currArabic,
            onProgress: ({ progress, stageTitle, status, processed, total }) => {
              setLoadingProgress(progress);
              setLoadingStageTitle(stageTitle);
              setLoadingStatus(status);
              setLoadingItemsCount({ processed, total });
            }
          });
        }

        setLoadingProgress(100);
        setLoadingStageTitle(currArabic ? 'اكتمل التجهيز بنجاح! جاهز للاستخدام' : 'Studio Ready');
        setLoadingStatus(currArabic ? 'اكتمل التحميل المسبق لجميع الموارد! أهلاً بك في سكينر.' : 'All assets preloaded. Welcome to Skinner.');
      } catch (err) {
        console.error('Initialization error:', err);
        showToastRef.current?.((isArabicRef.current ? 'فشل تحميل بيانات اللعبة: ' : 'Failed to initialize catalog: ') + err.message, 'error');
      } finally {
        setTimeout(() => {
          setLoading(false);
        }, 320);
      }
    }
    init();
  }, []);

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

  // Handler: Restore Target Costume and All Weapons/Files for the Active Legend
  const handleRestoreTargetCostume = useCallback(async () => {
    const legendName = activeLegend?.heroName || activeLegend?.displayName || activeLegend?.name;
    if (!legendName) {
      showToast(isArabic ? 'لم يتم تحديد أسطورة.' : 'No legend selected.', 'error');
      return;
    }

    setIsBusy(true);
    try {
      const count = await api.restoreLegend(legendName);
      if (count > 0) {
        showToast(
          isArabic
            ? `تمت استعادة كافة أسلحة وملفات "${legendName}" لحالتها الأصلية بنجاح (${count} ملف).`
            : `Successfully restored all weapons and files for "${legendName}" to pristine original state (${count} files).`,
          'success'
        );
        await fetchSystemInfo();
      } else {
        showToast(
          isArabic
            ? `كافة ملفات وأسلحة "${legendName}" بحالتها الأصلية بالفعل.`
            : `All weapons and files for "${legendName}" are already in original state.`,
          'info'
        );
      }
    } catch (err) {
      showToast((isArabic ? 'فشلت الاستعادة: ' : 'Restore failed: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [activeLegend, isArabic, showToast, fetchSystemInfo]);

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
        localStorage.removeItem('brawlcraft_ko_customizations');
        localStorage.removeItem('brawlcraft_ko_effect_customization');
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
        onOpenSettings={() => {
          if (!loading) setIsSettingsOpen(true);
        }}
        isLoading={loading}
        hasPendingUpdate={Boolean(pendingUpdate && !isUpdateModalOpen)}
        onOpenUpdate={() => {
          if (!loading) setIsUpdateModalOpen(true);
        }}
      />

      {/* Workspace Body: Sidebar Navigation on Left + Main Content on Right */}
      <div className="app-body">
        {/* Navigation Sidebar (Left side) */}
        <Sidebar
          activeTab={activeLegend ? 'legends' : activeTab}
          onSelectTab={(tabId) => {
            if (loading) return;
            if (pageContainerRef.current) {
              if (!activeLegend && activeTab === 'legends') {
                rosterScrollPos.current = pageContainerRef.current.scrollTop;
              }
            }
            if (activeLegend) {
              setActiveLegend(null);
            }
            setActiveTab(tabId);
            if (tabId === 'legends' && pageContainerRef.current) {
              const saved = rosterScrollPos.current;
              requestAnimationFrame(() => {
                if (pageContainerRef.current) {
                  pageContainerRef.current.scrollTop = saved;
                }
              });
            } else if (pageContainerRef.current) {
              pageContainerRef.current.scrollTop = 0;
            }
          }}
          catalog={catalog}
          disabled={loading}
        />

        {/* Main Content Area (Right side) */}
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
              <div style={{ width: '100%', display: 'flex', flexDirection: 'column', gap: '10px' }}>
                {/* Stage Title & Item Counter */}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '13px' }}>
                  <span style={{ fontWeight: 700, color: 'var(--text-primary)' }}>
                    {loadingStageTitle}
                  </span>
                  <span style={{ fontWeight: 700, color: 'var(--accent, #3b82f6)' }}>
                    {loadingItemsCount.total > 0 ? `${loadingItemsCount.processed} / ${loadingItemsCount.total}` : ''}
                  </span>
                </div>

                {/* Progress Bar */}
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
                    transition: 'width 0.2s ease-out'
                  }} />
                </div>

                {/* Status Description and Percentage */}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '12px', color: 'var(--text-secondary)' }}>
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: '85%' }}>
                    {loadingStatus}
                  </span>
                  <span style={{ fontWeight: 800, color: 'var(--accent, #3b82f6)' }}>
                    {loadingProgress}%
                  </span>
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
            <>
              {/* Keep-Alive Mounted Legends Page (Zero Tab Switch Delay) */}
              <div style={{ display: activeTab === 'legends' ? 'block' : 'none', minHeight: '100%' }}>
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
              </div>

              {/* Keep-Alive Mounted KO Effects Page (Zero Tab Switch Delay) */}
              <div style={{ display: activeTab === 'ko_effects' ? 'block' : 'none', minHeight: '100%' }}>
                <KoEffectsPage
                  catalog={catalog}
                  systemInfo={systemInfo}
                  showToast={showToast}
                  isBusy={isBusy}
                  onApplyModification={handleApplyModification}
                  onRestartGame={handleRestartGame}
                  onLaunchGame={handleLaunchGame}
                />
              </div>
            </>
          )}
        </div>
        </main>
      </div>

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
