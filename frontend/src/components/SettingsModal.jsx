import React, { useState, useEffect, useRef } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import api from '../services/api';
import { clearAssetImageCache } from './AssetImage';

export default function SettingsModal({
  isOpen,
  onClose,
  onRestoreEverything,
  onReloadCatalog,
  showToast,
  isBusy
}) {
  const { lang, setLang, isRTL } = useLanguage();
  const [activeTab, setActiveTab] = useState('language');
  const [confirmResetStock, setConfirmResetStock] = useState(false);
  const [isLangDropdownOpen, setIsLangDropdownOpen] = useState(false);
  const langDropdownRef = useRef(null);

  // Maintenance & Troubleshooting State
  const [repairLoading, setRepairLoading] = useState(false);
  const [repairResult, setRepairResult] = useState(null);
  const [cacheLoading, setCacheLoading] = useState(false);
  const [cacheResult, setCacheResult] = useState(null);

  // Real Software Update State
  const [updateState, setUpdateState] = useState({
    loading: false,
    checked: false,
    hasUpdate: false,
    currentVersion: '2.0.0',
    latestVersion: '2.0.0',
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
    setRepairResult(null);
    setCacheResult(null);
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
    setRepairResult(null);
    try {
      clearAssetImageCache();
      const res = await api.repairGameInstallation();
      if (res && res.success) {
        if (onReloadCatalog) await onReloadCatalog();
        setRepairResult({
          success: true,
          message: isRTL
            ? 'تم الفحص الشامل وإصلاح كافة المشاكل بنجاح! تم تنظيف الكاش وإعادة ربط أصول اللعبة.'
            : 'Diagnostic & auto-repair completed successfully! Assets refreshed and cache cleared.',
          details: `${isRTL ? 'الأساطير' : 'Legends'}: ${res.legendCount || 70} | ${isRTL ? 'المظاهر' : 'Costumes'}: ${res.costumeCount || 795} | ${isRTL ? 'الملفات المفحوصة' : 'Files Checked'}: ${res.swfCount || 564}`
        });
        showToast?.(
          isRTL ? 'تم إصلاح المشكلات وتحديث أصول اللعبة بنجاح ✓' : 'Troubleshooting complete: Assets & paths repaired! ✓',
          'success'
        );
      } else {
        setRepairResult({
          success: false,
          message: res?.message || (isRTL ? 'تعذر إتمام الإصلاح التلقائي.' : 'Failed to complete auto-repair.')
        });
        showToast?.(res?.message || (isRTL ? 'فشل فحص وإصلاح الملفات' : 'Repair failed'), 'error');
      }
    } catch (err) {
      setRepairResult({
        success: false,
        message: err.message
      });
      showToast?.(err.message, 'error');
    } finally {
      setRepairLoading(false);
    }
  };

  const handleClearCache = async () => {
    if (cacheLoading || isBusy) return;
    setCacheLoading(true);
    setCacheResult(null);
    try {
      clearAssetImageCache();
      await api.clearAssetCache();
      setCacheResult({
        success: true,
        message: isRTL
          ? 'تم تفريغ ذاكرة التخزين المؤقت للصور بنجاح! ستتم إعادة توليد الرسوم بأحدث دقة.'
          : 'Image preview cache cleared! All visuals will refresh fresh.'
      });
      showToast?.(isRTL ? 'تم تفريغ كاش الصور بنجاح ✓' : 'Visual cache cleared! ✓', 'success');
    } catch (err) {
      setCacheResult({
        success: false,
        message: err.message
      });
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
          currentVersion: res.currentVersion || '2.0.0',
          latestVersion: res.latestVersion || '2.0.0',
          releaseTitle: res.releaseTitle || '',
          downloadUrl: res.downloadUrl || 'https://github.com/dahakabdellah12/BrawlCraft/releases',
          htmlUrl: res.htmlUrl || 'https://github.com/dahakabdellah12/BrawlCraft/releases',
          assetName: res.assetName || null,
          assetSizeFormatted: res.assetSizeFormatted || null,
          releaseNotes: res.releaseNotes || '',
          repo: res.repo || 'dahakabdellah12/BrawlCraft',
          repoUrl: res.repoUrl || 'https://github.com/dahakabdellah12/BrawlCraft',
          checkedAt: res.checkedAt || new Date().toLocaleTimeString(),
          error: res.networkError
            ? (isRTL ? 'تعذر الاتصال بـ GitHub Releases. تحقق من اتصال الإنترنت.' : 'Could not reach GitHub Releases. Check internet connection.')
            : ''
        });
        if (res.hasUpdate) {
          showToast?.(
            isRTL ? `يتوفر إصدار جديد على GitHub: v${res.latestVersion}!` : `New update available on GitHub: v${res.latestVersion}!`,
            'info'
          );
        } else if (!res.networkError) {
          showToast?.(
            isRTL ? 'أنت تستخدم أحدث إصدار v2.0.0 من GitHub ✓' : 'You are running the latest v2.0.0 release from GitHub ✓',
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
    if (updateState.downloadUrl) {
      api.openExternalUrl(updateState.downloadUrl);
    }
  };

  const pageMeta = {
    language: {
      title: isRTL ? 'لغة الواجهة' : 'Language'
    },
    maintenance: {
      title: isRTL ? 'الصيانة والاستعادة' : 'Maintenance & Reset'
    },
    about: {
      title: isRTL ? 'حول التطبيق' : 'About BrawlCraft'
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
              <span className="nav-text">{isRTL ? 'اللغة' : 'Language'}</span>
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
              <span className="nav-text">{isRTL ? 'الصيانة والاستعادة' : 'Maintenance'}</span>
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
              <span className="nav-text">{isRTL ? 'حول التطبيق' : 'About'}</span>
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
                      <div className="setting-name">{isRTL ? 'اللغة المختارة' : 'Selected Language'}</div>
                      <div className="setting-desc">
                        {isRTL ? 'التبديل الفوري بين اللغة العربية والإنجليزية.' : 'Instantly toggle between Arabic and English.'}
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
                        <span id="selected-label">{lang === 'ar' ? 'العربية' : 'English'}</span>
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
                          <span>🔧</span>
                          <span>{isRTL ? 'إصلاح وتشخيص شامل وحل المشاكل (Auto-Repair)' : 'Diagnostic & Auto-Repair'}</span>
                        </div>
                        <div className="setting-desc" style={{ marginTop: '4px' }}>
                          {isRTL
                            ? 'خيار ذكي لحل أي مشاكل في اختفاء الشخصيات، البطاقات الفارغة، تلف الكاش، أو خطأ قراءة ملفات SWF، مع إعادة تهيئة ومزامنة كافة أصول اللعبة تلقائياً.'
                            : 'Smart diagnostic tool that fixes missing character portraits, empty cards, corrupted cache, and SWF read issues by resynchronizing all game assets.'}
                        </div>
                        {repairResult && (
                          <div
                            style={{
                              marginTop: '10px',
                              padding: '10px 14px',
                              borderRadius: '8px',
                              backgroundColor: repairResult.success ? 'rgba(16, 185, 129, 0.12)' : 'rgba(239, 68, 68, 0.12)',
                              border: `1px solid ${repairResult.success ? 'rgba(16, 185, 129, 0.35)' : 'rgba(239, 68, 68, 0.35)'}`,
                              fontSize: '12px',
                              color: repairResult.success ? 'var(--success, #10B981)' : 'var(--danger, #EF4444)'
                            }}
                          >
                            <div style={{ fontWeight: 700 }}>{repairResult.message}</div>
                            {repairResult.details && <div style={{ fontSize: '11px', marginTop: '4px', opacity: 0.85 }}>{repairResult.details}</div>}
                          </div>
                        )}
                      </div>

                      <button
                        type="button"
                        className="btn btn-primary"
                        onClick={handleRunDiagnosticRepair}
                        disabled={repairLoading || isBusy}
                        style={{ whiteSpace: 'nowrap', fontWeight: 700, minWidth: '130px', padding: '9px 18px', marginTop: '2px' }}
                      >
                        {repairLoading ? (isRTL ? 'جاري الفحص...' : 'Diagnosing...') : (isRTL ? 'فحص وإصلاح الآن' : 'Run Auto-Repair')}
                      </button>
                    </div>

                    {/* Item 2: Clear Visual Cache */}
                    <div className="setting" style={{ alignItems: 'flex-start' }}>
                      <div className="setting-info" style={{ flex: 1 }}>
                        <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                          <span>🧹</span>
                          <span>{isRTL ? 'تفريغ كاش الصور وتحديث الرسوم' : 'Clear Visual Cache & Refresh'}</span>
                        </div>
                        <div className="setting-desc" style={{ marginTop: '4px' }}>
                          {isRTL
                            ? 'مسح التخزين المؤقت لرسوم الشخصيات والأسلحة في الذاكرة لإعادة توليد صور البطاقات بأعلى دقة متوفرة.'
                            : 'Purges all in-memory character, costume, and weapon vector previews to trigger crisp, high-res rendering.'}
                        </div>
                        {cacheResult && (
                          <div
                            style={{
                              marginTop: '10px',
                              padding: '8px 12px',
                              borderRadius: '8px',
                              backgroundColor: 'rgba(16, 185, 129, 0.12)',
                              border: '1px solid rgba(16, 185, 129, 0.35)',
                              fontSize: '12px',
                              color: 'var(--success, #10B981)',
                              fontWeight: 600
                            }}
                          >
                            {cacheResult.message}
                          </div>
                        )}
                      </div>

                      <button
                        type="button"
                        className="btn btn-secondary"
                        onClick={handleClearCache}
                        disabled={cacheLoading || isBusy}
                        style={{ whiteSpace: 'nowrap', fontWeight: 600, minWidth: '130px', padding: '9px 18px', marginTop: '2px' }}
                      >
                        {cacheLoading ? (isRTL ? 'جاري التفريغ...' : 'Clearing...') : (isRTL ? 'تفريغ الكاش' : 'Clear Cache')}
                      </button>
                    </div>

                    {/* Item 3: Restore Original Game Files */}
                    <div className="setting" style={{ alignItems: 'flex-start' }}>
                      <div className="setting-info" style={{ flex: 1 }}>
                        <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                          <span>🔄</span>
                          <span>{isRTL ? 'استعادة ملفات اللعبة الأصلية (Stock Restore)' : 'Restore Original Game Files'}</span>
                        </div>
                        <div className="setting-desc" style={{ marginTop: '4px' }}>
                          {isRTL
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
                        {isRTL ? 'إعادة كل شيء للأصل' : 'Restore Stock'}
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
                      {isRTL ? 'هل تريد بالتأكيد إعادة كل شيء إلى الأصل؟' : 'Are you sure you want to restore everything to original?'}
                    </h2>
                    <p style={{ fontSize: '13px', color: 'var(--text-secondary)', maxWidth: '440px', lineHeight: 1.5, margin: 0 }}>
                      {isRTL
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
                        {isBusy ? (isRTL ? 'جاري الاستعادة...' : 'Restoring...') : (isRTL ? 'نعم' : 'Yes')}
                      </button>
                      <button
                        type="button"
                        className="btn btn-secondary"
                        onClick={() => setConfirmResetStock(false)}
                        disabled={isBusy}
                        style={{ minWidth: '96px', padding: '10px 24px', fontSize: '14px' }}
                      >
                        {isRTL ? 'لا' : 'No'}
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
                      <div className="setting-name">{isRTL ? 'اسم التطبيق' : 'Application Name'}</div>
                      <div className="setting-desc">BrawlCraft — Ultimate Open-Source Cosmetic Studio for Brawlhalla</div>
                    </div>
                    <span style={{ fontWeight: 700, color: 'var(--accent-light)', fontSize: '13px' }}>
                      BrawlCraft
                    </span>
                  </div>

                  <div className="setting">
                    <div className="setting-info">
                      <div className="setting-name">{isRTL ? 'رقم الإصدار' : 'Application Version'}</div>
                      <div className="setting-desc">{isRTL ? 'الإصدار المكتبي المستقر مفتوح المصدر.' : 'Installed open-source desktop release.'}</div>
                    </div>
                    <span style={{ color: 'var(--text-muted)', fontSize: '13px', fontWeight: 600 }}>v2.0.0</span>
                  </div>

                  <div className="setting">
                    <div className="setting-info">
                      <div className="setting-name">{isRTL ? 'معمارية النواة والترخيص' : 'Architecture & License'}</div>
                      <div className="setting-desc">.NET 8 (WPF x64) + WebView2 + React 18 · MIT Open Source</div>
                    </div>
                    <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>MIT License</span>
                  </div>

                  {/* OPEN SOURCE GITHUB ROW */}
                  <div className="setting" style={{ alignItems: 'center' }}>
                    <div className="setting-info" style={{ flex: 1 }}>
                      <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span>📂</span>
                        <span>{isRTL ? 'المستودع المفتوح المصدر (GitHub)' : 'Open Source Repository (GitHub)'}</span>
                      </div>
                      <div className="setting-desc">
                        {isRTL
                          ? 'كود البرنامج بالكامل مفتوح المصدر ومتاح مجاناً على منصة GitHub للجميع.'
                          : 'Full source code is publicly available under the open-source MIT License on GitHub.'}
                        <div style={{ marginTop: '4px', fontFamily: 'monospace', color: 'var(--accent-light)', fontSize: '12px' }}>
                          github.com/dahakabdellah12/BrawlCraft
                        </div>
                      </div>
                    </div>
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => api.openExternalUrl('https://github.com/dahakabdellah12/BrawlCraft')}
                      style={{ minWidth: '135px', fontWeight: 600, padding: '8px 16px' }}
                    >
                      {isRTL ? 'عرض على GitHub ↗' : 'View on GitHub ↗'}
                    </button>
                  </div>

                  {/* REAL GITHUB RELEASES UPDATE CHECKER */}
                  <div className="setting" style={{ alignItems: 'flex-start', padding: '14px 0' }}>
                    <div className="setting-info" style={{ flex: 1 }}>
                      <div className="setting-name" style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span>🚀</span>
                        <span>{isRTL ? 'تحديثات البرنامج (GitHub Releases)' : 'Software Updates (GitHub Releases)'}</span>
                      </div>
                      <div className="setting-desc" style={{ marginTop: '4px' }}>
                        {updateState.loading ? (
                          <span style={{ color: 'var(--accent-light)' }}>
                            {isRTL ? 'جاري الاتصال بـ GitHub Releases للتحقق من أحدث إصدار...' : 'Connecting to GitHub Releases to check for updates...'}
                          </span>
                        ) : updateState.checked ? (
                          updateState.hasUpdate ? (
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                              <span style={{ color: 'var(--gold, #F5C542)', fontWeight: 700 }}>
                                {isRTL
                                  ? `يتوفر إصدار جديد مفتوح المصدر: v${updateState.latestVersion}!`
                                  : `A new open-source release is available: v${updateState.latestVersion}!`}
                              </span>
                              {updateState.releaseTitle && (
                                <span style={{ fontSize: '12px', color: 'var(--text-primary)' }}>
                                  {updateState.releaseTitle}
                                </span>
                              )}
                              {updateState.assetName && (
                                <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                                  {isRTL ? 'حزمة التثبيت:' : 'Setup Package:'} {updateState.assetName} {updateState.assetSizeFormatted ? `(${updateState.assetSizeFormatted})` : ''}
                                </span>
                              )}
                            </div>
                          ) : updateState.error ? (
                            <span style={{ color: 'var(--danger, #EF4444)' }}>
                              {updateState.error}
                            </span>
                          ) : (
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                              <span style={{ color: 'var(--success, #10B981)', fontWeight: 600 }}>
                                {isRTL
                                  ? `أنت تستخدم أحدث إصدار مستقر من GitHub v${updateState.currentVersion} ✓`
                                  : `You are running the latest GitHub release v${updateState.currentVersion} ✓`}
                              </span>
                              <span style={{ fontSize: '11px', opacity: 0.75 }}>
                                ({isRTL ? 'تم الفحص من GitHub:' : 'Checked via GitHub:'} {updateState.checkedAt})
                              </span>
                            </div>
                          )
                        ) : (
                          <span>
                            {isRTL
                              ? 'فحص أحدث الإصدارات المنشورة مباشرة على مستودع GitHub الرسمي (GitHub Releases API).'
                              : 'Automatically check for official releases published on GitHub Releases.'}
                          </span>
                        )}
                      </div>
                    </div>

                    <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', alignItems: 'center' }}>
                      {updateState.checked && updateState.hasUpdate && (
                        <button
                          type="button"
                          className="btn btn-primary"
                          onClick={handleDownloadUpdate}
                          style={{
                            minWidth: '120px',
                            fontWeight: 700,
                            padding: '8px 16px',
                            backgroundColor: 'var(--accent)',
                            boxShadow: '0 4px 14px rgba(0, 150, 255, 0.4)'
                          }}
                        >
                          {isRTL ? 'تحميل التحديث 📥' : 'Download 📥'}
                        </button>
                      )}

                      {updateState.checked && (
                        <button
                          type="button"
                          className="btn btn-secondary"
                          onClick={() => api.openExternalUrl(updateState.htmlUrl || 'https://github.com/dahakabdellah12/BrawlCraft/releases')}
                          style={{ minWidth: '110px', fontWeight: 600, padding: '8px 14px' }}
                        >
                          {isRTL ? 'سجل الإصدارات ↗' : 'Releases ↗'}
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
                          color: updateState.checked && !updateState.hasUpdate && !updateState.error ? 'var(--success, #10B981)' : 'inherit',
                          borderColor: updateState.checked && !updateState.hasUpdate && !updateState.error ? 'rgba(16, 185, 129, 0.4)' : 'var(--border-primary)'
                        }}
                      >
                        {updateState.loading
                          ? (isRTL ? 'جاري الفحص...' : 'Checking...')
                          : updateState.checked
                            ? (isRTL ? 'إعادة الفحص' : 'Check Again')
                            : (isRTL ? 'فحص من GitHub 🚀' : 'Check GitHub 🚀')}
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