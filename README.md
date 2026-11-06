# cjr

为仓颉程序准备对应版本的运行库，然后启动程序。

```sh
cjr ./app --port 8080
cjr --offline ./app
cjr inspect ./app --json
cjr list
cjr cache clean --dry-run
cjr cache clean --all
```

仓颉程序通常需要随 SDK 提供的动态运行库，不同版本的运行库未必兼容。cjr 从二进制元数据读取完整 SDK 版本，通过 [cangjie-version-manifest](https://github.com/Zxilly/cangjie-version-manifest) 找到官方发行包。它优先复用 cjv 缓存；本地没有可用内容时才下载。

支持 Windows x64、Linux x64，以及 macOS ARM64/x64 的二进制解析和加载配置。Windows 和 Linux 已通过多版本运行测试；macOS 的本地验证目前限于 SDK 文件解析和解包，原生运行由 CI 验证。测试范围见 [验收记录](docs/implementation.md)。

## 构建

需要 Rust 1.89 或更新版本，以及 C 编译器和 CMake。Rustls 使用的 aws-lc 需要编译本地代码；Windows 使用 MSVC 工具链。

```sh
cargo build --release --locked
cargo install --path . --locked
```

`Cargo.lock` 固定完整依赖图。Rust 1.89 已在 Linux 上通过编译检查。

## 命令

| 命令 | 用途 |
| --- | --- |
| `cjr [--offline] [-v] <binary> [args...]` | 准备运行库并执行；`-v` 显示版本与缓存来源 |
| `cjr inspect <binary> [--json]` | 查看版本、架构、依赖和本地运行库候选 |
| `cjr list [--json]` | 查看 cjr/cjv 本地资源、大小、来源与使用状态 |
| `cjr cache clean [--all] [--dry-run]` | 清理 cjr 缓存 |

binary 后的参数全部交给应用，包括 `--help`、`--offline` 和 `--`。裸程序名按 PATH 查找；与辅助命令同名的程序可以写成 `cjr ./inspect` 或 `cjr -- inspect`。工作目录、标准输入输出和退出状态按原程序保留。

`inspect` 和 `list` 只读取本地文件。报告中的缓存记录表示文件曾被登记，实际运行前仍会校验内容。

## 缓存与 cjv

默认缓存目录是 `~/.cjr`，可通过 `CJR_HOME` 指定一个独立目录。cjr 按 cjv 的配置顺序读取 `CJV_HOME`、`CJV_CONFIG_FILE`、用户 settings 和 `CJV_FALLBACK_SETTINGS`，不需要调用 cjv 命令。

已准备好的运行库直接复用。需要准备时，依次尝试 cjv 官方归档、cjv 受管安装、cjr 归档和官方下载。归档按官方 SHA-256 校验；受管安装在持有 cjv 锁时复制，并检查版本与架构。受管安装的文件可能被用户修改，因此 receipt 将其标记为 `cjv-managed`，与通过归档摘要校验的 `archive-verified` 区分。

```text
~/.cjr/
  .cjr-store.json         # 专属缓存目录的所有权标记
  .cjr-store.lock         # 存储协调锁，不删除
  metadata/              # 官方索引
  downloads/<sha256>     # 已校验 SDK 归档
  runtimes/<bundle-id>/  # receipt.json 与 payload/
  staging/               # 未发布临时文件
  usage/                 # 使用租约
  locks/                 # 持久协调锁，不删除
```

下载使用 HTTPS。缺少官方摘要的发行包不会自动安装。提取时保留目标 runtime 目录、相对动态库链接、文件权限和顶层许可通知。

缓存准备、发布和清理共用一个存储锁，所以不同版本的首次准备也会串行。启动前登记使用记录，随后释放存储锁。

`cache clean` 删除下载归档、清单和废弃 staging，保留运行库。加上 `--all` 后，也会删除能确认未被使用的运行库；`--dry-run` 只预览。清理始终保留 cjv 管理的内容。

Windows 通过 Job Object 跟踪派生进程，Linux 使用 subreaper。macOS 无法证明全部后代进程已经退出时，会保留使用记录至同一主机下次启动。cjr 被强杀、目标启动后台进程或记录无法读取时，也可能暂时无法回收运行库。

## 限制

cjr 只处理同一官方 SDK 正确构建的程序及其依赖，按完整版本匹配运行库。它不修复混编产物，也不接受自编 SDK 或手动覆盖版本。

程序由系统加载器执行。cjr 保留二进制原路径，只为子进程设置加载环境；应用自己的动态库和 stdx 仍需由应用提供。全静态仓颉程序无需下载运行库，但仍可能依赖系统 C/C++ 库。

- Windows 应用目录的同名 DLL、Linux 旧式 RPATH 等可能抢先加载其他运行库。发现冲突时，cjr 会报错。
- macOS 支持 thin/fat slice、最低系统版本检查和 `@rpath`、`@loader_path`、`@executable_path`。受限或 hardened 程序会被拒绝；cjr 不改写签名或 entitlements。
- ELF 需要保留 section headers；尚未验证的 Mach-O 指针 fixups 会返回诊断。
- 静态预检无法覆盖任意运行时插件、延迟加载、SxS 或自定义加载器。系统加载器的错误仍输出到 stderr。

## 开发与测试

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
python scripts/sdk_acceptance.py --cjr target/release/cjr --versions 1.0.5 1.2.0
```

Windows 将最后一条的程序路径改为 `target/release/cjr.exe`。SDK 测试会下载并校验官方归档，编译包含集合、异常、线程和分配操作的程序，再检查执行结果、参数、退出码、离线缓存和实际加载路径。

CI 覆盖 Windows、Linux、macOS ARM64 和 Intel。前三者运行官方 SDK 测试；Intel macOS 暂时只有缺少摘要的 Nightly 研究资产，因此只编译和测试 cjr。

[架构设计](DESIGN.md) 说明版本选择、缓存和各平台加载行为。[工程约束](docs/engineering.md) 记录模块职责、依赖选择和维护要求。
