import React from 'react';
import { useLanguage } from '../i18n/LanguageContext';

export const SIDEBAR_TABS = [
  {
    id: 'legends',
    labelAr: 'الأساطير',
    labelEn: 'Legends',
    disabled: false
  },
  {
    id: 'ko_effects',
    labelAr: 'تأثيرات الإطاحة',
    labelEn: 'KO Effects',
    disabled: false
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
              className={`sidebar-item ${isActive ? 'active' : ''} ${isTabDisabled ? 'sidebar-item-disabled' : ''}`}
              style={disabled ? { opacity: 0.5, cursor: 'not-allowed', pointerEvents: 'none' } : undefined}
              onClick={() => {
                if (!isTabDisabled && onSelectTab) {
                  onSelectTab(tab.id);
                }
              }}
              title={disabled ? (isArabic ? 'جاري تجهيز الموارد...' : 'Loading resources...') : tab.disabled ? (isArabic ? 'معطل حالياً' : 'Disabled') : ''}
            >
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
