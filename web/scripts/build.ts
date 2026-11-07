import { spawnSync } from 'node:child_process';
import { copyFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import type { Language } from '../src/i18n/resources';
import { buildBookNavigation } from './books';
import { buildLandingLocales } from './prerender';
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
for (const [book, languages] of Object.entries(books)) {
  for (const [language, title] of Object.entries(languages)) {
    const result = spawnSync(
      'mdbook',
      [
        'build',
        resolve(root, 'docs', book),
        '--dest-dir',
        resolve(root, 'dist/book', book, language),
      ],
      {
        stdio: 'inherit',
        env: {
          ...process.env,
          MDBOOK_BOOK__SRC: language === 'en' ? 'en' : 'src',
          MDBOOK_BOOK__LANGUAGE: language,
          MDBOOK_BOOK__TITLE: title,
          MDBOOK_OUTPUT__HTML__SITE_URL: `${base}book/${book}/${language}/`,
        },
      },
    );
    if (result.error) throw result.error;
    if (result.status !== 0) process.exit(result.status ?? 1);
    await buildBookNavigation(
      resolve(root, 'dist'),
      base,
      book,
      language as Language,
    );
  }
}
