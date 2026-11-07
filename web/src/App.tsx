import { useTranslation } from 'react-i18next';
import { DocumentationLinks } from './components/DocumentationLinks';
import { Hero } from './components/Hero';
import { InstallMethods } from './components/InstallMethods';
import { Navigation } from './components/Navigation';
import { repository, type SiteConfig } from './config';
import type { Language } from './i18n/resources';

export function App({
  base,
  version,
  language,
}: SiteConfig & { language: Language }) {
  const { t } = useTranslation();
  return (
    <>
      <a className="skip" href="#main">
        {t('skip')}
      </a>
      <div className="page">
        <Navigation base={base} language={language} />
        <main id="main">
          <Hero />
          <InstallMethods base={base} version={version} language={language} />
          <DocumentationLinks base={base} language={language} />
        </main>
        <footer>
          <a href={repository}>GitHub</a>
          <a href={`${repository}/issues`}>{t('issues')}</a>
          <a href="https://cjv.zxilly.com">cjv</a>
          <a href="https://cangjie-lang.cn/">{t('cangjie')}</a>
        </footer>
      </div>
    </>
  );
}
