import { readdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { load } from 'cheerio';

const origin = 'https://cjr.zxilly.com';
const summaries: Record<string, [string, string]> = {
  'user-guide/introduction': [
    '了解 cjr 如何识别仓颉程序的 SDK 版本、准备匹配的运行库并启动程序，以及它与 cjv 的协作方式。',
    'Learn how cjr identifies a Cangjie program’s SDK version, prepares a matching runtime, and launches the program with optional cjv integration.',
  ],
  'user-guide/installation': [
    '在 Windows、Linux 或 macOS 上下载和安装 cjr，将可执行文件加入 PATH，并运行第一个仓颉程序。',
    'Download and install cjr on Windows, Linux, or macOS, add it to PATH, and run your first Cangjie program.',
  ],
  'user-guide/commands': [
    '查阅 cjr 运行程序、检查依赖、列出运行库和清理缓存的命令，以及离线模式和详细日志参数。',
    'Reference cjr commands for running programs, inspecting dependencies, listing runtimes, and cleaning caches, including offline and verbose options.',
  ],
  'user-guide/cache': [
    '配置 CJR_HOME 缓存位置，了解 cjr 如何复用 cjv 下载和已安装工具链，以及运行库缓存的清理规则。',
    'Configure CJR_HOME, understand how cjr reuses cjv downloads and managed toolchains, and manage cached runtimes safely.',
  ],
  'user-guide/platforms': [
    '查看 cjr 支持的平台和架构、各系统的运行库搜索方式，并排查版本不匹配、动态库缺失和离线运行问题。',
    'Check cjr platform and architecture support, runtime search behavior, and troubleshooting for SDK versions, missing libraries, and offline execution.',
  ],
  'dev-guide/introduction': [
    '了解 cjr 的 Rust 项目结构、开发边界和贡献约定，找到构建测试、架构、网站维护与发布指南。',
    'Explore the cjr Rust project, development boundaries, and contribution conventions, with links to building, architecture, website maintenance, and releases.',
  ],
  'dev-guide/building': [
    '准备 Rust、C 编译器和 CMake，构建 cjr，并运行格式检查、Clippy、单元测试与平台相关验证。',
    'Set up Rust, a C compiler, and CMake to build cjr, then run formatting, Clippy, unit tests, and platform checks.',
  ],
  'dev-guide/architecture': [
    '理解 cjr 的二进制检查、官方 SDK 匹配、cjv 兼容、下载校验、运行库缓存与子进程生命周期模块。',
    'Understand cjr modules for binary inspection, official SDK matching, cjv compatibility, verified downloads, runtime storage, and child-process lifecycle.',
  ],
  'dev-guide/website': [
    '维护 cjr 的中英文 React 首页和 mdBook 文档，运行本地构建与检查，并部署静态站点到 EdgeOne。',
    'Maintain the bilingual React landing pages and mdBook manuals, run local website checks, and deploy the static cjr site to EdgeOne.',
  ],
  'dev-guide/releases': [
    '按 cjr 版本发布流程更新清单与文档，创建稳定版本标签，构建各平台归档，并验证 GitHub 发布产物。',
    'Follow the cjr release workflow to update manifests and documentation, tag a stable version, build platform archives, and verify GitHub release assets.',
  ],
};

export async function buildSiteSeo(dist: string, base: string) {
  const urls: string[] = [];
  for (const entry of await readdir(dist, { recursive: true })) {
    if (!entry.endsWith('.html')) continue;
    const path = entry.replaceAll('\\', '/');
    const file = resolve(dist, entry);
    const $ = load(await readFile(file, 'utf8'), { scriptingEnabled: false });
    if (/\/(404|print|toc)\.html$/.test(path)) {
      $('<meta name="robots" content="noindex,follow">').appendTo('head');
    } else {
      let canonicalPath = path.replace(/index\.html$/, '');
      // mdBook duplicates its first chapter at index.html; retain the named chapter.
      if (/^book\/.+\/(en|zh-CN)\/$/.test(canonicalPath)) {
        canonicalPath += 'introduction.html';
      }
      // The unprefixed landing page is the Chinese canonical.
      if (canonicalPath === 'zh-CN/') canonicalPath = '';
      const canonical = `${origin}${base}${canonicalPath}`;
      $('link[rel="canonical"]').remove();
      $('<link rel="canonical">').attr('href', canonical).appendTo('head');
      urls.push(canonical);
      const match = canonicalPath.match(
        /^book\/([^/]+)\/(en|zh-CN)\/([^/]+)\.html$/,
      );
      if (match) {
        const summary = summaries[`${match[1]}/${match[3]}`];
        if (!summary)
          throw new Error(`Missing SEO summary for ${canonicalPath}`);
        $('meta[name="description"]').remove();
        $('<meta name="description">')
          .attr('content', summary[match[2] === 'en' ? 1 : 0])
          .appendTo('head');
      }
      $('link[rel="alternate"][hreflang]').each((_, element) => {
        const href = $(element).attr('href');
        if (!href) return;
        let absolute = new URL(href, origin).href;
        if (absolute === `${origin}${base}zh-CN/`)
          absolute = `${origin}${base}`;
        absolute = absolute.replace(
          /(\/book\/[^/]+\/(?:en|zh-CN)\/)index\.html$/,
          '$1introduction.html',
        );
        $(element).attr('href', absolute);
      });
      for (const [property, content] of Object.entries({
        'og:type': 'website',
        'og:title': $('title').text(),
        'og:description': $('meta[name="description"]').attr('content') ?? '',
        'og:url': canonical,
        'og:locale': $('html').attr('lang') === 'en' ? 'en_US' : 'zh_CN',
      })) {
        $(`meta[property="${property}"]`).remove();
        $('<meta>').attr({ property, content }).appendTo('head');
      }
      $('<meta name="twitter:card" content="summary">').appendTo('head');
      $('<script>')
        .attr({
          defer: '',
          src: 'https://trail.learningman.top/script.js',
          'data-website-id': '39ab5a3f-d5ed-4ff4-b1aa-214767a220e7',
          'data-domains': 'cjr.zxilly.com',
          'data-exclude-search': 'true',
          'data-exclude-hash': 'true',
        })
        .appendTo('head');
    }
    await writeFile(file, $.html());
  }
  await writeFile(
    resolve(dist, 'sitemap.xml'),
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${[
      ...new Set(urls),
    ]
      .sort()
      .map((url) => `  <url><loc>${url}</loc></url>`)
      .join('\n')}\n</urlset>\n`,
  );
  await writeFile(
    resolve(dist, 'robots.txt'),
    `User-agent: *\nAllow: /\n\nSitemap: ${origin}${base}sitemap.xml\n`,
  );
}
