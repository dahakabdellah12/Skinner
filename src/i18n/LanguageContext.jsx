import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import { translations } from './translations';

const LanguageContext = createContext(null);

const STORAGE_KEY = 'brawlhalla_skin_studio_language';

export function LanguageProvider({ children }) {
  const [lang, setLangState] = useState(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved === 'ar' || saved === 'en') return saved;
    } catch { }
    // Default to 'ar' if user asked for Arabic, or 'en'
    return 'ar';
  });

  const isArabic = lang === 'ar';

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, lang);
    } catch { }

    if (typeof document !== 'undefined') {
      document.documentElement.lang = lang;
      document.documentElement.dir = 'ltr';
    }
  }, [lang]);

  const setLang = useCallback((newLang) => {
    if (newLang === 'ar' || newLang === 'en') {
      setLangState(newLang);
    }
  }, []);

  const toggleLang = useCallback(() => {
    setLangState((prev) => (prev === 'ar' ? 'en' : 'ar'));
  }, []);

  const t = useCallback((key, params = {}) => {
    const dict = translations[lang] || translations.en;
    let text = dict[key] ?? translations.en[key] ?? key;

    if (params && typeof params === 'object') {
      Object.keys(params).forEach((p) => {
        text = text.replace(new RegExp(`\\{${p}\\}`, 'g'), params[p]);
      });
    }

    return text;
  }, [lang]);

  const tWeapon = useCallback((weaponName) => {
    if (!weaponName) return '';
    const key = `weapon_${weaponName.replace(/\s+/g, '')}`;
    const dict = translations[lang] || translations.en;
    return dict[key] || translations.en[key] || weaponName;
  }, [lang]);

  const value = {
    lang,
    setLang,
    toggleLang,
    isArabic,
    t,
    tWeapon
  };

  return (
    <LanguageContext.Provider value={value}>
      {children}
    </LanguageContext.Provider>
  );
}

// oxlint-disable-next-line react/only-export-components
export function useLanguage() {
  const context = useContext(LanguageContext);
  if (!context) {
    throw new Error('useLanguage must be used within a LanguageProvider');
  }
  return context;
}

export default LanguageContext;
