import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

export function CopyCommand({ command }: { command: string }) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<'copied' | 'copyFailed' | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  async function copy() {
    clearTimeout(timer.current);
    try {
      await navigator.clipboard.writeText(command);
      setStatus('copied');
    } catch {
      setStatus('copyFailed');
    }
    timer.current = setTimeout(() => setStatus(null), 5000);
  }

  return (
    <>
      <div className="codeblock">
        <code id="install-command">{command}</code>
        <button
          type="button"
          className="copy"
          aria-label={t('copyInstall')}
          onClick={copy}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <rect x="8" y="8" width="12" height="12" rx="2" />
            <path d="M16 8V4H4v12h4" />
          </svg>
        </button>
      </div>
      <p className="copy-status" role="status">
        {status ? t(status) : ''}
      </p>
    </>
  );
}
