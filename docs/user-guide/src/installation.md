# 安装与运行

## 从源码安装

需要 Rust 1.89 或更新版本、C 编译器和 CMake。Windows 使用 MSVC 工具链。

```sh
cargo install --git https://github.com/Zxilly/cjr --locked
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

第一次运行可能需要下载完整 SDK 归档，cjr 只提取所需运行库。后续运行会复用缓存。cjr 不复制或改写目标程序；工作目录、标准输入输出和程序参数沿用当前调用。

## 离线使用

先在联网时运行一次目标程序，再使用：

```sh
cjr --offline ./app
```

离线模式禁止网络请求。已保存的运行库记录，或本地清单加匹配的 SDK 缓存，必须足以确定官方资产身份。缺少这些内容时会返回错误。

`--offline` 和 `-v` 必须放在程序路径之前。路径后面的参数全部交给目标程序。
