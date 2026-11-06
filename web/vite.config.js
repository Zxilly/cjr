import { defineConfig } from 'vite';

export default defineConfig({
  base: process.env.SITE_BASE || '/',
  build: { outDir: '../dist', emptyOutDir: true },
});
