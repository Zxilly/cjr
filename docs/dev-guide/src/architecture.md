# 架构

## 模块边界

| 模块 | 职责 |
| --- | --- |
| `app`、`cli` | 命令解析和执行编排 |
| `inspect` | PE、ELF、Mach-O 的版本与依赖解析 |
| `catalog` | 通过官方清单匹配完整 SDK 版本与平台 |
| `cjv` | 读取 cjv 配置、归档和受管安装 |
| `provision` | 下载、校验和选择性解包 |
| `store` | 不可变运行库快照、使用记录与清理 |
| `loader` | 宿主兼容性、加载路径和依赖检查 |
| `process` | 启动、信号、退出状态与子进程监督 |
| `platform` | 必要的原生调用及其安全封装 |

## 运行流程

读取程序元数据后，cjr 检查宿主架构和依赖。需要运行库时，根据完整 SDK 版本选择具有官方摘要的资产，复用本地内容或下载归档，提取并发布运行库快照。最终依赖检查通过后登记使用记录，释放存储锁，再启动程序。

SDK 版本来自 PE `.cjsdkv`、ELF `.cjmetadata` 或 Mach-O `__CJ_METADATA,__cjsdkversion`。它是发行包选择依据，不是通用 ABI 指纹。所有解析必须校验边界，未知布局返回错误。

## 缓存生命周期

`StoreTransaction` 持有存储排他锁。元数据、准备、发布、使用登记和清理通过同一把锁协调。不同版本的首次准备会串行，程序运行期间不持有锁。

运行库发布到 `runtimes/<bundle-id>/`，包含 `receipt.json` 和 `payload/`。内容清单记录文件摘要、权限和链接目标。发布通过同卷 rename 完成，不就地修改已发布运行库，每次使用前重新校验。

下载由 reqwest 执行，重试由 backon 管理。归档校验 SHA-256 后，只提取 runtime 目录和许可通知；拒绝越界路径、重复条目、特殊文件及逃出 payload 的链接。

cjv 受管安装读取按 `.lifecycle.lock`、`.lifecycle.lookup.lock` 的顺序加锁。使用归档的来源标为 `archive-verified`，受管安装快照标为 `cjv-managed`。安装记录不等于官方归档的逐字节校验。

## 加载和进程

环境修改只作用于子进程。参数使用 `OsString`，不拼接 shell 命令。Windows 检查应用目录 DLL 遮蔽，Linux 检查 RPATH/RUNPATH 和预加载冲突，macOS 展开 dyld 的路径占位符并拒绝已识别的受限程序。

Windows 使用 Job Object，Linux 使用 subreaper。Unix 转发终止信号并恢复前台终端。无法证明派生进程全部退出时保留运行库使用记录；同一主机重启后可以回收旧会话记录。

监督涉及进程级状态，同一进程内只允许一个监督实例。该 library 不保证任意宿主嵌入后信号 disposition 完全无变化。缓存面向当前用户的本地文件系统，不提供同一 UID 恶意篡改或网络文件系统锁语义的隔离保证。
