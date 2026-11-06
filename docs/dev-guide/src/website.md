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
  zh-CN/index.html
  en/index.html
  assets/
  book/user-guide/zh-CN/
  book/user-guide/en/
  book/dev-guide/zh-CN/
  book/dev-guide/en/
```

默认部署在域名根路径。在子路径部署时，例如 GitHub Pages 的 `/cjr/`，设置 `SITE_BASE=/cjr/` 后构建。首页链接和静态资源将使用同一前缀。

部署时上传整个 `dist/`。不要提交 `node_modules/`、`dist/`、`.edgeone/` 或 mdBook 的 `book/` 输出。

修改文档时同步更新两种语言和各自的 `SUMMARY.md`。安装说明应指向可用的安装方式，不为尚未发布的安装脚本或二进制提供下载按钮。

## 语言与版本

首页文案位于 `web/src/locales/zh-CN.json` 和 `en.json`，由 i18next 处理。显式语言 URL 优先，其次是旧链接的 `?lang=`、已保存的选择和浏览器语言。不支持的语言回退到英文。

构建时生成 `/zh-CN/` 和 `/en/` 的完整 HTML，下载和文档链接不依赖 JavaScript。页面标题、搜索摘要、无障碍标签和复制提示均使用翻译资源。下载版本从 `Cargo.toml` 读取，与二进制保持一致。

两种语言的文档使用相同章节文件名。构建过程为每章添加语言链接，切换时保留章节、移除无法跨语言对应的标题锚点；首页链接保持所选语言。中文文档工具栏翻译位于 `web/src/locales/book-zh-CN.json`。

`npm test` 构建站点并验证语言页面、下载链接和章节切换。新增文案必须同时补齐两种语言，缺少翻译键会使构建失败。可以用 `SITE_BASE=/cjr/ npm test` 验证子路径部署。

## 生产部署

官网为 [cjr.zxilly.com](https://cjr.zxilly.com)，托管在 EdgeOne Makers。项目 `cjr-web` 使用腾讯云中国站账户，部署区域为全球（含中国大陆）。

`.github/workflows/website.yml` 在 `master` 的网站、文档、版本或部署工作流变更后自动构建、测试并上传完整站点，也支持手动触发。发布工作流在 Release 正式发布后调用它，更新网站的版本和下载链接。生产部署串行执行，不中断正在上传的任务。

GitHub 仓库配置：

| 类型 | 名称 | 内容 |
| --- | --- | --- |
| Variable | `EDGEONE_PROJECT_NAME` | `cjr-web` |
| Secret | `EDGEONE_API_TOKEN` | 中国站 Makers API Token |

在 Makers 中绑定生产域名 `cjr.zxilly.com`，再按控制台提供的目标地址配置 Cloudflare CNAME，使用 **DNS only**。证书由 Makers 管理。DNS 和域名绑定只需配置一次，不由每次部署修改。

手动部署已构建的站点：

```sh
npx --yes edgeone@1.6.47 login --site china --token "$EDGEONE_API_TOKEN"
npx --yes edgeone@1.6.47 makers deploy ./dist \
  --name cjr-web --area global --env production --skip-ai-gateway-sync
```

部署记录见 [GitHub Actions](https://github.com/Zxilly/cjr/actions/workflows/website.yml)。需要回退时，在 Makers 的部署列表中选择先前成功的生产部署。
