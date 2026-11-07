import { useTranslation } from 'react-i18next';
import type { Language } from '../i18n/resources';
import { BookLink } from './BookLink';

export function DocumentationLinks({
  base,
  language,
}: {
  base: string;
  language: Language;
}) {
  const { t } = useTranslation();
  return (
    <>
      <section className="reading" aria-label={t('docs')}>
        <BookLink
          className="guide"
          base={base}
          language={language}
          book="user-guide"
        >
          <span>
            <strong>{t('userGuide')}</strong>
            <span>{t('userGuideHint')}</span>
          </span>
          <span aria-hidden="true">↗</span>
        </BookLink>
        <BookLink
          className="guide"
          base={base}
          language={language}
          book="dev-guide"
        >
          <span>
            <strong>{t('devGuide')}</strong>
            <span>{t('devGuideHint')}</span>
          </span>
          <span aria-hidden="true">↗</span>
        </BookLink>
      </section>
      <p className="scope">
        <BookLink
          base={base}
          language={language}
          book="user-guide"
          page="platforms.html"
        >
          {t('scope')}
        </BookLink>
      </p>
    </>
  );
}
