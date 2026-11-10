import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { load } from 'cheerio';

const dist = fileURLToPath(new URL('../../dist/', import.meta.url));
const base = process.env.SITE_BASE || '/';
const origin = 'https://cjr.zxilly.com';

test('sitemap points only to canonical pages with localized metadata and analytics', async () => {
  const sitemap = load(await readFile(resolve(dist, 'sitemap.xml'), 'utf8'), { xml: true });
  const urls = sitemap('loc').toArray().map((element) => sitemap(element).text());
  assert.equal(urls.length, 22);
  assert.equal(new Set(urls).size, urls.length);
  assert.equal(sitemap('lastmod').length, 0, 'build dates are not page modification dates');
  const descriptions = new Set();
  for (const url of urls) {
    assert.ok(url.startsWith(origin + base));
    assert.doesNotMatch(url, /\/(index|print|toc|404)\.html$/);
    const path = url.slice((origin + base).length);
    const $ = load(await readFile(resolve(dist, path || 'index.html', path.endsWith('/') ? 'index.html' : ''), 'utf8'));
    assert.equal($('link[rel="canonical"]').attr('href'), url);
    assert.ok($('meta[name="description"]').attr('content')?.length > 20);
    descriptions.add($('meta[name="description"]').attr('content'));
    assert.equal($('meta[property="og:url"]').attr('content'), url);
    const tracker = $('script[src="https://trail.learningman.top/script.js"]');
    assert.equal(tracker.length, 1);
    assert.equal(tracker.attr('data-domains'), 'cjr.zxilly.com');
    assert.equal(tracker.attr('data-exclude-search'), 'true');
    for (const link of $('link[rel="alternate"]').toArray()) {
      assert.ok($(link).attr('href').startsWith(origin + base));
    }
  }
  assert.equal(descriptions.size, urls.length);
  assert.match(await readFile(resolve(dist, 'robots.txt'), 'utf8'), /Sitemap: https:\/\/cjr\.zxilly\.com\//);
});
