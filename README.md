# cjr — Cangjie Runtime Manager

[![CI](https://img.shields.io/github/actions/workflow/status/Zxilly/cjr/ci.yml?branch=master&style=flat-square)](https://github.com/Zxilly/cjr/actions/workflows/ci.yml)

[English](README.EN.md) | 中文

cjr 自动准备所需运行库，用一条命令启动仓颉程序。

[官网](https://cjr.zxilly.com/zh-CN/) · [用户手册](https://cjr.zxilly.com/book/user-guide/zh-CN/)

```sh
cjr ./app --port 8080
```

## 安装

从 [GitHub Releases](https://github.com/Zxilly/cjr/releases/latest) 下载对应平台的归档，解压后将 `cjr` 所在目录加入 `PATH`。提供 Windows x64、Linux x64 和 macOS ARM64/x64 版本，以及各归档的 SHA-256 校验文件。

也可以从源码安装：

需要 Rust 1.89+、C 编译器和 CMake；Windows 使用 MSVC 工具链。

```sh
cargo install --git https://github.com/Zxilly/cjr --tag v0.0.1 --locked
```

安装步骤和 PATH 配置见[安装与运行](docs/user-guide/src/installation.md)。

## 常用命令

```sh
cjr --offline ./app
cjr inspect ./app --json
cjr list
cjr cache clean --dry-run
cjr cache clean --all
```

程序路径后的参数原样传给应用。`inspect` 和 `list` 只查询本地内容。清理只操作 cjr 自己的缓存，不修改 cjv 的文件；`--all` 也会跳过使用状态不明的运行库。

Windows x64、Linux x64 和 macOS ARM64/x64 的可用运行库以[官方发行清单](https://github.com/Zxilly/cangjie-version-manifest)为准。cjr 只处理同一官方 SDK 正确构建的程序，应用自己的库、stdx 和系统依赖仍需另行提供。详情见[平台与常见问题](docs/user-guide/src/platforms.md)。

## 文档

- [用户手册](docs/user-guide/src/introduction.md)：安装、命令、缓存及常见问题。
- [命令参考](docs/user-guide/src/commands.md)：参数边界、JSON 输出和清理行为。
- [开发指南](docs/dev-guide/src/introduction.md)：构建、模块职责与维护约定。
- [文档与网站](docs/dev-guide/src/website.md)：landing 页和中英文 mdBook 的本地开发与构建。

## 开发

```sh
cargo build --release --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

网站位于 `web/`，文档位于 `docs/`。问题反馈请提交到 [Issues](https://github.com/Zxilly/cjr/issues)。
