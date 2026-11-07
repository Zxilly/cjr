import '@fontsource/patua-one/latin-400.css';
import '@fontsource-variable/jetbrains-mono';
import '@fontsource-variable/noto-sans-sc';
import './style.css';
import { StrictMode } from 'react';
import { createRoot, hydrateRoot } from 'react-dom/client';
import { I18nextProvider } from 'react-i18next';
import { App } from './App';
import { createSiteI18n } from './i18n';
import { detectLanguage } from './i18n/browser';
import { isLanguage } from './i18n/resources';

const base = import.meta.env.BASE_URL;
const language = await detectLanguage(base);
const root = document.getElementById('root');
if (!root) throw new Error('Missing application root');

// Explicit language routes keep server markup and hydration in the same locale.
const pathLanguage =
  window.location.pathname.slice(base.length).split('/')[0] ?? '';
if (!isLanguage(pathLanguage) && document.documentElement.lang !== language) {
  window.location.replace(
    `${base}${language}/${window.location.search}${window.location.hash}`,
  );
} else {
  const i18n = await createSiteI18n(language, __CJR_VERSION__);
  document.documentElement.lang = language;
  document.title = i18n.t('title');
  document
    .querySelector('meta[name="description"]')
    ?.setAttribute('content', i18n.t('description'));
  const app = (
    <StrictMode>
      <I18nextProvider i18n={i18n}>
        <App base={base} version={__CJR_VERSION__} language={language} />
      </I18nextProvider>
    </StrictMode>
  );
  if (root.hasChildNodes()) hydrateRoot(root, app);
  else createRoot(root).render(app);
}
