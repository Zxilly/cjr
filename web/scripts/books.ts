import { readdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { load } from 'cheerio';
import { createSiteI18n } from '../src/i18n';
import { type Language, languages } from '../src/i18n/resources';
import chinese from '../src/locales/book-zh-CN.json';

const bookChinese: Record<string, string> = chinese;

export async function buildBookNavigation(
  dist: string,
  base: string,
  book: string,
  language: Language,
) {
  const i18n = await createSiteI18n(language, '');
  const t = i18n.t;
  const directory = resolve(dist, 'book', book, language);
  for (const entry of await readdir(directory, { recursive: true })) {
    if (!entry.endsWith('.html')) continue;
    const chapter = entry.replaceAll('\\', '/');
    const file = resolve(directory, entry);
    const $ = load(await readFile(file, 'utf8'), { scriptingEnabled: false });
    if (!$('.right-buttons').length) continue;
    const nav = $('<nav class="book-languages">').attr(
      'aria-label',
      t('bookLanguage'),
    );
    $('<a>').attr('href', `${base}${language}/`).text(t('home')).appendTo(nav);
    for (const target of languages) {
      const href = `${base}book/${book}/${target}/${chapter}`;
      $('<a>')
        .attr({ href, lang: target, hreflang: target, 'data-language': target })
        .attr('aria-current', target === language ? 'true' : null)
        .text(target === 'en' ? 'English' : '中文')
        .appendTo(nav);
      $('<link rel="alternate">')
        .attr({ hreflang: target, href })
        .appendTo('head');
    }
    $('.right-buttons').prepend(nav);
    $('<link rel="stylesheet">')
      .attr('href', `${base}book-navigation.css`)
      .appendTo('head');
    $('<script>').attr('src', `${base}book-navigation.js`).appendTo('body');
    if (language === 'zh-CN') {
      for (const attribute of ['aria-label', 'title', 'placeholder']) {
        $(`[${attribute}]`).each((_, el) => {
          const translated = bookChinese[$(el).attr(attribute) ?? ''];
          if (translated) $(el).attr(attribute, translated);
        });
      }
      $('.theme, .mdbook-help-title, #mdbook-help-popup p').each((_, el) => {
        const translated = bookChinese[$(el).text().trim()];
        if (translated) $(el).text(translated);
      });
    }
    await writeFile(file, $.html());
  }
}
