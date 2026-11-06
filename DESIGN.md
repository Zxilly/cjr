# cjr 架构

cjr 根据仓颉程序中的 SDK 版本，找到对应的官方运行库，并通过系统加载器启动程序。SDK 版本是发行包的选择依据，不是通用 ABI 指纹。

输入程序及其仓颉依赖应由同一官方 SDK 正确构建。版本未知、官方资产缺失或加载路径存在冲突时，cjr 返回错误。它不尝试相邻版本，也不提供 runtime 覆盖或 SDK 导入命令。

## 执行流程

1. 解析程序路径，读取 PE、ELF 或 Mach-O 的仓颉元数据。
2. 检查宿主架构及动态依赖。全静态仓颉程序可以直接启动。
3. 按完整 SDK 版本和平台查找官方资产。已有 receipt 可以提供离线资产身份。
4. 复用已校验的 cjr 运行库；否则从 cjv、本地归档或官方下载准备快照。
5. 按最终运行库路径检查依赖和加载冲突，生成启动计划。
6. 登记使用记录、释放存储锁，再启动并等待目标程序。

一个 Cargo package 同时提供 library 和 CLI。`app` 编排流程；`inspect` 解析格式；`catalog` 选择资产；`cjv` 适配缓存协议；`provision` 准备运行库；`store` 管理发布与清理；`loader` 检查加载路径；`process` 监督子进程。必要的原生调用集中在私有 `platform` 模块。

## 版本识别

| 格式 | 版本位置 | 处理方式 |
| --- | --- | --- |
| PE | `.cjsdkv` | 将版本表地址映射到文件偏移，读取版本字符串 |
| ELF | `.cjmetadata` | 读取版本指针表，处理 RELATIVE relocation 和对齐零条目 |
| Mach-O | `__CJ_METADATA,__cjsdkversion` | 先选择宿主原生 slice，再读取版本表 |

解析器检查指针、长度和文件边界。重复的同版记录可以合并；缺失或未知布局返回诊断，不用全文件字符串搜索猜测版本。ELF 当前需要 section headers；Mach-O 版本指针的 chained fixups 尚未验证。

依赖图用于查找缺库、架构错误和搜索路径冲突。符号信息可用于拒绝明显缺失的依赖，但不能证明跨版本 ABI 兼容。运行时通过 `dlopen` 或 `LoadLibrary` 加载的任意插件无法静态穷举。

## 官方清单

清单来自 `Zxilly/cangjie-version-manifest`，支持 GitHub 和 GitCode 镜像。

- `versions.json` 按 channel、完整版本、平台索引资产。
- `nightly.json` 是独立的 channel 文档，稳定清单未命中时再查询。
- 每个资产提供文件名、URL 和 SHA-256；空摘要不允许自动安装。
- `latest` 不参与运行时选择。相同版本若对应多个不同摘要，返回歧义错误。

`--offline` 禁止网络访问。本地清单或 receipt 能确认资产身份，且存在匹配的运行库、归档或 cjv 安装时，即可运行。

| 目标 | 清单键 | runtime 目录 |
| --- | --- | --- |
| Windows x64 | `win32-x64` | `windows_x86_64_cjnative` / `windows_x86_64_llvm` |
| Linux x64 | `linux-x64` | `linux_x86_64_cjnative` / `linux_x86_64_llvm` |
| macOS ARM64 | `darwin-arm64` | `darwin_aarch64_cjnative` / `darwin_aarch64_llvm` |
| macOS x64 | `darwin-x64` | `darwin_x86_64_cjnative` / `darwin_x86_64_llvm` |

交叉编译 SDK 不能作为其宿主系统的运行包。Linux ARM64 尚未实现。

## cjv 适配

已有 cjr 快照直接复用。准备新快照时，来源顺序为 cjv 官方归档、cjv 官方受管安装、cjr 归档、官方下载。

