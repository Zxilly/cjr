# Documentation and website

The site follows cjv's structure: `web/` contains the landing page, while `docs/user-guide/` and `docs/dev-guide/` contain the two manuals. Each book uses `src/` for Chinese and `en/` for English.

Vite builds static HTML, CSS and JavaScript. Fonts are bundled locally. mdBook generates the manuals.

## Local development

Install Node.js 22.12+ or 24+, npm and mdBook 0.5.4.

```sh
cd web
npm ci
npm run dev
```

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

Deploy the whole `dist/` directory to a static host. The build does not publish the site or configure a domain. Do not commit `node_modules/`, `dist/` or mdBook `book/` output.

Update both languages and their `SUMMARY.md` files when editing documentation. Installation instructions must use available methods; do not add download buttons for unpublished binaries or installer scripts.

## Languages and versions

Landing-page messages live in `web/src/locales/zh-CN.json` and `en.json`, managed by i18next. Explicit language URLs take precedence, followed by the legacy `?lang=` query, a saved preference and the browser language. Unsupported languages fall back to English.

The build generates complete HTML at `/zh-CN/` and `/en/`. Download and documentation links work without JavaScript. Titles, descriptions, accessible labels and copy feedback use translation resources. The download version comes from `Cargo.toml`, matching the binary.

Both documentation languages use matching chapter filenames. The build adds language links to each chapter. Switching keeps the chapter and drops heading anchors that differ between translations. Home links preserve the selected language. Chinese documentation toolbar translations live in `web/src/locales/book-zh-CN.json`.

`npm test` builds the site and verifies language pages, downloads and chapter switching. Add messages to both languages; missing translation keys fail the build. Use `SITE_BASE=/cjr/ npm test` to check a subpath deployment.
