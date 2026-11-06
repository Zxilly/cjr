# 文档与网站

网站沿用 cjv 的组织方式：`web/` 是 landing 页，`docs/user-guide/` 和 `docs/dev-guide/` 分别维护用户手册与开发指南。每本书的 `src/` 为中文，`en/` 为英文。

landing 页使用 Vite 构建静态 HTML、CSS 和 JavaScript，字体随站点打包，不依赖外部字体服务。文档由 mdBook 生成。

## 本地开发

需要 Node.js 22.12+ 或 24+、npm 和 mdBook 0.5.4。

```sh
cd web
npm ci
npm run dev
```

单独预览中文手册：

```sh
mdbook serve docs/user-guide
```

## 构建完整站点

```sh
cd web
npm run build
npm run preview
```

构建输出位于根目录 `dist/`，包含首页和四本书：

```text
dist/
  index.html
  assets/
  book/user-guide/zh-CN/
  book/user-guide/en/
  book/dev-guide/zh-CN/
  book/dev-guide/en/
```

默认部署在域名根路径。在子路径部署时，例如 GitHub Pages 的 `/cjr/`，设置 `SITE_BASE=/cjr/` 后构建。首页链接和静态资源将使用同一前缀。

部署时将整个 `dist/` 交给静态站点服务。构建不会自行发布或配置域名。不要提交 `node_modules/`、`dist/` 或 mdBook 的 `book/` 输出。

修改文档时同步更新两种语言和各自的 `SUMMARY.md`。安装说明应指向可用的安装方式，不为尚未发布的安装脚本或二进制提供下载按钮。
