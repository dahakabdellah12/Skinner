import React, { useState, useEffect, useRef } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import api from '../services/api';
import { clearAssetImageCache } from '../services/assetImageCache';
import { APP_VERSION } from '../version';
import { Wrench, Sparkles, RotateCcw, Download } from 'lucide-react';

// ─── GitHub icon (Octicons-style filled mark) ────────────────────────────────
const GitHubIcon = ({ size = 18 }) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
  >
    <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
  </svg>
);

export default function SettingsModal({
  isOpen,
  onClose,
  onRestoreEverything,
  onReloadCatalog,
  onTriggerUpdate,
  showToast,
  isBusy
}) {
  const { lang, setLang, isArabic } = useLanguage();
  const [activeTab, setActiveTab] = useState('language');
  const [confirmResetStock, setConfirmResetStock] = useState(false);
  const [isLangDropdownOpen, setIsLangDropdownOpen] = useState(false);
  const langDropdownRef = useRef(null);

  // Auto-update preference (persisted in localStorage)
  const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(() => {
    try {
      return localStorage.getItem('brawlcraft_auto_update') !== 'false';
    } catch {
      return true;
    }
  });

  const handleToggleAutoUpdate = (val) => {
    setAutoUpdateEnabled(val);
    try {
      localStorage.setItem('brawlcraft_auto_update', String(val));
    } catch {}
  };

  // Maintenance & Troubleshooting State
  const [repairLoading, setRepairLoading] = useState(false);
  const [cacheLoading, setCacheLoading] = useState(false);

  // Real Software Update State
  const [updateState, setUpdateState] = useState({
    loading: false,
    checked: false,
    hasUpdate: false,
    currentVersion: APP_VERSION,
    latestVersion: APP_VERSION,
    downloadUrl: '',
    releaseNotes: '',
    checkedAt: '',
    error: ''
  });

  useEffect(() => {
    const handleClickOutside = (event) => {
      if (langDropdownRef.current && !langDropdownRef.current.contains(event.target)) {
        setIsLangDropdownOpen(false);
      }
    };
    const handleKeyDown = (event) => {
      if (event.key === 'Escape') {
        setIsLangDropdownOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  const handleSelectLanguage = (newLang) => {
    setLang(newLang);
    setIsLangDropdownOpen(false);
  };

  const handleTabChange = (tab) => {
    setActiveTab(tab);
    setConfirmResetStock(false);
  };

  if (!isOpen) return null;

  const handleClose = () => {
    if (isBusy || repairLoading || cacheLoading) return;
    setConfirmResetStock(false);
    onClose();
  };

  const handleConfirmResetStock = async () => {
    await onRestoreEverything();
    setConfirmResetStock(false);
  };

  const handleRunDiagnosticRepair = async () => {
    if (repairLoading || isBusy) return;
    setRepairLoading(true);
    try {
      clearAssetImageCache();
      const res = await api.repairGameInstallation();
      if (res && res.success) {
        if (onReloadCatalog) await onReloadCatalog();
        const successMsg = isArabic
          ? 'تم الفحص الشامل وإصلاح كافة المشاكل وتحديث أصول اللعبة بنجاح!'
          : 'Diagnostic & auto-repair completed successfully! Assets refreshed and cache cleared.';
        showToast?.(successMsg, 'success');
      } else {
        showToast?.(res?.message || (isArabic ? 'تعذر إتمام الإصلاح التلقائي.' : 'Failed to complete auto-repair.'), 'error');
      }
    } catch (err) {
      showToast?.(err.message, 'error');
    } finally {
      setRepairLoading(false);
    }
  };

  const handleClearCache = async () => {
    if (cacheLoading || isBusy) return;
    setCacheLoading(true);
    try {
      clearAssetImageCache();
      await api.clearAssetCache();
      if (onReloadCatalog) await onReloadCatalog();
      const successMsg = isArabic
        ? 'تم تفريغ ذاكرة التخزين المؤقت للصور وتحديث كافة الرسوم بنجاح!'
        : 'Image preview cache cleared! All visuals will refresh fresh.';
      showToast?.(successMsg, 'success');
    } catch (err) {
      showToast?.(err.message, 'error');
    } finally {
      setCacheLoading(false);
    }
  };

  const handleCheckUpdates = async () => {
    if (updateState.loading) return;
    setUpdateState((prev) => ({ ...prev, loading: true, error: '' }));
    try {
      const res = await api.checkForUpdates();
      if (res) {
        setUpdateState({
          loading: false,
          checked: true,
          hasUpdate: !!res.hasUpdate,
          currentVersion: res.currentVersion || APP_VERSION,
          latestVersion: res.latestVersion || APP_VERSION,
          releaseTitle: res.releaseTitle || '',
          downloadUrl: res.downloadUrl || 'https://github.com/dahakabdellah12/BrawlCraft#download',
          htmlUrl: res.htmlUrl || 'https://github.com/dahakabdellah12/BrawlCraft#download',
          assetName: res.assetName || null,
          assetSizeFormatted: res.assetSizeFormatted || null,
          sha256: res.sha256 || null,
          verified: !!res.verified,
          releaseNotes: res.releaseNotes || '',
          repo: res.repo || 'dahakabdellah12/BrawlCraft',
          repoUrl: res.repoUrl || 'https://github.com/dahakabdellah12/BrawlCraft',
          checkedAt: res.checkedAt || new Date().toLocaleTimeString(),
          error: res.networkError
            ? (isArabic ? 'تعذر الاتصال بخادم التحديثات. تحقق من اتصال الإنترنت.' : 'Could not reach the update server. Check your internet connection.')
            : ''
        });
        if (res.hasUpdate) {
          showToast?.(
            isArabic ? `يتوفر إصدار جديد على GitHub: v${res.latestVersion}!` : `New update available on GitHub: v${res.latestVersion}!`,
            'info'
          );
        } else if (!res.networkError) {
          showToast?.(
            isArabic ? `أنت تستخدم أحدث إصدار v${res.currentVersion || APP_VERSION} من GitHub ✓` : `You are running the latest v${res.currentVersion || APP_VERSION} release from GitHub ✓`,
            'success'
          );
        }
      }
    } catch (err) {
      setUpdateState((prev) => ({
        ...prev,
        loading: false,
        checked: true,
        error: err.message
      }));
    }
  };

  const handleDownloadUpdate = () => {
    if (!updateState.downloadUrl) return;
    if (onTriggerUpdate) {
      onTriggerUpdate(updateState);
    }
  };

  const pageMeta = {
    language: {
      title: isArabic ? 'لغة الواجهة' : 'Language'
    },
    maintenance: {
      title: isArabic ? 'الصيانة والاستعادة' : 'Maintenance & Reset'
    },
    updates: {
      title: isArabic ? 'تحديثات البرنامج' : 'Software Updates'
    },
    about: {
      title: isArabic ? 'حول التطبيق' : 'About BrawlCraft'
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.76)',
        backdropFilter: 'blur(6px)',
        WebkitBackdropFilter: 'blur(6px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 10000,
        padding: '20px',
        animation: 'settingsFadeIn 0.2s ease-out'
      }}
      onClick={handleClose}
    >
      <div
        className="settings-window"
        style={{
          width: '100%',
          maxWidth: '820px',
          height: '520px',
          maxHeight: '90vh',
          backgroundColor: 'var(--bg-primary)',
          border: '1px solid var(--border-primary)',
          borderRadius: '12px',
          boxShadow: '0 20px 40px rgba(0, 0, 0, 0.65)',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'row',
          direction: 'ltr'
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Sidebar */}
        <aside
          className="settings-sidebar"
          style={{
            width: '215px',
            flexShrink: 0,
            padding: '24px 14px',
            backgroundColor: 'var(--bg-secondary)',
            borderRight: '1px solid var(--border-primary)',
            display: 'flex',
            flexDirection: 'column'
          }}
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '5px' }}>
            {/* Nav Items */}
            <button
              type="button"
              className={`nav-item ${activeTab === 'language' ? 'active' : ''}`}
              onClick={() => handleTabChange('language')}
            >
              <span className="nav-icon" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
                  <title>language</title>
                  <path fill="currentColor" d="m12.87 15.07l-2.54-2.51l.03-.03A17.5 17.5 0 0 0 14.07 6H17V4h-7V2H8v2H1v2h11.17C11.5 7.92 10.44 9.75 9 11.35C8.07 10.32 7.3 9.19 6.69 8h-2c.73 1.63 1.73 3.17 2.98 4.56l-5.09 5.02L4 19l5-5l3.11 3.11zM18.5 10h-2L12 22h2l1.12-3h4.75L21 22h2zm-2.62 7l1.62-4.33L19.12 17z" />
                </svg>
              </span>
              <span className="nav-text">{isArabic ? 'اللغة' : 'Language'}</span>
            </button>

            <button
              type="button"
              className={`nav-item ${activeTab === 'maintenance' ? 'active' : ''}`}
              onClick={() => handleTabChange('maintenance')}
            >
              <span className="nav-icon" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
                  <title>maintenance</title>
                  <path fill="currentColor" d="M8 2a6 6 0 0 1 5.743 7.743L20 16a2.828 2.828 0 0 1-3.785 4.194L16 20l-6.257-6.257a6 6 0 0 1-7.458-7.577L5.123 9l2.814-.937l.125-.126L9 5.127L6.158 2.288C6.738 2.101 7.358 2 8 2m4.586 9.57a.829.829 0 0 0-1.156 1.188l5.984 5.828a.829.829 0 0 0 1.172-1.172z" />
                </svg>
              </span>
              <span className="nav-text">{isArabic ? 'الصيانة والاستعادة' : 'Maintenance'}</span>
            </button>

            <button
              type="button"
              className={`nav-item ${activeTab === 'updates' ? 'active' : ''}`}
              onClick={() => handleTabChange('updates')}
            >
              <span className="nav-icon" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
                  <path fill="currentColor" d="M12 22c1.1 0 2-.9 2-2h-4a2 2 0 0 0 2 2m6-6v-5c0-3.07-1.64-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.63 5.36 6 7.92 6 11v5l-2 2v1h16v-1z" />
                </svg>
              </span>
              <span className="nav-text">{isArabic ? 'التحديثات' : 'Updates'}</span>
            </button>

            <button
              type="button"
              className={`nav-item ${activeTab === 'about' ? 'active' : ''}`}
              onClick={() => handleTabChange('about')}
            >
              <span className="nav-icon" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24">
                  <path fill="currentColor" d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10s10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z" />
                </svg>
              </span>
              <span className="nav-text">{isArabic ? 'حول التطبيق' : 'About'}</span>
            </button>
          </div>
        </aside>

        {/* Main Content Area */}
        <main
          className="settings-main"
          style={{
            flex: 1,
            minWidth: 0,
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'space-between',
            backgroundColor: 'var(--bg-primary)'
          }}
        >
          {/* Scrollable Container */}
          <div
            style={{
              padding: '28px 34px',
              overflowY: 'auto',
              flex: 1
            }}
          >
            {/* Header */}
            <header
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: '26px'
              }}
            >
              <div>
                <h1
                  style={{
                    margin: 0,
                    fontSize: '22px',
                    fontWeight: 700,
                    letterSpacing: '-0.3px',
                    color: 'var(--text-primary)'
                  }}
                >
                  {pageMeta[activeTab].title}
                </h1>
              </div>

              <div>
                <button
                  type="button"
                  onClick={handleClose}
                  disabled={isBusy}
                  className="settings-close-btn"
                  aria-label="Close"
                  title="✕"
                >
                  <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18" />
                    <line x1="6" y1="6" x2="18" y2="18" />
                  </svg>
                </button>
              </div>
            </header>

            {/* TAB: LANGUAGE */}
            {activeTab === 'language' && (
              <section className="settings-page" style={{ display: 'flex', flexDirection: 'column', gap: '22px' }}>
                <div className="card-group" style={{ overflow: 'visible' }}>
                  <div className="setting" style={{ overflow: 'visible' }}>
                    <div className="setting-info">
                      <div className="setting-name">{isArabic ? 'اللغة المختارة' : 'Selected Language'}</div>
                      <div className="setting-desc">
                        {isArabic ? 'التبديل الفوري بين اللغة العربية والإنجليزية.' : 'Instantly toggle between Arabic and English.'}
                      </div>
                    </div>

                    <div
                      className={`dropdown ${isLangDropdownOpen ? 'open' : ''}`}
                      id="dropdown"
                      ref={langDropdownRef}
                    >
                      <button
                        type="button"
                        className="dropdown-button"
                        id="dropdown-button"
                        aria-haspopup="listbox"
                        aria-expanded={isLangDropdownOpen}
                        onClick={() => setIsLangDropdownOpen((prev) => !prev)}
                      >
                        <span id="selected-label">
                          {lang === 'ar' ? 'العربية' : 'English'}
                        </span>
                        <span className="arrow" />
                      </button>

                      <div className="dropdown-options" role="listbox">
                        <button
                          type="button"
                          className={`option ${lang === 'ar' ? 'selected' : ''}`}
                          role="option"
                          aria-selected={lang === 'ar'}
                          data-value="ar"
                          onClick={() => handleSelectLanguage('ar')}
                        >
                          <span>العربية</span>
                          <span className="check">✓</span>
                        </button>

                        <button
                          type="button"
                          className={`option ${lang === 'en' ? 'selected' : ''}`}
                          role="option"
                          aria-selected={lang === 'en'}
                          data-value="en"
                          onClick={() => handleSelectLanguage('en')}
                        >
                          <span>English</span>
                          <span className="check">✓</span>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              </section>
            )}

            {/* TAB: MAINTENANCE & RESET */}
            {activeTab === 'maintenance' && (
              <section className="settings-page" style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                {!confirmResetStock ? (
                  <div className="card-group" style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    {/* Item 1: Auto-Repair & Diagnostic */}
                    <div className="setting" style={{ alignItems: 'flex-start' }}>
                      <div className="setting-info" style={{ flex: 1 }}>
                        <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                          <Wrench size={16} style={{ color: 'var(--accent)' }} />
                          <span>{isArabic ? 'إصلاح وتشخيص شامل وحل المشاكل (Auto-Repair)' : 'Diagnostic & Auto-Repair'}</span>
                        </div>
                        <div className="setting-desc" style={{ marginTop: '4px' }}>
                          {isArabic
                            ? 'خيار ذكي لحل أي مشاكل في اختفاء الشخصيات، البطاقات الفارغة، تلف الكاش، أو خطأ قراءة ملفات SWF، مع إعادة تهيئة ومزامنة كافة أصول اللعبة تلقائياً.'
                            : 'Smart diagnostic tool that fixes missing character portraits, empty cards, corrupted cache, and SWF read issues by resynchronizing all game assets.'}
                        </div>
                      </div>

                      <button
                        type="button"
                        className="btn btn-primary"
                        onClick={handleRunDiagnosticRepair}
                        disabled={repairLoading || isBusy}
                        style={{ whiteSpace: 'nowrap', fontWeight: 700, minWidth: '130px', padding: '9px 18px', marginTop: '2px' }}
                      >
                        {repairLoading ? (isArabic ? 'جاري الفحص...' : 'Diagnosing...') : (isArabic ? 'فحص وإصلاح الآن' : 'Run Auto-Repair')}
                      </button>
                    </div>

                    {/* Item 2: Clear Visual Cache */}
                    <div className="setting" style={{ alignItems: 'flex-start' }}>
                      <div className="setting-info" style={{ flex: 1 }}>
                        <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                          <Sparkles size={16} style={{ color: 'var(--accent)' }} />
                          <span>{isArabic ? 'تفريغ كاش الصور وتحديث الرسوم' : 'Clear Visual Cache & Refresh'}</span>
                        </div>
                        <div className="setting-desc" style={{ marginTop: '4px' }}>
                          {isArabic
                            ? 'مسح التخزين المؤقت لرسوم الشخصيات والأسلحة في الذاكرة لإعادة توليد صور البطاقات بأعلى دقة متوفرة.'
                            : 'Purges all in-memory character, costume, and weapon vector previews to trigger crisp, high-res rendering.'}
                        </div>
                      </div>

                      <button
                        type="button"
                        className="btn btn-secondary"
                        onClick={handleClearCache}
                        disabled={cacheLoading || isBusy}
                        style={{ whiteSpace: 'nowrap', fontWeight: 600, minWidth: '130px', padding: '9px 18px', marginTop: '2px' }}
                      >
                        {cacheLoading ? (isArabic ? 'جاري التفريغ...' : 'Clearing...') : (isArabic ? 'تفريغ الكاش' : 'Clear Cache')}
                      </button>
                    </div>

                    {/* Item 3: Restore Original Game Files */}
                    <div className="setting" style={{ alignItems: 'flex-start' }}>
                      <div className="setting-info" style={{ flex: 1 }}>
                        <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                          <RotateCcw size={16} style={{ color: 'var(--accent)' }} />
                          <span>{isArabic ? 'استعادة ملفات اللعبة الأصلية (Stock Restore)' : 'Restore Original Game Files'}</span>
                        </div>
                        <div className="setting-desc" style={{ marginTop: '4px' }}>
                          {isArabic
                            ? 'إلغاء كافة المودات والتعديلات النشطة واستعادة الملفات الرسمية النظيفة للعبة كما تم تنزيلها من Steam.'
                            : 'Revert all applied mods and safely restore game files to their clean pristine official state.'}
                        </div>
                      </div>

                      <button
                        type="button"
                        className="btn btn-secondary"
                        onClick={() => setConfirmResetStock(true)}
                        disabled={isBusy || repairLoading}
                        style={{ whiteSpace: 'nowrap', fontWeight: 600, minWidth: '130px', padding: '9px 18px', marginTop: '2px' }}
                      >
                        {isArabic ? 'إعادة كل شيء للأصل' : 'Restore Stock'}
                      </button>
                    </div>
                  </div>
                ) : (
                  <div
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      justifyContent: 'center',
                      minHeight: '260px',
                      textAlign: 'center',
                      padding: '30px 20px',
                      gap: '16px',
                      backgroundColor: 'var(--bg-card)',
                      border: '1px solid var(--border-primary)',
                      borderRadius: '11px'
                    }}
                  >
                    <h2 style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-primary)', margin: 0 }}>
                      {isArabic ? 'هل تريد بالتأكيد إعادة كل شيء إلى الأصل؟' : 'Are you sure you want to restore everything to original?'}
                    </h2>
                    <p style={{ fontSize: '13px', color: 'var(--text-secondary)', maxWidth: '440px', lineHeight: 1.5, margin: 0 }}>
                      {isArabic
                        ? 'سيتم إلغاء جميع التعديلات والمظاهر النشطة واستعادة الملفات الرسمية النظيفة للعبة.'
                        : 'All active mods and customizations will be reverted, restoring original game assets.'}
                    </p>
                    <div style={{ display: 'flex', gap: '14px', marginTop: '8px' }}>
                      <button
                        type="button"
                        className="btn btn-primary"
                        onClick={handleConfirmResetStock}
                        disabled={isBusy}
                        style={{ minWidth: '96px', padding: '10px 24px', fontSize: '14px' }}
                      >
                        {isBusy ? (isArabic ? 'جاري الاستعادة...' : 'Restoring...') : (isArabic ? 'نعم' : 'Yes')}
                      </button>
                      <button
                        type="button"
                        className="btn btn-secondary"
                        onClick={() => setConfirmResetStock(false)}
                        disabled={isBusy}
                        style={{ minWidth: '96px', padding: '10px 24px', fontSize: '14px' }}
                      >
                        {isArabic ? 'لا' : 'No'}
                      </button>
                    </div>
                  </div>
                )}
              </section>
            )}

            {/* TAB: ABOUT */}
            {activeTab === 'about' && (
              <section className="settings-page" style={{ display: 'flex', flexDirection: 'column', gap: '22px' }}>
                <div className="card-group">
                  <div className="setting">
                    <div className="setting-info">
                      <div className="setting-name">{isArabic ? 'اسم التطبيق' : 'Application Name'}</div>
                      <div className="setting-desc">BrawlCraft — Ultimate Cosmetic Studio for Brawlhalla</div>
                    </div>
                    <span style={{ fontWeight: 700, color: 'var(--accent-light)', fontSize: '13px' }}>
                      BrawlCraft
                    </span>
                  </div>

                  <div className="setting">
                    <div className="setting-info">
                      <div className="setting-name">{isArabic ? 'رقم الإصدار' : 'Application Version'}</div>
                      <div className="setting-desc">{isArabic ? 'الإصدار المكتبي المستقر.' : 'Installed desktop release.'}</div>
                    </div>
                    <span style={{ color: 'var(--text-muted)', fontSize: '13px', fontWeight: 600 }}>v{APP_VERSION}</span>
                  </div>

                  <div className="setting">
                    <div className="setting-info">
                      <div className="setting-name">{isArabic ? 'المنصة' : 'Platform'}</div>
                      <div className="setting-desc">.NET 8 (WPF x64) + WebView2 + React 19</div>
                    </div>
                    <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>Windows x64</span>
                  </div>

                  {/* OFFICIAL GITHUB ROW — icon only */}
                  <div className="setting" style={{ alignItems: 'center' }}>
                    <div className="setting-info" style={{ flex: 1 }}>
                      <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <GitHubIcon size={15} />
                        <span>{isArabic ? 'المستودع الرسمي' : 'Official Repository'}</span>
                      </div>
                      <div className="setting-desc">
                        {isArabic
                          ? 'تابع الإصدارات الرسمية والتحديثات على منصة GitHub.'
                          : 'Follow official builds and updates on GitHub.'}
                        <div style={{ marginTop: '4px', fontFamily: 'monospace', color: 'var(--accent-light)', fontSize: '12px' }}>
                          github.com/dahakabdellah12/BrawlCraft
                        </div>
                      </div>
                    </div>
                    {/* Icon-only GitHub button */}
                    <button
                      type="button"
                      className="github-icon-btn"
                      onClick={() => api.openExternalUrl('https://github.com/dahakabdellah12/BrawlCraft')}
                      title={isArabic ? 'عرض على GitHub' : 'View on GitHub'}
                      aria-label={isArabic ? 'عرض على GitHub' : 'View on GitHub'}
                    >
                      <GitHubIcon size={18} />
                    </button>
                  </div>
                </div>
              </section>
            )}

            {/* TAB: UPDATES */}
            {activeTab === 'updates' && (
              <section className="settings-page" style={{ display: 'flex', flexDirection: 'column', gap: '22px' }}>
                <div className="card-group">

                  {/* Auto-update toggle */}
                  <div className="setting" style={{ alignItems: 'center' }}>
                    <div className="setting-info" style={{ flex: 1 }}>
                      <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                          <path d="M12 22c1.1 0 2-.9 2-2h-4a2 2 0 0 0 2 2m6-6v-5c0-3.07-1.64-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.63 5.36 6 7.92 6 11v5l-2 2v1h16v-1z"/>
                        </svg>
                        <span>{isArabic ? 'تحديثات البرنامج التلقائية' : 'Automatic In-App Updates'}</span>
                      </div>
                      <div className="setting-desc">
                        {isArabic
                          ? 'عند تفعيل هذا الخيار، يقوم BrawlCraft بالتحقق وتثبيت التحديثات تلقائياً في الخلفية تماماً مثل تحديثات Steam.'
                          : 'When enabled, BrawlCraft checks and automatically updates seamlessly in the background just like Steam game updates.'}
                      </div>
                    </div>
                    {/* Toggle switch */}
                    <button
                      type="button"
                      role="switch"
                      aria-checked={autoUpdateEnabled}
                      onClick={() => handleToggleAutoUpdate(!autoUpdateEnabled)}
                      className={`toggle-switch ${autoUpdateEnabled ? 'on' : 'off'}`}
                      title={autoUpdateEnabled
                        ? (isArabic ? 'انقر لتعطيل الإشعارات التلقائية' : 'Click to disable auto-notifications')
                        : (isArabic ? 'انقر لتفعيل الإشعارات التلقائية' : 'Click to enable auto-notifications')}
                    >
                      <span className="toggle-thumb" />
                    </button>
                  </div>

                  {/* Manual check row */}
                  <div className="setting" style={{ alignItems: 'flex-start', padding: '14px 0' }}>
                    <div className="setting-info" style={{ flex: 1 }}>
                      <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                          <path d="M19 8l-4 4h3c0 3.31-2.69 6-6 6a5.87 5.87 0 0 1-2.8-.7l-1.46 1.46A7.93 7.93 0 0 0 12 20c4.42 0 8-3.58 8-8h3zM6 12c0-3.31 2.69-6 6-6 1.01 0 1.97.25 2.8.7l1.46-1.46A7.93 7.93 0 0 0 12 4c-4.42 0-8 3.58-8 8H1l4 4 4-4z"/>
                        </svg>
                        <span>{isArabic ? 'التحقق اليدوي من التحديثات' : 'Manual Update Check'}</span>
                      </div>
                      <div className="setting-desc" style={{ marginTop: '4px' }}>
                        {updateState.loading ? (
                          <span style={{ color: 'var(--accent-light)' }}>
                            {isArabic ? 'جاري التحقق من أحدث إصدار...' : 'Checking for the latest version...'}
                          </span>
                        ) : updateState.checked ? (
                          updateState.hasUpdate ? (
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                              <span style={{ color: 'var(--warning, #F59E0B)', fontWeight: 700 }}>
                                {isArabic
                                  ? `يتوفر إصدار جديد: v${updateState.latestVersion}!`
                                  : `New release available: v${updateState.latestVersion}!`}
                              </span>
                              {updateState.releaseTitle && (
                                <span style={{ fontSize: '12px', color: 'var(--text-primary)' }}>
                                  {updateState.releaseTitle}
                                </span>
                              )}
                              {updateState.assetName && (
                                <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                                  {isArabic ? 'حزمة التثبيت:' : 'Setup Package:'} {updateState.assetName} {updateState.assetSizeFormatted ? `(${updateState.assetSizeFormatted})` : ''}
                                </span>
                              )}
                              {updateState.sha256 ? (
                                <span style={{ fontSize: '11px', color: 'var(--success, #22C55E)' }}>
                                  {isArabic ? 'تم التحقق من بصمة SHA-256 ✓' : 'SHA-256 checksum available ✓'}
                                </span>
                              ) : (
                                <span style={{ fontSize: '11px', color: 'var(--warning, #F59E0B)' }}>
                                  {isArabic
                                    ? 'لا تتوفر بصمة تحقق لهذا الإصدار، لذلك تم تعطيل التثبيت التلقائي.'
                                    : 'No checksum is published for this release, so automatic install is disabled.'}
                                </span>
                              )}
                            </div>
                          ) : updateState.error ? (
                            <span style={{ color: 'var(--error, #EF4444)' }}>
                              {updateState.error}
                            </span>
                          ) : (
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                              <span style={{ color: 'var(--success, #22C55E)', fontWeight: 600 }}>
                                {isArabic
                                  ? `أنت تستخدم أحدث إصدار v${updateState.currentVersion} ✓`
                                  : `You are on the latest version v${updateState.currentVersion} ✓`}
                              </span>
                              <span style={{ fontSize: '11px', opacity: 0.75 }}>
                                ({isArabic ? 'تم الفحص من GitHub:' : 'Checked via GitHub:'} {updateState.checkedAt})
                              </span>
                            </div>
                          )
                        ) : (
                          <span>
                            {isArabic
                              ? 'التحقق من أحدث إصدار منشور على المستودع الرسمي.'
                              : 'Check the official repository for the latest published version.'}
                          </span>
                        )}
                      </div>
                    </div>

                    <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', alignItems: 'center', marginTop: '2px' }}>
                      {updateState.checked && updateState.hasUpdate && (
                        <button
                          type="button"
                          className="btn btn-primary"
                          onClick={handleDownloadUpdate}
                          disabled={!updateState.sha256}
                          style={{
                            minWidth: '150px',
                            fontWeight: 700,
                            padding: '8px 18px',
                            backgroundColor: 'var(--accent)',
                            boxShadow: '0 4px 14px rgba(59, 130, 246, 0.4)'
                          }}
                        >
                          <span style={{ display: 'inline-flex', alignItems: 'center', gap: '6px' }}>
                            <Download size={14} />
                            <span>{isArabic ? 'تثبيت التحديث الآن' : 'Install Update Now'}</span>
                          </span>
                        </button>
                      )}

                      {updateState.checked && (
                        <button
                          type="button"
                          className="btn btn-secondary"
                          onClick={() => api.openExternalUrl(updateState.htmlUrl || 'https://github.com/dahakabdellah12/BrawlCraft#download')}
                          style={{ fontWeight: 600, padding: '8px 14px' }}
                        >
                          {isArabic ? 'صفحة التحميل ↗' : 'Download Page ↗'}
                        </button>
                      )}

                      <button
                        type="button"
                        className="btn btn-secondary"
                        onClick={handleCheckUpdates}
                        disabled={updateState.loading}
                        style={{
                          minWidth: '115px',
                          fontWeight: 600,
                          padding: '8px 16px',
                          color: updateState.checked && !updateState.hasUpdate && !updateState.error ? 'var(--success, #22C55E)' : 'inherit',
                          borderColor: updateState.checked && !updateState.hasUpdate && !updateState.error ? 'rgba(34, 197, 94, 0.4)' : 'var(--border-primary)'
                        }}
                      >
                        {updateState.loading
                          ? (isArabic ? 'جاري الفحص...' : 'Checking...')
                          : updateState.checked
                            ? (isArabic ? 'إعادة الفحص' : 'Check Again')
                            : (isArabic ? 'فحص الآن' : 'Check Now')}
                      </button>
                    </div>
                  </div>

                </div>
              </section>
            )}
          </div>
        </main>
      </div>

      <style>{`
        @keyframes settingsFadeIn {
          from { opacity: 0; transform: scale(0.98); }
          to { opacity: 1; transform: scale(1); }
        }

        .nav-item {
          width: 100%;
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 11px 12px;
          color: var(--text-secondary);
          background: transparent;
          border: 1px solid transparent;
          border-radius: 8px;
          cursor: pointer;
          text-align: left;
          transition: all 0.15s ease;
          font-size: 13px;
          font-weight: 500;
        }

        .nav-item:hover {
          background: var(--bg-hover);
          color: var(--text-primary);
        }

        .nav-item.active {
          color: var(--accent-light);
          background: var(--accent-soft);
          border-color: rgba(59, 130, 246, 0.3);
          font-weight: 600;
        }

        .nav-icon {
          width: 18px;
          text-align: center;
          font-size: 16px;
          flex-shrink: 0;
        }

        .card-group {
          background: var(--bg-card);
          border: 1px solid var(--border-primary);
          border-radius: 11px;
          padding: 0 18px;
        }

        .setting {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 20px;
          min-height: 68px;
          border-bottom: 1px solid var(--border-primary);
          padding: 10px 0;
        }

        .setting:last-child {
          border-bottom: none;
        }

        .setting-info {
          min-width: 0;
          flex: 1;
        }

        .setting-name {
          font-size: 13px;
          font-weight: 600;
          color: var(--text-primary);
          margin-bottom: 4px;
        }

        .setting-desc {
          color: var(--text-secondary);
          font-size: 12px;
          line-height: 1.45;
        }

        /* Custom Dropdown (Platform Colors) */
        .dropdown {
          position: relative;
          width: 240px;
        }

        .dropdown-button {
          display: flex;
          justify-content: space-between;
          align-items: center;
          width: 100%;
          padding: 11px 14px;
          background: var(--bg-card);
          color: var(--text-primary);
          border: 1px solid var(--border-primary);
          border-radius: 8px;
          font-size: 13px;
          font-family: inherit;
          cursor: pointer;
          transition: border-color .2s, background-color .2s;
        }

        .dropdown-button:hover,
        .dropdown.open .dropdown-button {
          border-color: var(--accent);
        }

        .arrow {
          width: 8px;
          height: 8px;
          border-right: 1.5px solid var(--text-secondary);
          border-bottom: 1.5px solid var(--text-secondary);
          transform: rotate(45deg) translateY(-2px);
          transition: transform .2s ease;
          display: inline-block;
          flex-shrink: 0;
        }

        .dropdown.open .arrow {
          transform: rotate(225deg) translate(-2px, -1px);
        }

        .dropdown-options {
          position: absolute;
          top: calc(100% + 6px);
          left: 0;
          right: 0;
          padding: 5px;
          background: var(--bg-card);
          border: 1px solid var(--border-primary);
          border-radius: 8px;
          box-shadow: 0 10px 28px rgba(0, 0, 0, 0.65);
          display: none;
          z-index: 50;
        }

        .dropdown.open .dropdown-options {
          display: block;
          animation: appear .15s ease;
        }

        .option {
          display: flex;
          justify-content: space-between;
          align-items: center;
          width: 100%;
          padding: 10px 12px;
          border: none;
          border-radius: 6px;
          background: transparent;
          color: var(--text-secondary);
          text-align: start;
          font-size: 13px;
          font-family: inherit;
          cursor: pointer;
          transition: .15s ease;
        }

        .option:hover {
          background: var(--bg-hover);
          color: var(--text-primary);
        }

        .option.selected {
          color: var(--accent-light);
          background: var(--accent-soft);
          font-weight: 600;
        }

        .check {
          opacity: 0;
          color: var(--accent-light);
          font-size: 13px;
          font-weight: bold;
        }

        .option.selected .check {
          opacity: 1;
        }

        @keyframes appear {
          from {
            opacity: 0;
            transform: translateY(-4px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }

        .settings-close-btn {
          background: transparent;
          border: 1px solid var(--border-primary);
          color: var(--text-secondary);
          cursor: pointer;
          width: 32px;
          height: 32px;
          border-radius: 8px;
          display: flex;
          align-items: center;
          justify-content: center;
          transition: all 0.15s ease;
          padding: 0;
        }

        .settings-close-btn:hover:not(:disabled) {
          background: var(--bg-hover);
          color: var(--text-primary);
          border-color: var(--border-secondary);
        }

        .settings-close-btn:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        /* Button styles */
        .btn {
          padding: 8px 15px;
          border-radius: 7px;
          border: 1px solid var(--border-primary);
          cursor: pointer;
          transition: all 0.15s ease;
          font-weight: 600;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 6px;
        }

        .btn-secondary {
          color: var(--text-primary);
          background: var(--button-secondary);
        }

        .btn-secondary:hover:not(:disabled) {
          background: var(--button-secondary-hover);
          border-color: var(--border-secondary);
        }

        .btn-primary {
          color: var(--text-on-accent);
          background: var(--accent);
          border-color: var(--accent);
        }

        .btn-primary:hover:not(:disabled) {
          background: var(--accent-hover);
        }

        /* GitHub icon-only button */
        .github-icon-btn {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 38px;
          height: 38px;
          border-radius: 8px;
          border: 1px solid var(--border-primary);
          background: var(--button-secondary);
          color: var(--text-secondary);
          cursor: pointer;
          transition: all 0.15s ease;
          flex-shrink: 0;
          padding: 0;
        }

        .github-icon-btn:hover {
          background: var(--bg-hover);
          color: var(--text-primary);
          border-color: var(--border-secondary);
        }

        /* Toggle switch */
        .toggle-switch {
          position: relative;
          display: inline-flex;
          align-items: center;
          width: 44px;
          height: 24px;
          border-radius: 12px;
          border: none;
          cursor: pointer;
          flex-shrink: 0;
          transition: background 0.2s ease;
          padding: 0;
        }

        .toggle-switch.on {
          background: var(--accent);
        }

        .toggle-switch.off {
          background: var(--bg-active, #363636);
        }

        .toggle-switch:focus-visible {
          outline: 2px solid var(--focus-ring);
          outline-offset: 2px;
        }

        .toggle-thumb {
          position: absolute;
          width: 18px;
          height: 18px;
          border-radius: 50%;
          background: #fff;
          transition: left 0.2s ease;
          box-shadow: 0 1px 4px rgba(0,0,0,0.35);
        }

        .toggle-switch.on .toggle-thumb {
          left: calc(100% - 21px);
        }

        .toggle-switch.off .toggle-thumb {
          left: 3px;
        }

        @media (max-width: 720px) {
          .settings-sidebar {
            width: 64px !important;
            padding: 16px 6px !important;
          }
          .nav-item {
            justify-content: center !important;
            padding: 10px 0 !important;
          }
          .nav-text {
            display: none !important;
          }
          .nav-icon {
            font-size: 18px !important;
          }
          .settings-main > div {
            padding: 18px 16px !important;
          }
        }
      `}</style>
    </div>
  );
}