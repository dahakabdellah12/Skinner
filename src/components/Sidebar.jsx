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
  },
  {
    id: 'weapons',
    labelAr: 'سكنات الأسلحة',
    labelEn: 'Weapon Skins',
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
              className={`sidebar-item ${isActive ? 'active' : ''} ${tab.disabled ? 'sidebar-item-disabled' : ''}`}
              onClick={() => {
                if (!tab.disabled && onSelectTab) {
                  onSelectTab(tab.id);
                }
              }}
              title={tab.disabled ? (isArabic ? 'معطل حالياً للتركيز على المنصة وسكنات الشخصيات' : 'Disabled to focus on platform and character skins') : ''}
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
