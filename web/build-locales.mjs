import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { load } from 'cheerio';
import { createInstance } from 'i18next';

export const languages = ['zh-CN', 'en'];
const resources = {};
for (const language of languages) {
  resources[language] = { translation: JSON.parse(await readFile(new URL(`./src/locales/${language}.json`, import.meta.url), 'utf8')) };
}
const keys = Object.keys(resources['zh-CN'].translation).sort();
if (JSON.stringify(keys) !== JSON.stringify(Object.keys(resources.en.translation).sort())) {
  throw new Error('Website translation keys differ between languages');
}

async function translator(language, version) {
  const instance = createInstance();
  await instance.init({ lng: language, resources, fallbackLng: false, interpolation: { escapeValue: false } });
  return key => {
    if (!instance.exists(key)) throw new Error(`Missing ${language} translation: ${key}`);
    return instance.t(key, { version });
  };
}

export async function buildLandingLocales(dist, base, version) {
  const template = await readFile(resolve(dist, 'index.html'), 'utf8');
  for (const language of languages) {
    const t = await translator(language, version);
    const $ = load(template, { scriptingEnabled: false });
    $('html').attr('lang', language);
    $('title').text(t('title'));
    $('meta[name="description"]').attr('content', t('description'));
    $('[data-text]').each((_, el) => $(el).text(t($(el).attr('data-text'))));
    $('[data-label]').each((_, el) => $(el).attr('aria-label', t($(el).attr('data-label'))));
    $('[data-language]').each((_, el) => {
      const target = $(el).attr('data-language');
      $(el).attr('href', `${base}${target}/`).removeAttr('aria-current');
      if (target === language) $(el).attr('aria-current', 'true');
      $('<link rel="alternate">').attr({ hreflang: target, href: `${base}${target}/` }).appendTo('head');
    });
    $('[data-book]').each((_, el) => $(el).attr('href', `${base}book/${$(el).attr('data-book')}/${language}/${$(el).attr('data-page')}`));
    $('[data-asset]').each((_, el) => $(el).attr('href', `https://github.com/Zxilly/cjr/releases/download/v${version}/cjr-v${version}-${$(el).attr('data-asset')}`));
    $('[data-release]').attr('href', `https://github.com/Zxilly/cjr/releases/tag/v${version}`);
    $('#install-command').text(`cargo install --git https://github.com/Zxilly/cjr --tag v${version} --locked`);
    await mkdir(resolve(dist, language), { recursive: true });
    await writeFile(resolve(dist, language, 'index.html'), $.html());
    if (language === 'zh-CN') await writeFile(resolve(dist, 'index.html'), $.html());
  }
}

const bookChinese = JSON.parse(await readFile(new URL('./src/locales/book-zh-CN.json', import.meta.url), 'utf8'));

export async function buildBookNavigation(dist, base, book, language) {
  const t = await translator(language);
  const directory = resolve(dist, 'book', book, language);
  for (const entry of await readdir(directory, { recursive: true })) {
    if (!entry.endsWith('.html')) continue;
    const chapter = entry.replaceAll('\\', '/');
    const file = resolve(directory, entry);
    const $ = load(await readFile(file, 'utf8'), { scriptingEnabled: false });
    if (!$('.right-buttons').length) continue;
    const nav = $('<nav class="book-languages">').attr('aria-label', t('bookLanguage'));
    $('<a>').attr('href', `${base}${language}/`).text(t('home')).appendTo(nav);
    for (const target of languages) {
      const href = `${base}book/${book}/${target}/${chapter}`;
      $('<a>').attr({ href, lang: target, hreflang: target, 'data-language': target })
        .attr('aria-current', target === language ? 'true' : null)
        .text(target === 'en' ? 'English' : '中文').appendTo(nav);
      $('<link rel="alternate">').attr({ hreflang: target, href }).appendTo('head');
    }
    $('.right-buttons').prepend(nav);
    $('<link rel="stylesheet">').attr('href', `${base}book-navigation.css`).appendTo('head');
    $('<script>').attr('src', `${base}book-navigation.js`).appendTo('body');
    if (language === 'zh-CN') {
      for (const attribute of ['aria-label', 'title', 'placeholder']) {
        $(`[${attribute}]`).each((_, el) => {
          const translated = bookChinese[$(el).attr(attribute)];
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
