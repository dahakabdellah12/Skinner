import React, { useState, useEffect, useCallback, useRef } from 'react';
import TitleBar from './components/TitleBar';
import LegendsPage from './pages/LegendsPage';
import LegendCustomizer from './components/LegendCustomizer';
import ErrorBoundary from './components/ErrorBoundary';
import SettingsModal from './components/SettingsModal';
import UpdateModal from './components/UpdateModal';
import Toast from './components/Toast';
import { api } from './services/api';
import { useLanguage } from './i18n/LanguageContext';

export default function App() {
  const { isRTL } = useLanguage();
  const [catalog, setCatalog] = useState(null);
  const [systemInfo, setSystemInfo] = useState(null);
  const [loading, setLoading] = useState(true);
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
    setCustomizations((prev) => {
      const updated = { ...prev, [heroName]: data };
      try {
        localStorage.setItem('brawlhalla_legend_customizations', JSON.stringify(updated));
      } catch { }
      return updated;
    });
  }, []);

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

  // Initialize catalog and system info on boot
  useEffect(() => {
    async function init() {
      setLoading(true);
      try {
        const [sys, cat] = await Promise.all([
          api.getSystemInfo(),
          api.getCatalog()
        ]);
        setSystemInfo(sys);
        setCatalog(cat);
      } catch (err) {
        console.error('Initialization error:', err);
        showToast((isRTL ? 'فشل تحميل بيانات اللعبة: ' : 'Failed to load game catalog: ') + err.message, 'error');
      } finally {
        setLoading(false);
      }
    }
    init();
  }, [showToast, isRTL]);

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
      try { return localStorage.getItem('brawlcraft_auto_update') !== 'false'; } catch { return true; }
    })();

    if (!autoUpdateEnabled) return;

    // Delay slightly so the UI is fully ready before the network check
    const timer = setTimeout(async () => {
      try {
        const res = await api.checkForUpdates();
        if (res && res.hasUpdate && res.downloadUrl) {
          setPendingUpdate(res);
          setIsUpdateModalOpen(true);
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
      await api.installUpdate(pendingUpdate.downloadUrl, pendingUpdate.latestVersion);
    } catch (err) {
      console.error('Install update failed:', err);
      showToast(
        (isRTL ? 'فشل تثبيت التحديث: ' : 'Update installation failed: ') + err.message,
        'error'
      );
      setUpdateProgress(null);
    }
  }, [pendingUpdate, isRTL, showToast]);

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
    setIsUpdateModalOpen(false);
  }, []);

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
        if (req.autoRestart && systemInfo?.isGameRunning) {
          showToast(
            isRTL
              ? 'تم تطبيق التعديل بنجاح وإعادة تشغيل اللعبة فوراً!'
              : 'Modification applied and game restarted successfully!',
            'success'
          );
        } else if (systemInfo?.isGameRunning) {
          showToast(
            isRTL
              ? 'تم تطبيق التعديل على اللعبة مباشرة! غيّر الأسطورة في شاشة الاختيار لترى التعديل.'
              : 'Applied live! Switch legends in-game to see the new look.',
            'success'
          );
        } else {
          showToast(
            isRTL
              ? 'تم تطبيق التعديل بنجاح على ملفات اللعبة.'
              : 'Modification successfully written to game files.',
            'success'
          );
        }
        await fetchSystemInfo();
      } else {
        showToast(result.message || (isRTL ? 'فشل تطبيق التعديل.' : 'Modification failed.'), 'error');
      }
    } catch (err) {
      console.error('Apply error:', err);
      showToast((isRTL ? 'خطأ أثناء تطبيق التعديل: ' : 'Error applying modification: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [isRTL, showToast, systemInfo, fetchSystemInfo]);

  // Handler: Restore Target Costume's SWF
  const handleRestoreTargetCostume = useCallback(async () => {
    const swf = activeLegend?.primarySwf;
    if (!swf) {
      showToast(isRTL ? 'لا يوجد ملف مرتبط بهذه الأسطورة.' : 'No SWF file linked.', 'error');
      return;
    }

    setIsBusy(true);
    try {
      const success = await api.restoreCostume(swf);
      if (success) {
        showToast(
          isRTL
            ? `تمت استعادة ملف "${swf}" لحالته الأصلية بنجاح.`
            : `Successfully restored "${swf}" to original pristine state.`,
          'success'
        );
        await fetchSystemInfo();
      } else {
        showToast(
          isRTL
            ? `الملف "${swf}" بحالته الأصلية بالفعل.`
            : `File "${swf}" is already original or has no backup.`,
          'info'
        );
      }
    } catch (err) {
      showToast((isRTL ? 'فشلت الاستعادة: ' : 'Restore failed: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [activeLegend, isRTL, showToast, fetchSystemInfo]);

  // Handler: Instant Restart Game
  const handleRestartGame = useCallback(async () => {
    setIsBusy(true);
    try {
      const ok = await api.restartGame();
      if (ok) {
        showToast(
          isRTL ? 'تمت إعادة تشغيل اللعبة فوراً وتحديث المظاهر!' : 'Brawlhalla restarted successfully!',
          'success'
        );
      } else {
        showToast(isRTL ? 'فشلت إعادة تشغيل اللعبة.' : 'Failed to restart Brawlhalla.', 'error');
      }
      await fetchSystemInfo();
    } catch (err) {
      showToast((isRTL ? 'خطأ في إعادة التشغيل: ' : 'Restart error: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [isRTL, showToast, fetchSystemInfo]);

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

      const msg = isRTL
        ? (restoredCount > 0
            ? `تمت استعادة كافة ملفات اللعبة للأصل (${restoredCount} ملف) وإعادة ضبط جميع القوائم بنجاح!`
            : 'كافة ملفات اللعبة بحالتها الأصلية بالفعل، وتمت إعادة ضبط جميع القوائم والتخصيصات للأصل!')
        : (restoredCount > 0
            ? `All game files restored (${restoredCount} files) and all menus reset successfully!`
            : 'All game files are already in pristine state, and all menus & customizations have been reset!');

      showToast(msg, 'success');
    } catch (err) {
      console.error('Factory restore failed:', err);
      showToast((isRTL ? 'فشلت استعادة الملفات الأصلية: ' : 'Failed to restore original files: ') + err.message, 'error');
    } finally {
      setIsBusy(false);
    }
  }, [isRTL, showToast, fetchSystemInfo]);

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
          isRTL
            ? `تم التعرف على مسار اللعبة بنجاح! تم تحميل ${res.catalog?.legends?.length || 0} أسطورة.`
            : `Game path detected! Loaded ${res.catalog?.legends?.length || 0} legends.`,
          'success'
        );
      }
    } catch (err) {
      console.error('Failed to set game path:', err);
      showToast(
        (isRTL ? 'فشل تحديد مسار اللعبة: ' : 'Failed to set game path: ') + err.message,
        'error'
      );
    } finally {
      setLoading(false);
    }
  }, [isRTL, showToast]);

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
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '60vh', gap: '16px' }}>
              <div style={{
                width: '36px',
                height: '36px',
                border: '3px solid var(--border-secondary)',
                borderTopColor: 'var(--accent)',
                borderRadius: '50%',
                animation: 'spin 0.8s linear infinite'
              }} />
              <p style={{ color: 'var(--text-secondary)', fontSize: '14px', fontWeight: 600 }}>
                {isRTL ? 'جاري فحص ملفات اللعبة وتحميل الأساطير والمظاهر...' : 'Analyzing Brawlhalla assets and loading legends...'}
              </p>
              <style>{`@keyframes spin { 100% { transform: rotate(360deg); } }`}</style>
            </div>
          ) : activeLegend ? (
            <ErrorBoundary onReset={handleBackToLegends}>
              <LegendCustomizer
                key={activeLegend.heroName}
                legend={activeLegend}
                catalog={catalog}
                systemInfo={systemInfo}
                savedCustomization={customizations[activeLegend.heroName]}
                onSaveCustomization={(data) => handleSaveCustomization(activeLegend.heroName, data)}
                onBack={handleBackToLegends}
                onApplyModification={handleApplyModification}
                onRestoreTargetCostume={handleRestoreTargetCostume}
                onRestartGame={handleRestartGame}
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
      />

      {/* Toast Notification */}
      {toast && <Toast toast={toast} onDismiss={dismissToast} />}
    </div>
  );
}
