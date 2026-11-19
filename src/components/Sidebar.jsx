import React from 'react';
import { useLanguage } from '../i18n/LanguageContext';

export const SIDEBAR_TABS = [
  {
    id: 'legends',
    labelAr: 'الأساطير',
    labelEn: 'Legends',
    disabled: false,
    icon: (
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
        <circle cx="12" cy="7" r="4" />
      </svg>
    )
  },
  {
    id: 'ko_effects',
    labelAr: 'تأثيرات الإطاحة',
    labelEn: 'KO Effects',
    disabled: false,
    icon: (
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
      </svg>
    )
  },
  {
    id: 'weapons',
    labelAr: 'سكنات الأسلحة (معطل)',
    labelEn: 'Weapon Skins (Disabled)',
    disabled: true,
    icon: (
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5" />
        <line x1="13" y1="19" x2="19" y2="13" />
        <line x1="16" y1="16" x2="20" y2="20" />
        <line x1="19" y1="21" x2="21" y2="19" />
      </svg>
    )
  }
];

export default function Sidebar({
  activeTab = 'legends',
  onSelectTab,
  disabled = false
}) {
  const { isArabic } = useLanguage();

  return (
    <aside className="sidebar-container" aria-label="Navigation Sidebar">
      <nav className="sidebar-nav">
        {SIDEBAR_TABS.map((tab) => {
          const isActive = activeTab === tab.id;
          const isTabDisabled = disabled || tab.disabled;

          return (
            <button
              key={tab.id}
              type="button"
              disabled={isTabDisabled}
              className={`sidebar-item ${isActive ? 'active' : ''} ${tab.disabled ? 'sidebar-item-disabled' : ''}`}
              onClick={() => {
                if (!tab.disabled && onSelectTab) {
                  onSelectTab(tab.id);
                }
              }}
              title={tab.disabled ? (isArabic ? 'معطل حالياً للتركيز على المنصة وسكنات الشخصيات' : 'Disabled to focus on platform and character skins') : ''}
            >
              <span className="sidebar-icon">
                {tab.icon}
              </span>
              <span className="sidebar-item-label">
                {isArabic ? tab.labelAr : tab.labelEn}
              </span>
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
