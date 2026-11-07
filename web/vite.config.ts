import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import { siteConfig } from './scripts/settings.ts';

export default defineConfig({
  plugins: [react()],
  base: siteConfig.base,
  define: { __CJR_VERSION__: JSON.stringify(siteConfig.version) },
  build: { outDir: '../dist', emptyOutDir: true },
});
