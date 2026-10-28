import React from 'react';
import { Play, RotateCcw, Languages } from 'lucide-react';
import { useLanguage } from '../i18n/LanguageContext';

export default function Header({ currentTab, systemInfo, onLaunchGame, onRestartGame, onRestoreAll, isBusy }) {
  const { t, toggleLang, lang } = useLanguage();

  const titles = {
    Studio: { title: t('title_studio'), subtitle: t('subtitle_studio') },
    Legends: { title: t('title_legends'), subtitle: t('subtitle_legends') },
    Costumes: { title: t('title_costumes'), subtitle: t('subtitle_costumes') },
    WeaponSkins: { title: t('title_weapons'), subtitle: t('subtitle_weapons') }
  };

  const meta = titles[currentTab] || { title: currentTab, subtitle: '' };

  return (
    <header style={{
      height: '64px',
      padding: '0 32px',
      borderBottom: '1px solid var(--border-default)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      backgroundColor: 'var(--bg-main)',
      flexShrink: 0
    }}>
      <div>
        <h2 style={{ fontSize: '18px', fontWeight: 'bold' }}>{meta.title}</h2>
        <p style={{ fontSize: '12px', color: 'var(--text-muted)' }}>{meta.subtitle}</p>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
        {systemInfo?.isGameRunning && (
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            padding: '4px 10px',
            backgroundColor: 'rgba(16, 185, 129, 0.15)',
            border: '1px solid rgba(16, 185, 129, 0.4)',
            borderRadius: 'var(--radius-full)',
            color: '#34D399',
            fontSize: '11px',
            fontWeight: '600'
          }}>
            <span style={{ width: '7px', height: '7px', borderRadius: '50%', backgroundColor: '#10B981', display: 'inline-block' }} />
            <span>{t('badge_game_running')}</span>
          </div>
        )}

        {/* Language Switcher */}
        <button
          className="btn btn-secondary"
          onClick={toggleLang}
          title={lang === 'ar' ? 'Switch to English' : 'التبديل إلى العربية'}
          style={{ gap: '6px' }}
        >
          <Languages size={15} />
          <span>{t('lang_toggle')}</span>
        </button>

        {systemInfo?.isGameRunning && (
          <button
            className="btn btn-secondary"
            onClick={onRestartGame}
            disabled={isBusy}
            title={t('btn_restart_game_tooltip')}
            style={{ borderColor: 'rgba(99, 102, 241, 0.4)', color: '#A5B4FC' }}
          >
            <RotateCcw size={14} className={isBusy ? "animate-spin" : ""} />
            <span>{t('btn_restart_game')}</span>
          </button>
        )}

        <button
          className="btn btn-secondary"
          onClick={onRestoreAll}
          disabled={isBusy}
          title={t('btn_restore_pristine_tooltip')}
        >
          <RotateCcw size={14} />
          <span>{t('btn_restore_pristine')}</span>
        </button>

        <button
          className="btn btn-primary"
          onClick={onLaunchGame}
          disabled={isBusy}
          title={systemInfo?.isGameRunning ? "Bring running Brawlhalla window to front" : "Launch Brawlhalla via Steam"}
        >
          <Play size={14} fill="#FFFFFF" />
          <span>{systemInfo?.isGameRunning ? t('btn_focus_game') : t('btn_launch_game')}</span>
        </button>
      </div>
    </header>
  );
}
