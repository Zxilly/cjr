import { useTranslation } from 'react-i18next';
import { repository } from '../config';
import { type Language, languages } from '../i18n/resources';

export function Navigation({
  base,
  language,
}: {
  base: string;
  language: Language;
}) {
  const { t } = useTranslation();
  return (
    <nav className="topbar" aria-label={t('navigation')}>
      <a href={repository} className="repo-link">
        Zxilly / cjr <span aria-hidden="true">↗</span>
      </a>
      <nav className="languages" aria-label={t('language')}>
        {languages.map((target, index) => (
          <span className="language-option" key={target}>
            {index > 0 ? <span aria-hidden="true">/</span> : null}
            <a
              href={`${base}${target}/`}
              lang={target}
              hrefLang={target}
              data-language={target}
              aria-current={target === language ? 'true' : undefined}
            >
              {target === 'en' ? 'English' : '中文'}
            </a>
          </span>
        ))}
      </nav>
    </nav>
  );
}
