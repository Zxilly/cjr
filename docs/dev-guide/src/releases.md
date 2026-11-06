# 发布版本

版本号由 `Cargo.toml` 的 `package.version` 定义。Clap 在编译时将它嵌入程序，`cjr --version` 和 `cjr -V` 均输出该版本；运行时不依赖 Git 或仓库文件。

## 发布步骤

1. 更新 `Cargo.toml` 和 `Cargo.lock` 中的包版本，同步网站包版本、中英文安装示例。
2. 运行格式化、Clippy、测试及 `web/` 下的 `npm test`，提交并推送改动。
3. 创建与包版本一致的稳定标签，例如 `git tag -a v0.0.1 -m "cjr v0.0.1"`，再运行 `git push origin v0.0.1`。

`release.yml` 首先检查标签与包版本，再调用常规 CI。通过后创建草稿 Release，使用 `taiki-e/upload-rust-binary-action` 构建、打包并上传四个平台的程序及 SHA-256 文件：

- Windows x64：`x86_64-pc-windows-msvc`，ZIP。
- Linux x64：`x86_64-unknown-linux-musl`，tar.gz。
- macOS Apple Silicon：`aarch64-apple-darwin`，tar.gz。
- macOS Intel：`x86_64-apple-darwin`，tar.gz。

归档命名为 `cjr-v<版本>-<target>`，包含可执行文件及中英文 README。macOS 最低版本为 13.0。发布任务逐个运行二进制的 `--version`，全部成功后由 `softprops/action-gh-release` 将草稿公开。失败时草稿保持未发布，可在 Actions 中重跑失败任务。已公开版本不移动标签，后续修复使用新版本号。

Action 固定到完整提交 SHA，并由 Dependabot 跟踪更新。工作流使用 GitHub 提供的 `GITHUB_TOKEN`，只在 Release 创建、上传和公开任务中授予写权限。

## 静态链接

`.cargo/config.toml` 为 Windows MSVC 和 Linux musl 目标启用 `crt-static`。Windows 将 C/C++ 运行时链接进程序，仍依赖 Windows 系统 DLL；Linux 产物不得包含动态库依赖或 ELF 解释器。Rust 依赖与可打包的原生依赖静态链接。macOS 保留 `/usr/lib/`、`/System/Library/` 下的系统库，不尝试静态替代 libSystem 或系统框架。

发布工作流读取最终二进制的依赖表：拒绝 Windows 动态 CRT、Linux 动态依赖和 macOS 非系统库。此策略针对 cjr 本身；被启动的仓颉程序仍使用匹配的动态运行库。
