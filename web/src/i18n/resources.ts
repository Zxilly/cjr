import en from '../locales/en.json';
import zh from '../locales/zh-CN.json';

export const languages = ['zh-CN', 'en'] as const;
export type Language = (typeof languages)[number];
export const resources = {
  'zh-CN': { translation: zh },
  en: { translation: en },
};

export function isLanguage(value: string): value is Language {
  return languages.some((language) => language === value);
}

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'translation';
    resources: { translation: typeof en };
    strictKeyChecks: true;
  }
}
