import { createInstance } from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import zh from './locales/zh-CN.json';
import en from './locales/en.json';

const i18n = createInstance();
await i18n.use(LanguageDetector).init({
  resources: { 'zh-CN': { translation: zh }, en: { translation: en } },
  supportedLngs: ['zh-CN', 'en'],
  fallbackLng: 'en',
  load: 'currentOnly',
  interpolation: { escapeValue: false },
  detection: {
    order: ['path', 'querystring', 'localStorage', 'navigator', 'htmlTag'],
    lookupFromPathIndex: import.meta.env.BASE_URL.split('/').filter(Boolean).length,
    lookupQuerystring: 'lang',
    lookupLocalStorage: 'cjr.language',
    caches: ['localStorage'],
    convertDetectedLanguage: language => /^zh(?:-|$)/i.test(language) ? 'zh-CN' : language,
  },
});
export default i18n;
