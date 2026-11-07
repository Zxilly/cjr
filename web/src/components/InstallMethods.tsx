import * as Tabs from '@radix-ui/react-tabs';
import { useTranslation } from 'react-i18next';
import {
  downloads,
  downloadUrl,
  installCommand,
  releaseUrl,
  type SiteConfig,
} from '../config';
import type { Language } from '../i18n/resources';
import { BookLink } from './BookLink';
import { CopyCommand } from './CopyCommand';

export function InstallMethods({
  base,
  version,
  language,
}: SiteConfig & { language: Language }) {
  const { t } = useTranslation();
  const guide = (
    <BookLink
      base={base}
      language={language}
      book="user-guide"
      page="installation.html"
      className="text-link"
    >
      {t('installDocs')}
    </BookLink>
  );
  return (
    <section className="quickstart" aria-label={t('quickstart')}>
      <Tabs.Root defaultValue="install">
        <Tabs.List className="tabs" aria-label={t('steps')}>
          <Tabs.Trigger value="install">{t('install')}</Tabs.Trigger>
          <Tabs.Trigger value="source">{t('sourceInstall')}</Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="install" className="panel">
          <h2>{t('installTitle')}</h2>
          <p>{t('requirements')}</p>
          <nav className="downloads" aria-label={t('downloads')}>
            {downloads.map(({ label, asset }) => (
              <a
                key={asset}
                data-asset={asset}
                href={downloadUrl(version, asset)}
              >
                {t(label)}
              </a>
            ))}
          </nav>
          <div className="panel-actions">
            <a className="text-link" href={releaseUrl(version)}>
              {t('releaseNotes')}
            </a>
            {guide}
          </div>
        </Tabs.Content>
        <Tabs.Content value="source" className="panel">
          <h2>{t('sourceTitle')}</h2>
          <p>{t('sourceRequirements')}</p>
          <CopyCommand command={installCommand(version)} />
          <p className="note">{t('sourcePath')}</p>
          {guide}
        </Tabs.Content>
      </Tabs.Root>
    </section>
  );
}
