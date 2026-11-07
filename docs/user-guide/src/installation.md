# 安装与运行

## 下载预编译版本

从 [GitHub Releases](https://github.com/Zxilly/cjr/releases/latest) 下载与你的系统和架构对应的预编译归档。

| 系统 | v0.0.1 下载 |
| --- | --- |
| Windows x64 | [ZIP](https://github.com/Zxilly/cjr/releases/download/v0.0.1/cjr-v0.0.1-x86_64-pc-windows-msvc.zip) |
| Linux x64 | [tar.gz](https://github.com/Zxilly/cjr/releases/download/v0.0.1/cjr-v0.0.1-x86_64-unknown-linux-musl.tar.gz) |
| macOS Apple Silicon | [tar.gz](https://github.com/Zxilly/cjr/releases/download/v0.0.1/cjr-v0.0.1-aarch64-apple-darwin.tar.gz) |
| macOS Intel | [tar.gz](https://github.com/Zxilly/cjr/releases/download/v0.0.1/cjr-v0.0.1-x86_64-apple-darwin.tar.gz) |

Release 同时提供每个归档的 `.sha256` 文件。Linux 使用 `sha256sum -c <文件名>.sha256`，macOS 使用 `shasum -a 256 -c <文件名>.sha256` 校验。Windows 使用 `Get-FileHash <归档路径> -Algorithm SHA256`，将结果与校验文件中的摘要比较。

解压后，将 `cjr`（Windows 为 `cjr.exe`）所在目录加入 `PATH`。Linux / macOS 也可以将程序放入已有的用户命令目录：

```sh
mkdir -p ~/.local/bin
install -m 755 cjr ~/.local/bin/cjr
```

确认 `~/.local/bin` 已在 `PATH` 中。Windows 可将 `cjr.exe` 放到固定目录，再通过“环境变量”设置将该目录加入用户 `Path`。重新打开终端，运行：

```sh
cjr --version
# cjr 0.0.1
```

Windows 版本已包含 C/C++ 运行时。macOS 预编译版本要求 macOS 13 或更新版本。Linux 版本使用 musl 静态链接；目标仓颉程序及其运行库的系统要求仍需满足。

## 从源码安装

需要 Rust 1.89 或更新版本、C 编译器和 CMake。Windows 使用 MSVC 工具链。

```sh
cargo install --git https://github.com/Zxilly/cjr --tag v0.0.1 --locked
```

Cargo 将程序安装到其 `bin` 目录。确认该目录在 `PATH` 中，然后检查安装：

```sh
cjr --version
```

也可以在本地检出仓库后安装：

```sh
git clone https://github.com/Zxilly/cjr.git
cd cjr
cargo install --path . --locked
```

## 运行程序

Linux / macOS：

```sh
cjr ./app --port 8080
```

Windows PowerShell：

```powershell
cjr .\app.exe --port 8080
```

第一次运行可能需要下载完整 SDK 归档，cjr 只提取所需运行库。后续运行会复用缓存。cjr 直接启动原路径下的程序，保持程序文件原样；工作目录、标准输入输出和程序参数沿用当前调用。

## 离线使用

先在联网时运行一次目标程序，再使用：

```sh
cjr --offline ./app
```

离线模式仅使用本地内容。已保存的运行库记录，或本地清单加匹配的 SDK 缓存，必须足以确定官方资产身份。缺少这些内容时会返回错误。

`--offline` 和 `-v` 必须放在程序路径之前。路径后面的参数全部交给目标程序。
