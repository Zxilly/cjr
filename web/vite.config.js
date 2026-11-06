import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';
import { parse } from 'smol-toml';

const manifest = parse(readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf8'));

export default defineConfig({
  base: process.env.SITE_BASE || '/',
  define: { __CJR_VERSION__: JSON.stringify(manifest.package.version) },
  build: { outDir: '../dist', emptyOutDir: true },
});
