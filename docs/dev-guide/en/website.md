# Documentation and website

The site follows cjv's structure: `web/` contains the landing page, while `docs/user-guide/` and `docs/dev-guide/` contain the two manuals. Each book uses `src/` for Chinese and `en/` for English.

The landing page uses React, TypeScript and Vite. Radix Tabs handles installation method switching, and react-i18next provides translations. React prerenders both languages during the build, and the browser hydrates those pages for interaction. Fonts are bundled locally. mdBook generates the manuals.

## Local development

Install Node.js 22.22.2+, 24.15+ or 26+, npm and mdBook 0.5.4. The current Node.js LTS is recommended.

```sh
cd web
npm ci
npm run dev
```

Frontend source layout:

| Path | Responsibility |
| --- | --- |
| `src/App.tsx` | Page composition |
| `src/components/` | Navigation, installation methods, downloads, documentation links and command copying |
| `src/config.ts` | Release assets and installation commands |
| `src/i18n/` | Translation types, language detection and instance initialization |
| `src/locales/` | Chinese and English translations |
| `scripts/` | Version loading, React prerendering, mdBook builds and navigation |

Use `npm run typecheck` for strict TypeScript checks, `npm run lint` for code checks and `npm run format` to apply formatting. `npm run test:unit` runs interaction tests with Vitest and Testing Library.

To preview the Chinese user guide separately:

```sh
mdbook serve docs/user-guide
```

## Build the full site

```sh
cd web
npm run build
npm run preview
```

The repository-root `dist/` directory contains the landing page and all four books:

```text
dist/
  index.html
  zh-CN/index.html
  en/index.html
  assets/
  book/user-guide/zh-CN/
  book/user-guide/en/
  book/dev-guide/zh-CN/
  book/dev-guide/en/
```

The default base path is `/`. For a subpath deployment such as GitHub Pages at `/cjr/`, set `SITE_BASE=/cjr/` when building. Landing-page links and assets share that prefix.

Upload the whole `dist/` directory. Keep `node_modules/`, `dist/`, `.edgeone/` and mdBook `book/` output in ignored local directories.

Update both languages and their `SUMMARY.md` files when editing documentation. Installation instructions and download buttons must point to published binaries or installer scripts.

## Languages and versions

Landing-page messages live in `web/src/locales/zh-CN.json` and `en.json`, managed by i18next. Explicit language URLs take precedence, followed by the legacy `?lang=` query, a saved preference and the browser language. Other languages fall back to English.

The build generates complete HTML at `/zh-CN/` and `/en/` using the same React components as the browser. Static HTML provides download and documentation links. Each language has an independent translation instance, and hydration uses the page's locale. The root path redirects to the preferred language page when needed. Titles, descriptions, accessible labels and copy feedback use translation resources. The download version comes from `Cargo.toml`, matching the binary.

Both documentation languages use matching chapter filenames. The build adds language links to each chapter. Switching keeps the chapter and drops heading anchors that differ between translations. Home links preserve the selected language. Chinese documentation toolbar translations live in `web/src/locales/book-zh-CN.json`.

`npm test` runs Biome checks, component interaction tests, TypeScript checks, the complete site build and output tests. Tests cover keyboard tab navigation, clipboard success and failure feedback, bilingual hydration, language precedence, downloads and chapter switching. Add messages to both languages; missing translation keys fail the build. Use `SITE_BASE=/cjr/ npm test` to check a subpath deployment. CI and production deployment use this same command.

## Production deployment

The website at [cjr.zxilly.com](https://cjr.zxilly.com) runs on EdgeOne Makers. The `cjr-web` project uses a Tencent Cloud China-site account with global coverage, including mainland China.

`.github/workflows/website.yml` builds, tests and uploads the complete site when website, documentation, version or deployment workflow files change on `master`. It also supports manual runs. The release workflow calls it after publishing a release to update the version and download links. Production deployments run serially; each upload completes before the next deployment starts.

GitHub repository settings:

| Type | Name | Value |
| --- | --- | --- |
| Variable | `EDGEONE_PROJECT_NAME` | `cjr-web` |
| Secret | `EDGEONE_API_TOKEN` | China-site Makers API token |

Bind `cjr.zxilly.com` to production in Makers, then add the CNAME target provided by the console to Cloudflare with **DNS only** enabled. Makers manages the certificate. Deployments reuse the existing DNS and domain binding settings.

To deploy an already built site manually:

```sh
npx --yes edgeone@1.6.47 login --site china --token "$EDGEONE_API_TOKEN"
npx --yes edgeone@1.6.47 makers deploy ./dist \
  --name cjr-web --area global --env production --skip-ai-gateway-sync
```

Deployment runs are listed in [GitHub Actions](https://github.com/Zxilly/cjr/actions/workflows/website.yml). To roll back, select an earlier successful production deployment in Makers.