cjv 的持久归档键为 `SHA256(kind + NUL + url + NUL + lowercase(sha256))`，归档的 `kind` 是 `artifact`。cjr 读取 `cache/<key>.json`、`cache/<key>.data`，也识别 `downloads/<sha256>` 中的完整归档。文件复制到私有 staging 后再校验官方摘要；读取失败或摘要不符时继续尝试其他来源。

受管安装必须有匹配官方资产的 release、tuple 和 sha256 记录。cjr 按顺序获取 `.lifecycle.lock` 和 `.lifecycle.lookup.lock`，复查安装状态并复制 runtime。Unix 使用 flock；Windows 使用首字节 LockFileEx，与 cjv 的锁协议保持一致。旧布局、外部链接和未完成事务会被跳过。

安装记录只能说明来源，不能证明文件此后没有被修改。因此从安装目录复制的快照标为 `cjv-managed`；通过官方归档摘要校验的快照标为 `archive-verified`。cjr 检查复制后的版本、架构和内容树，之后的运行不再依赖 cjv 目录。cjr 不恢复 cjv 事务，也不修改或清理其文件。

## 下载、提取与发布

下载使用 reqwest，完整操作的重试交给 backon。最多三次请求，指数退避并加入 jitter。连接、超时、断流、408、429 和 5xx 可重试；永久 HTTP 错误、摘要不符、资源超限和磁盘写入错误直接返回。当前没有断点续传。

ZIP 和 TAR.GZ 只提取目标 runtime 目录及顶层许可通知。提取保留文件权限和安全的相对链接，拒绝越界路径、Windows ADS/设备路径、重复条目、特殊文件和逃出 payload 的链接。单文件上限 1 GiB，总量 4 GiB，最多 100,000 个选中条目。

快照存放在 `runtimes/<bundle-id>/`，包含 `receipt.json` 和 `payload/`。bundle ID 由提取 recipe、官方归档摘要、平台和排序后的文件清单生成。清单记录内容摘要、权限及链接目标。

`StoreTransaction` 持有全局排他锁。元数据写入、准备、发布、使用登记和清理都通过该锁协调。发布只消费本存储 staging 中的普通目录，通过同卷 rename 使完整快照可见。cjr 不就地更新已发布运行库；使用前重新检查内容。不同资产的首次准备也会串行。

缓存是当前用户管理的本地目录。原子发布不保证断电后所有数据均已持久化，也不防止同一 UID 的恶意并发篡改。网络文件系统的锁语义未纳入支持范围。

## 使用记录与清理

启动前写入使用记录，其中包含主机与启动会话标识。未进入 spawn 就放弃执行时自动撤销记录；spawn 失败也会撤销。目标启动后，只有能证明派生进程全部退出才移除记录。

Windows 使用不带 KILL_ON_JOB_CLOSE 的 Job Object。Linux 开启 subreaper，并在检查后代进程时使用 NOWAIT，避免取走其他组件的子进程退出状态。macOS 无同等退出证明时保留记录。cjr 被强杀、后台派生进程仍存活、跨主机记录或状态未知时，清理会跳过对应运行库；同一主机重启后可回收旧会话记录。

`cache clean` 默认删除归档、清单和废弃 staging。`--all` 也清理能确认未使用的运行库，`--dry-run` 只预览。清理不删除协调锁，也不跟随链接删除外部目录。跳过使用中对象是正常结果，删除失败则返回非零状态。

## 平台加载

目标路径、参数、工作目录和标准流保持原样。参数使用 `OsString`，不拼接 shell 命令。环境修改只作用于子进程。启动前重新检查已记录的程序和依赖摘要。

### Windows

在子进程 PATH 前添加运行库目录。应用目录的 DLL 优先于 PATH；同名关键库若与所选运行库不一致，返回 `LibraryShadowing`。

进程先挂起创建，加入 Job Object 后恢复。共享控制台的 Ctrl+C 由系统送达目标，父进程等待并保留原始退出状态。SxS、延迟加载和自定义 DLL 搜索不在完整预检范围内。

### Linux

