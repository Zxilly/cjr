# 实现与验收记录

2026-11-07 本地测试记录。Windows 和 WSL Linux 已运行官方 SDK 样本；macOS 本地测试覆盖解析与解包，原生运行结果另由 CI 记录。

## 已实现

- 默认执行、inspect/list 文本与 JSON、cache clean/all/dry-run、离线模式；OsString 参数边界。
- 有界 PE `.cjsdkv`、ELF `.cjmetadata` 与 RELA、Mach-O `__cjsdkversion`/fat slice 解析；依赖、搜索路径、最低系统和符号版本信息。
- 稳定/Nightly 官方清单、GitHub/GitCode 元数据来源、空摘要拒绝、SHA-256 校验和完整下载重试。
- cjv 配置、持久归档及完成下载适配；按 lifecycle → lookup 顺序获取平台锁，受管 SDK 自动快照。
- ZIP/TAR.GZ 选择性提取、重复路径/越界/特殊文件拒绝、相对 symlink、文件权限、内容树 receipt、原子发布、每次使用校验。
- Windows DLL / ELF RPATH 冲突检查、常见传递依赖检查、glibc 下限、macOS dyld 路径与受限执行拒绝。
- Windows 挂起创建 → Job 分配 → 恢复线程，Unix 进程组/信号监督，Linux subreaper，使用租约和保守清理。

缓存准备、清理和使用登记共用一个存储锁，不同版本的首次准备会串行。目标运行期间不持有该锁。

## 真实样本

已有 SDK 的来源与完整摘要见 [研究汇总](../research/README.md)。验收脚本：

- `research/implementation_matrix.py`：Windows/Linux 多版本产品执行与 inspect、仅 receipt 离线命中、清理、DLL 遮蔽。
- `research/implementation_edges.py`：cjv 仅受管安装快照、参数/退出码、实际加载库路径、活跃清理、信号、同 SDK 动态库。
- `research/implementation_paths.py`：损坏归档回退、并发发布、runtime 篡改拒绝与离线失败。
- `research/implementation_lock.py`：Go 进程按 cjv 的顺序/字节范围持锁时拒绝快照，释放后快照成功；验证 OS 锁协议，未覆盖完整 cjv 更新事务。
- `scripts/sdk_acceptance.py`：可在干净 CI 主机重复的官方下载、编译和运行流程，包含 macOS。

每次本地运行的 JSON 结果保存在被忽略的 `research/implementation-artifacts/` 中；摘要见 [acceptance-summary.json](acceptance-summary.json)。研究报告保留了早期失败案例；下表记录当前实现的测试结果。

测试使用 SDK 1.0.5、1.1.3 和 1.2.0。

| 验收项 | 结果 |
| --- | --- |
| Windows 产品多版本矩阵 | 23/23；包含三个版本的默认、动态标准库和 strip 执行 |
| Linux 产品多版本矩阵 | 34/34；包含三个版本的默认、动态标准库、static 和 strip 执行 |
| Windows 补充验收 | 9/9；受管安装快照、参数/退出码、真实模块路径、活跃清理、同 SDK 动态库 |
| Linux 补充验收 | 10/10；包括真实模块路径、父进程 SIGTERM 转发、RPATH 冲突拒绝 |
| 两平台缓存故障与并发 | 各 5/5；损坏 cjv 归档回退到 cjr 归档、并发准备、损坏 runtime 拒绝、清理和离线缺失 |
| 两平台 Go/Rust 锁协议 | 各 2/2；Go 持锁时约 2 秒超时回退，释放后快照成功 |
| Linux 全新缓存网络路径 | 官方 1.2.0 清单/SDK 下载、摘要校验、runtime 提取、真实 workload 启动成功 |
| macOS 解析器 | 9 个真实 SDK 产物正确识别；普通及 fat fixture 测试通过 |
| macOS SDK 解包 | Linux 上通过 1.0.5/1.2.0 两份官方归档摘要、选择性提取与 PCRE2 两级 symlink 验收 |

Windows/Linux 的 `cargo fmt`、严格 Clippy、单元/集成测试和 release 构建通过。仓颉 Windows SDK 的中文参数异常见下文。

## 已知限制与待验收

1. macOS 三个官方 SDK 产物的 9 个 cjpm/std-core/strip 样本，已由 Rust 解析器正确提取版本、架构和最低系统版本；fat slice 使用构造 fixture 测试。未在 Mac 原生执行。CI 中已加入 ARM64 原生 SDK 验收与 Intel 编译测试，结果见仓库的 CI 运行记录。
2. macOS 当前对 hardened/library-validation/restricted 程序保守拒绝注入，不解析 entitlement 以放宽权限。普通 CLI 路径已实现。无法证明派生进程全部退出时，租约保留至同一主机下一次启动。
3. Windows 本机 1.2.0 Cangjie `main(args)` 收到中文参数时抛出 `IllegalArgumentException: Invalid unicode scalar value`，直接启动和通过 cjr 的结果相同。cjr 的 Windows 原生 Rust 子进程测试验证中文、空格、引号、反斜杠和字面 `--` 正确透传；不修改用户程序来规避 SDK 的参数解码行为。
4. ELF 解析要求可用 section headers，SDK version 使用已验证的普通指针/RELATIVE relocation；未知布局返回诊断，不扫描任意字符串猜版本。Mach-O 尚未验证 metadata 指针中的 chained fixups。
5. 依赖预检覆盖静态声明的普通加载路径，不承诺延迟加载、运行时插件、SxS/custom loader 或所有发行版的自定义系统库搜索配置。未做跨 SDK 混编求解。
6. 最低 Rust 1.89 已在 Linux 通过 `cargo +1.89.0 check --all-targets --locked`；Windows 常规验证使用较新稳定工具链，最低版本的 Windows 编译尚未单独运行。锁定完整依赖图。

原生平台使用 GitHub 官方的 [runner 标签](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)。

## 回归测试

模块维护要求见 [engineering.md](engineering.md)。HTTP 测试通过 mockito 模拟 503、404、断流和摘要不符，检查重试次数与失败结果。Clippy 检查生产代码的显式 panic / unwrap / expect。

常规测试在 Windows 通过 27 项，在 Linux 通过 29 项。Linux 另通过两份 macOS 官方 SDK 的解包测试。

PTY 回归曾发现 SIGTTOU 导致的前台终端恢复失败。修复后会检查前台进程组、信号掩码和 subreaper 状态是否恢复；CI 的 Unix job 包含该测试。
