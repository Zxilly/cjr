import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { act, StrictMode } from 'react';
import { hydrateRoot } from 'react-dom/client';
import { renderToString } from 'react-dom/server';
import { I18nextProvider } from 'react-i18next';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { installCommand } from './config';
import { createSiteI18n } from './i18n';
import type { Language } from './i18n/resources';

afterEach(cleanup);

async function application(language: Language = 'en', base = '/') {
  const i18n = await createSiteI18n(language, '0.0.1');
  return (
    <StrictMode>
      <I18nextProvider i18n={i18n}>
        <App base={base} version="0.0.1" language={language} />
      </I18nextProvider>
    </StrictMode>
  );
}

describe('installation flow', () => {
  it('switches methods with arrow keys and copies the versioned install command', async () => {
    const user = userEvent.setup();
    render(await application());
    const download = screen.getByRole('tab', { name: 'Download' });
    await user.click(download);
    await user.keyboard('{ArrowRight}');
    const source = screen.getByRole('tab', { name: 'Build from source' });
    expect(source.getAttribute('aria-selected')).toBe('true');
    expect(document.activeElement).toBe(source);
    expect(screen.getByRole('tabpanel').textContent).toContain(
      installCommand('0.0.1'),
    );
    await user.click(
      screen.getByRole('button', { name: 'Copy installation command' }),
    );
    expect(await navigator.clipboard.readText()).toBe(installCommand('0.0.1'));
    expect(screen.getByRole('status').textContent).toBe('Command copied');
    await user.click(source);
    await user.keyboard('{Home}');
    expect(download.getAttribute('aria-selected')).toBe('true');
    expect(
      screen.getByRole('link', { name: 'Linux x64' }).getAttribute('href'),
    ).toContain('x86_64-unknown-linux-musl.tar.gz');
  });

  it('offers manual copying when clipboard access fails', async () => {
    const user = userEvent.setup();
    vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValue(
      new DOMException('Permission denied'),
    );
    render(await application('zh-CN'));
    await user.click(screen.getByRole('tab', { name: '编译安装' }));
    await user.click(screen.getByRole('button', { name: '复制安装命令' }));
    expect(screen.getByRole('status').textContent).toBe(
      '请选中命令后手动复制。',
    );
    expect(screen.getByRole('tabpanel').textContent).toContain(
      installCommand('0.0.1'),
    );
  });
});

it.each(['en', 'zh-CN'] as const)(
  'hydrates the %s page and preserves locale and subpath links',
  async (language) => {
    const app = await application(language, '/cjr/');
    const container = document.createElement('div');
    container.innerHTML = renderToString(app);
    document.body.append(container);
    const errors = vi.fn();
    let root: ReturnType<typeof hydrateRoot> | undefined;
    try {
      await act(async () => {
        root = hydrateRoot(container, app, { onRecoverableError: errors });
      });
      expect(errors).not.toHaveBeenCalled();
      expect(
        container.querySelector('[data-language="en"]')?.getAttribute('href'),
      ).toBe('/cjr/en/');
      expect(
        container
          .querySelector('[data-book="user-guide"]')
          ?.getAttribute('href'),
      ).toBe(`/cjr/book/user-guide/${language}/installation.html`);
      const user = userEvent.setup();
      await user.click(
        screen.getByRole('tab', {
          name: language === 'en' ? 'Build from source' : '编译安装',
        }),
      );
      await waitFor(() =>
        expect(screen.getByRole('tabpanel').textContent).toContain(
          installCommand('0.0.1'),
        ),
      );
    } finally {
      await act(async () => root?.unmount());
      container.remove();
    }
  },
);
