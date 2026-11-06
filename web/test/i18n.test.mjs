import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, access } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { load } from 'cheerio';
import { parse } from 'smol-toml';

const root = fileURLToPath(new URL('../../', import.meta.url));
const dist = resolve(root, 'dist');
const base = process.env.SITE_BASE || '/';
const version = parse(await readFile(resolve(root, 'Cargo.toml'), 'utf8')).package.version;

test('localized landing pages work before JavaScript runs', async () => {
  for (const language of ['en', 'zh-CN']) {
    const $ = load(await readFile(resolve(dist, language, 'index.html'), 'utf8'), { scriptingEnabled: false });
    assert.equal($('html').attr('lang'), language);
    assert.match($('h2').first().text(), new RegExp(version.replaceAll('.', '\\.')));
    assert.equal($('[data-language][aria-current="true"]').attr('data-language'), language);
    assert.equal($('[data-asset]').length, 4);
    for (const link of $('[data-asset]').toArray()) {
      assert.equal($(link).attr('href'), `https://github.com/Zxilly/cjr/releases/download/v${version}/cjr-v${version}-${$(link).attr('data-asset')}`);
    }
    assert.match($('noscript').text(), language === 'en' ? /Download/ : /下载/);
    for (const link of $('[data-language], [data-book]').toArray()) {
      const href = $(link).attr('href');
      assert.ok(href.startsWith(base));
      await access(resolve(dist, href.slice(base.length), href.endsWith('/') ? 'index.html' : ''));
    }
  }
});

test('manual language links keep the chapter and use the deployment prefix', async () => {
  for (const book of ['user-guide', 'dev-guide']) {
    const chapter = book === 'user-guide' ? 'installation.html' : 'architecture.html';
    for (const language of ['en', 'zh-CN']) {
      const $ = load(await readFile(resolve(dist, 'book', book, language, chapter), 'utf8'));
      const links = $('.book-languages [data-language]').toArray();
      assert.equal(links.length, 2);
      for (const link of links) {
        const target = $(link).attr('data-language');
        assert.equal($(link).attr('href'), `${base}book/${book}/${target}/${chapter}`);
        await access(resolve(dist, 'book', book, target, chapter));
      }
      assert.equal($('.book-languages a').first().attr('href'), `${base}${language}/`);
      if (language === 'zh-CN') assert.equal($('#mdbook-searchbar').attr('placeholder'), '搜索本书……');
    }
  }
});
