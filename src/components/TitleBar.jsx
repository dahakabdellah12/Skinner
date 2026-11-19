import React, { useState, useEffect } from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import { api } from '../services/api';
import appLogo from '../assets/logo.png';
import { ArrowUpCircle } from 'lucide-react';

export default function TitleBar({
  onOpenSettings,
  hasPendingUpdate,
  onOpenUpdate,
  isLoading = false
}) {
  const { isArabic } = useLanguage();
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    // Check initial state
    api.isWindowMaximized().then((max) => {
      if (typeof max === 'boolean') {
        setIsMaximized(max);
      }
    }).catch(() => { });

    // Listen for C# State Changed event
    window.__setWindowState = (max) => {
      setIsMaximized(Boolean(max));
    };

    const handleResize = () => {
      // In web fallback mode or fallback check
      if (window.innerWidth >= screen.availWidth && window.innerHeight >= screen.availHeight) {
        setIsMaximized(true);
      }
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.__setWindowState = null;
      window.removeEventListener('resize', handleResize);
    };
  }, []);

  const handleMinimize = async () => {
    try {
      await api.windowMinimize();
    } catch (err) {
      console.warn('Minimize failed:', err);
    }
  };

  const handleMaximize = async () => {
    try {
      const result = await api.windowMaximize();
      if (typeof result === 'boolean') {
        setIsMaximized(result);
      } else {
        setIsMaximized((prev) => !prev);
      }
    } catch (err) {
      console.warn('Maximize failed:', err);
      setIsMaximized((prev) => !prev);
    }
  };

  const handleClose = async () => {
    try {
      await api.windowClose();
    } catch (err) {
      console.warn('Close failed:', err);
    }
  };

  const handleMouseDown = (e) => {
    if (e.button === 0 && !e.target.closest('button') && !e.target.closest('input')) {
      api.invoke('windowDrag');
    }
  };

  const handleDoubleClick = (e) => {
    if (!e.target.closest('button') && !e.target.closest('input')) {
      handleMaximize();
    }
  };

  return (
    <header className="title-bar" onMouseDown={handleMouseDown} onDoubleClick={handleDoubleClick}>
      <div className="title-bar-left">
        <div className="app-icon" style={{ background: 'transparent' }}>
          <img
            src={appLogo}
            alt="Logo"
            style={{
              width: '26px',
              height: '26px',
              objectFit: 'contain',
              display: 'block'
            }}
          />
        </div>
        <span className="app-title" style={{ fontSize: '14px', fontWeight: 700, letterSpacing: '-0.2px' }}>
          Skinner
        </span>

        {/* Gear icon button right next to Skinner title: no text, no circle/pill, interactive */}
        <button
          type="button"
          className="title-bar-settings-btn"
          onClick={() => {
            if (!isLoading && onOpenSettings) {
              onOpenSettings();
            }
          }}
          disabled={isLoading}
          style={isLoading ? { opacity: 0.4, cursor: 'not-allowed', pointerEvents: 'none' } : undefined}
          title={isLoading ? (isArabic ? 'جاري تجهيز الموارد...' : 'Loading resources...') : (isArabic ? 'الإعدادات' : 'Settings')}
          aria-label="Settings"
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
        </button>

        

        {hasPendingUpdate && (
          <button
            type="button"
            className="title-bar-update-btn"
            onClick={onOpenUpdate}
            title={isArabic ? 'تحديث متوفر - انقر لفتح نافذة التحديث' : 'Update Available - Click to view and install'}
            aria-label="Update"
          >
            <ArrowUpCircle size={14} />
            <span>{isArabic ? 'تحديث' : 'Update'}</span>
          </button>
        )}
      </div>

      {/* Drag region spacer */}
      <div style={{ flex: 1 }} />

      <div className="title-bar-actions">
        <div className="window-controls">
          <button
            className="window-button"
            onClick={handleMinimize}
            title={isArabic ? 'تصغير' : 'Minimize'}
            aria-label="Minimize"
          >
            <svg width="11" height="1" viewBox="0 0 11 1" fill="currentColor">
              <rect width="11" height="1"/>
            </svg>
          </button>

          {/* Maximize / Restore button with animated state change */}
          <button
            className={`window-button ${isMaximized ? 'is-maximized' : ''}`}
            onClick={handleMaximize}
            title={isMaximized ? (isArabic ? 'استعادة' : 'Restore') : (isArabic ? 'تكبير' : 'Maximize')}
            aria-label={isMaximized ? 'Restore' : 'Maximize'}
          >
            {isMaximized ? (
              <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="currentColor" strokeWidth="1.1" className="window-restore-icon">
                <path d="M3 3V1H10V8H8" />
                <rect x="1" y="3" width="7" height="7" fill="var(--bg-secondary)" />
              </svg>
            ) : (
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.1" className="window-maximize-icon">
                <rect x="0.5" y="0.5" width="9" height="9"/>
              </svg>
            )}
          </button>

          <button
            className="window-button close"
            onClick={handleClose}
            title={isArabic ? 'إغلاق' : 'Close'}
            aria-label="Close"
          >
            <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" strokeWidth="1.2">
              <line x1="1" y1="1" x2="9" y2="9"/>
              <line x1="9" y1="1" x2="1" y2="9"/>
            </svg>
          </button>
        </div>
      </div>
    </header>
  );
}
