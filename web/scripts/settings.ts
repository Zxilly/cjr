import { readFileSync } from 'node:fs';
import { parse } from 'smol-toml';
import type { SiteConfig } from '../src/config.ts';

const manifest = parse(
  readFileSync(new URL('../../Cargo.toml', import.meta.url), 'utf8'),
);
const pkg = manifest.package;
if (
  typeof pkg !== 'object' ||
  pkg === null ||
  !('version' in pkg) ||
  typeof pkg.version !== 'string'
) {
  throw new Error('Cargo.toml must contain package.version');
}
const base = process.env.SITE_BASE || '/';
if (!base.startsWith('/') || !base.endsWith('/') || base.includes('..')) {
  throw new Error('SITE_BASE must be an absolute URL path ending in /');
}
export const siteConfig: SiteConfig = { base, version: pkg.version };
