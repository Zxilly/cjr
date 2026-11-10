# 文档与网站

网站沿用 cjv 的组织方式：`web/` 是 landing 页，`docs/user-guide/` 和 `docs/dev-guide/` 分别维护用户手册与开发指南。每本书的 `src/` 为中文，`en/` 为英文。

landing 页使用 React、TypeScript 和 Vite。安装方式切换由 Radix Tabs 处理，翻译通过 react-i18next 提供。构建时由 React 预渲染双语 HTML，浏览器通过 hydration 接管交互。字体随站点打包，文档由 mdBook 生成。

## 本地开发

需要 Node.js 22.22.2+、24.15+ 或 26+，以及 npm、mdBook 0.5.4 和 mdbook-sitemap 0.1.1。建议使用当前 Node.js LTS。

```sh
cargo install mdbook-sitemap --version 0.1.1 --locked
cd web
npm ci
npm run dev
```

前端源码按职责组织：

| 路径 | 职责 |
| --- | --- |
| `src/App.tsx` | 页面组合 |
| `src/components/` | 导航、安装方式、下载与文档入口、命令复制 |
| `src/config.ts` | 发行资产和安装命令 |
| `src/i18n/` | 翻译类型、语言检测和实例初始化 |
| `src/locales/` | 中英文翻译资源 |
| `scripts/` | 版本读取、React 预渲染、mdBook 构建与导航 |

`npm run typecheck` 执行严格 TypeScript 检查；`npm run lint` 和 `npm run format` 分别检查和整理代码。`npm run test:unit` 运行 Vitest 与 Testing Library 交互测试。

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

`mdbook-sitemap` 后端根据章节结构生成每本书的 sitemap。构建脚本将 `html/` 复制到既有公开目录，再合并插件产物与中英文首页；`scripts/seo.ts` 继续处理页面摘要和 canonical。插件关闭 `include-lastmod`，避免把构建日期误当作内容修改日期。

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

默认部署在域名根路径。在子路径部署时，例如 GitHub Pages 的 `/cjr/`，设置 `SITE_BASE=/cjr/` 后构建。首页链接和静态资源将使用同一前缀。

部署时上传整个 `dist/`。`node_modules/`、`dist/`、`.edgeone/` 和 mdBook 的 `book/` 输出保留在本地忽略目录中。

修改文档时同步更新两种语言和各自的 `SUMMARY.md`。安装说明和下载按钮应指向已发布的安装脚本或二进制。

## 语言与版本

首页文案位于 `web/src/locales/zh-CN.json` 和 `en.json`，由 i18next 处理。显式语言 URL 优先，其次是旧链接的 `?lang=`、已保存的选择和浏览器语言。其他语言回退到英文。

构建时使用与浏览器相同的 React 组件生成 `/zh-CN/` 和 `/en/` 的完整 HTML，下载和文档链接由静态 HTML 提供。每种语言独立初始化翻译实例，hydration 使用对应页面的语言。根路径按语言偏好跳转到相应语言页。页面标题、搜索摘要、无障碍标签和复制提示均使用翻译资源。下载版本从 `Cargo.toml` 读取，与二进制保持一致。

两种语言的文档使用相同章节文件名。构建过程为每章添加语言链接，切换时保留章节、移除各语言特有的标题锚点；首页链接保持所选语言。中文文档工具栏翻译位于 `web/src/locales/book-zh-CN.json`。

`npm test` 依次执行 Biome 检查、组件交互测试、TypeScript 检查、完整站点构建和产物测试。测试覆盖 tab 键盘操作、复制成功与失败反馈、双语 hydration、语言优先级、下载链接和章节切换。新增文案必须同时补齐两种语言，缺少翻译键会使构建失败。可以用 `SITE_BASE=/cjr/ npm test` 验证子路径部署。CI 和生产部署使用同一套命令。

## 生产部署

官网为 [cjr.zxilly.com](https://cjr.zxilly.com)，托管在 EdgeOne Makers。项目 `cjr-web` 使用腾讯云中国站账户，部署区域为全球（含中国大陆）。

`.github/workflows/website.yml` 在 `master` 的网站、文档、版本或部署工作流变更后自动构建、测试并上传完整站点，也支持手动触发。发布工作流在 Release 正式发布后调用它，更新网站的版本和下载链接。生产部署串行执行，每次上传完成后再开始下一次部署。

GitHub 仓库配置：

| 类型 | 名称 | 内容 |
| --- | --- | --- |
| Variable | `EDGEONE_PROJECT_NAME` | `cjr-web` |
| Secret | `EDGEONE_API_TOKEN` | 中国站 Makers API Token |

在 Makers 中绑定生产域名 `cjr.zxilly.com`，再按控制台提供的目标地址配置 Cloudflare CNAME，使用 **DNS only**。证书由 Makers 管理。DNS 和域名绑定配置一次后持续沿用。

手动部署已构建的站点：

```sh
npx --yes edgeone@1.6.47 login --site china --token "$EDGEONE_API_TOKEN"
npx --yes edgeone@1.6.47 makers deploy ./dist \
  --name cjr-web --area global --env production --skip-ai-gateway-sync
```

部署记录见 [GitHub Actions](https://github.com/Zxilly/cjr/actions/workflows/website.yml)。需要回退时，在 Makers 的部署列表中选择先前成功的生产部署。
