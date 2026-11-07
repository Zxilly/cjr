import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { load } from 'cheerio';
import { StrictMode } from 'react';
import { renderToString } from 'react-dom/server';
import { I18nextProvider } from 'react-i18next';
import { App } from '../src/App';
import type { SiteConfig } from '../src/config';
import { createSiteI18n } from '../src/i18n';
import { languages, resources } from '../src/i18n/resources';

export async function buildLandingLocales(dist: string, config: SiteConfig) {
  const keys = Object.keys(resources['zh-CN'].translation).sort();
  if (
    JSON.stringify(keys) !==
    JSON.stringify(Object.keys(resources.en.translation).sort())
  ) {
    throw new Error('Website translation keys differ between languages');
  }
  const template = await readFile(resolve(dist, 'index.html'), 'utf8');
  for (const language of languages) {
    const i18n = await createSiteI18n(language, config.version);
    const $ = load(template, { scriptingEnabled: false });
    $('html').attr('lang', language);
    $('title').text(i18n.t('title'));
    $('meta[name="description"]').attr('content', i18n.t('description'));
    $('#root').html(
      renderToString(
        <StrictMode>
          <I18nextProvider i18n={i18n}>
            <App {...config} language={language} />
          </I18nextProvider>
        </StrictMode>,
      ),
    );
    $('<noscript>')
      .append($('<p class="page note">').text(i18n.t('noScript')))
      .appendTo('body');
    for (const target of languages) {
      $('<link rel="alternate">')
        .attr({ hreflang: target, href: `${config.base}${target}/` })
        .appendTo('head');
    }
    await mkdir(resolve(dist, language), { recursive: true });
    await writeFile(resolve(dist, language, 'index.html'), $.html());
    if (language === 'zh-CN')
      await writeFile(resolve(dist, 'index.html'), $.html());
  }
}
