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
  assets/
  book/user-guide/zh-CN/
  book/user-guide/en/
  book/dev-guide/zh-CN/
  book/dev-guide/en/
```

The default base path is `/`. For a subpath deployment such as GitHub Pages at `/cjr/`, set `SITE_BASE=/cjr/` when building. Landing-page links and assets share that prefix.

Deploy the whole `dist/` directory to a static host. The build does not publish the site or configure a domain. Do not commit `node_modules/`, `dist/` or mdBook `book/` output.

Update both languages and their `SUMMARY.md` files when editing documentation. Installation instructions must use available methods; do not add download buttons for unpublished binaries or installer scripts.
