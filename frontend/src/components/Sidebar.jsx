import React from 'react';
import {
  Palette,
  Users,
  Sparkles,
  Sword,
  Play
} from 'lucide-react';
import { useLanguage } from '../i18n/LanguageContext';

export default function Sidebar({ currentTab, setTab, systemInfo, onLaunchGame }) {
  const { t } = useLanguage();

  const navItems = [
    { id: 'Studio', label: t('tab_studio'), icon: Palette, highlight: true },
    { id: 'Legends', label: t('tab_legends'), icon: Users },
    { id: 'Costumes', label: t('tab_costumes'), icon: Sparkles },
    { id: 'WeaponSkins', label: t('tab_weapons'), icon: Sword }
  ];

  return (
    <aside style={{
      width: 'var(--sidebar-width)',
      height: '100vh',
      backgroundColor: 'var(--bg-deep)',
      borderInlineEnd: '1px solid var(--border-default)',
      display: 'flex',
      flexDirection: 'column',
      flexShrink: 0
    }}>
      {/* Brand Header */}
      <div style={{ padding: '20px 18px 16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <div style={{
            width: '34px',
            height: '34px',
            borderRadius: '8px',
            background: 'var(--accent-gradient)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#FFFFFF'
          }}>
            <Palette size={20} />
          </div>
          <div>
            <div style={{ fontSize: '15px', fontWeight: 'bold', color: 'var(--text-primary)' }}>{t('app_title')}</div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>{t('app_subtitle')}</div>
          </div>
        </div>

        {/* Connection Status Pill */}
        <div style={{
          marginTop: '14px',
          padding: '6px 10px',
          backgroundColor: 'var(--bg-elevated)',
          borderRadius: 'var(--radius-sm)',
          border: '1px solid var(--border-default)',
          display: 'flex',
          flexDirection: 'column',
          gap: '4px',
          fontSize: '11px',
          color: 'var(--text-secondary)'
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{
              width: '8px',
              height: '8px',
              borderRadius: '50%',
              backgroundColor: systemInfo?.isValid ? 'var(--color-success)' : 'var(--color-danger)'
            }} />
            <span style={{ textOverflow: 'ellipsis', overflow: 'hidden', whiteSpace: 'nowrap' }}>
              {systemInfo?.isValid
                ? `${t('badge_game_ready')} (v${systemInfo.version || '11.00'})`
                : t('badge_not_detected')}
            </span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '2px' }}>
            <span style={{
              width: '8px',
              height: '8px',
              borderRadius: '50%',
              backgroundColor: systemInfo?.isGameRunning ? '#10B981' : '#6B7280'
            }} />
            <span style={{
              color: systemInfo?.isGameRunning ? '#34D399' : 'var(--text-muted)',
              fontWeight: systemInfo?.isGameRunning ? '600' : 'normal'
            }}>
              {systemInfo?.isGameRunning ? t('badge_process_running') : t('badge_game_idle')}
            </span>
          </div>
        </div>
      </div>

      {/* Navigation Links */}
      <nav style={{ flex: 1, padding: '0 8px', overflowY: 'auto' }}>
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = currentTab === item.id;
          return (
            <button
              key={item.id}
              onClick={() => setTab(item.id)}
              style={{
                width: '100%',
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
                padding: '10px 12px',
                margin: '2px 0',
                borderRadius: 'var(--radius-sm)',
                border: 'none',
                backgroundColor: isActive ? 'var(--bg-elevated)' : 'transparent',
                color: isActive
                  ? '#FFFFFF'
                  : item.highlight
                    ? 'var(--accent-blue)'
                    : 'var(--text-secondary)',
                fontWeight: isActive || item.highlight ? '600' : '500',
                fontSize: '13px',
                cursor: 'pointer',
                textAlign: 'start',
                transition: 'all var(--transition-fast)'
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.backgroundColor = 'var(--bg-hover)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.backgroundColor = 'transparent';
                  e.currentTarget.style.color = item.highlight ? 'var(--accent-blue)' : 'var(--text-secondary)';
                }
              }}
            >
              <Icon size={16} color={isActive ? 'var(--accent-blue)' : item.highlight ? 'var(--accent-blue)' : 'var(--text-muted)'} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>

      {/* Sidebar Footer */}
      <div style={{
        padding: '14px 16px',
        borderTop: '1px solid var(--border-default)',
        backgroundColor: 'var(--bg-deep)'
      }}>
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: '10px',
          fontSize: '12px'
        }}>
          <span style={{ color: 'var(--text-secondary)' }}>{t('label_active_mods')}</span>
          <span style={{
            backgroundColor: 'rgba(99, 102, 241, 0.2)',
            color: '#A5B4FC',
            border: '1px solid rgba(99, 102, 241, 0.4)',
            padding: '2px 8px',
            borderRadius: 'var(--radius-full)',
            fontWeight: 'bold',
            fontSize: '11px'
          }}>
            {systemInfo?.activeModCount || 0}
          </span>
        </div>

        <button
          className="btn btn-primary"
          style={{ width: '100%', padding: '10px 14px' }}
          onClick={onLaunchGame}
          title={systemInfo?.isGameRunning ? "Bring active Brawlhalla game window to foreground" : "Launch Brawlhalla via Steam"}
        >
          <Play size={14} fill="#FFFFFF" />
          <span>{systemInfo?.isGameRunning ? t('btn_focus_brawlhalla') : t('btn_launch_brawlhalla')}</span>
        </button>
      </div>
    </aside>
  );
}
