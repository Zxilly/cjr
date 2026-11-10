import { spawnSync } from 'node:child_process';
import { copyFile, cp, mkdir, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import type { Language } from '../src/i18n/resources';
import { buildBookNavigation } from './books';
import { buildLandingLocales } from './prerender';
import { buildSiteSeo } from './seo';
import { siteConfig } from './settings';

const root = fileURLToPath(new URL('../../', import.meta.url));
const web = fileURLToPath(new URL('../', import.meta.url));
const { base, version } = siteConfig;
await build({
  root: web,
  configFile: resolve(web, 'vite.config.ts'),
  logLevel: 'warn',
});
await buildLandingLocales(resolve(root, 'dist'), { base, version });

await mkdir(resolve(root, 'dist/licenses'), { recursive: true });
for (const font of [
  '@fontsource/patua-one',
  '@fontsource-variable/jetbrains-mono',
  '@fontsource-variable/noto-sans-sc',
]) {
  await copyFile(
    resolve(web, 'node_modules', font, 'LICENSE'),
    resolve(root, 'dist/licenses', `${font.split('/')[1]}.txt`),
  );
}

const books: Record<string, Record<Language, string>> = {
  'user-guide': { 'zh-CN': 'cjr 用户手册', en: 'cjr User Guide' },
  'dev-guide': { 'zh-CN': 'cjr 开发指南', en: 'cjr Developer Guide' },
};
const staging = await mkdtemp(resolve(tmpdir(), 'cjr-books-'));
if (dirname(staging) !== resolve(tmpdir()))
  throw new Error('Invalid staging path');
const sitemaps: string[] = [];
try {
  for (const [book, languages] of Object.entries(books)) {
    for (const [language, title] of Object.entries(languages)) {
      const result = spawnSync(
        'mdbook',
        [
          'build',
          resolve(root, 'docs', book),
          '--dest-dir',
          resolve(staging, book, language),
        ],
        {
          stdio: 'inherit',
          env: {
            ...process.env,
            MDBOOK_BOOK__SRC: language === 'en' ? 'en' : 'src',
            MDBOOK_BOOK__LANGUAGE: language,
            MDBOOK_BOOK__TITLE: title,
            MDBOOK_OUTPUT__HTML__SITE_URL: `${base}book/${book}/${language}/`,
            MDBOOK_OUTPUT__SITEMAP__BASE_URL: `https://cjr.zxilly.com${base}book/${book}/${language}/`,
          },
        },
      );
      if (result.error) throw result.error;
      if (result.status !== 0)
        throw new Error(`mdBook exited with ${result.status}`);
      const destination = resolve(root, 'dist/book', book, language);
      await cp(resolve(staging, book, language, 'html'), destination, {
        recursive: true,
      });
      const sitemap = resolve(destination, 'sitemap.xml');
      await copyFile(
        resolve(staging, book, language, 'sitemap/sitemap.xml'),
        sitemap,
      );
      sitemaps.push(sitemap);
      await buildBookNavigation(
        resolve(root, 'dist'),
        base,
        book,
        language as Language,
      );
    }
  }
} finally {
  await rm(staging, { recursive: true, force: true });
}
await buildSiteSeo(resolve(root, 'dist'), base, sitemaps);