通过 `LD_LIBRARY_PATH` 提供运行库。检查 `DT_NEEDED`、RPATH/RUNPATH、`$ORIGIN`、传递依赖和可确认的 glibc 版本要求。旧式 RPATH、显式依赖路径、`LD_PRELOAD`、`LD_AUDIT` 和安全执行模式可能改变搜索结果；发现冲突时拒绝启动。系统 glibc 由宿主提供。

目标在独立进程组中执行。signal-hook 转发发给 cjr 的终止信号，并通过 SIGCHLD 唤醒退出检查。前台终端在结束后恢复；调用 tcsetpgrp 时临时屏蔽当前线程的 SIGTTOU，再恢复原掩码。

### macOS

选择 thin/fat 文件中与宿主原生架构一致的 slice，读取最低系统版本和依赖。按所属 image 展开 `@loader_path`，按主程序展开 `@executable_path`，结合 `LC_RPATH` 处理 `@rpath`，通过 `DYLD_LIBRARY_PATH` 提供运行库。

普通命令行程序是支持目标。cjr 不管理 `.app`，不依赖 Rosetta，不改写 load commands、签名、SIP 或 entitlements。已识别的受限或 hardened 程序会被拒绝，其他 dyld 失败保留原始诊断。PCRE2 的两级相对链接在提取后保留。

macOS ARM64 的官方 SDK 原生测试在 CI 中执行。Intel 发行资产缺少官方摘要时，只进行 cjr 编译和格式测试。

## 命令与错误

默认入口为 `cjr <binary> [args...]`，另提供 `inspect`、`list` 和 `cache clean`。参数规则见 [README](README.md)。查询命令只读本地文件；JSON 报告带 `schema_version`，保留未知状态和无法读取来源的诊断。

业务层返回 Result。`Outcome` 区分退出码与 Unix 信号，只有 CLI 入口终止进程。内部监督失败时由 RAII 终止并等待直属子进程；派生进程状态不明时保留使用记录。清理失败输出诊断，不覆盖目标退出状态。

监督涉及进程级信号、控制台和 subreaper 状态，同一进程内只允许一个监督实例。signal-hook 注销回调不保证恢复原始 OS signal disposition，因此这不是供任意宿主无副作用嵌入的进程库。

## 测试

常规测试检查格式解析、恶意输入、命令边界、下载故障、缓存校验、清理和进程状态。Unix 另用 PTY 检查前台进程组及信号掩码恢复。

官方 SDK 测试编译并运行集合、异常、线程和分配负载，覆盖默认、动态标准库、strip、离线缓存及实际加载路径。较早的 ABI 研究见 [research](research/README.md)，当前实现的测试记录见 [docs/implementation.md](docs/implementation.md)。

## 参考

- [版本清单仓库与协议](https://github.com/Zxilly/cangjie-version-manifest/blob/master/README.md)
- [版本清单 schema](https://github.com/Zxilly/cangjie-version-manifest/blob/master/src/schema.ts)
- [稳定版清单](https://github.com/Zxilly/cangjie-version-manifest/blob/master/versions.json)
- [仓颉包兼容性检查](https://cj-docs.gitcode.com/zh/1.2.0/dev-guide/source_zh_cn/Appendix/cangjie_package_compatibility.html)
- [仓颉 1.1.0 发行说明](https://docs.cangjie-lang.cn/docs/1.1.0/)
- [运行时部署](https://docs.cangjie-lang.cn/docs/1.0.0/user_manual/source_zh_cn/deploy_and_run/runtime_deploy_cjnative.html)
- [Windows DLL 搜索顺序](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order)
- [Linux loader](https://www.man7.org/linux/man-pages/man8/ld.so.8.html)
- [Rust object](https://docs.rs/object/latest/object/)、[reqwest blocking](https://docs.rs/reqwest/latest/reqwest/blocking/index.html)、[clap](https://docs.rs/clap/latest/clap/)、[标准库文件锁](https://doc.rust-lang.org/std/fs/struct.File.html)
