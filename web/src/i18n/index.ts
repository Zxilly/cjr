import { createInstance } from 'i18next';
import { initReactI18next } from 'react-i18next';
import { type Language, resources } from './resources';

export async function createSiteI18n(language: Language, version: string) {
  const instance = createInstance();
  await instance.use(initReactI18next).init({
    lng: language,
    resources,
    fallbackLng: false,
    interpolation: { escapeValue: false, defaultVariables: { version } },
    react: { useSuspense: false },
  });
  return instance;
}
