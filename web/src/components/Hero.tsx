import { useTranslation } from 'react-i18next';

export function Hero() {
  const { t } = useTranslation();
  return (
    <header className="hero">
      <h1>
        cjr
        <span className="cursor" aria-hidden="true">
          _
        </span>
      </h1>
      <p className="headline">{t('headline')}</p>
      <p className="intro">{t('intro')}</p>
      <div className="launch">
        <span aria-hidden="true">$</span>
        <code>cjr ./app</code>
      </div>
      <p className="platforms">
        Windows x64 <span>·</span> Linux x64 <span>·</span> macOS ARM64 / x64
      </p>
    </header>
  );
}
