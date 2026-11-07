import { createInstance } from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { isLanguage, type Language, languages } from './resources';

export async function detectLanguage(base: string): Promise<Language> {
  const detector = createInstance();
  await detector.use(LanguageDetector).init({
    resources: {},
    supportedLngs: languages,
    fallbackLng: 'en',
    load: 'currentOnly',
    detection: {
      order: ['path', 'querystring', 'localStorage', 'navigator', 'htmlTag'],
      lookupFromPathIndex: base.split('/').filter(Boolean).length,
      lookupQuerystring: 'lang',
      lookupLocalStorage: 'cjr.language',
      caches: ['localStorage'],
      convertDetectedLanguage: (language) =>
        /^zh(?:-|$)/i.test(language) ? 'zh-CN' : language,
    },
  });
  return isLanguage(detector.language) ? detector.language : 'en';
}
