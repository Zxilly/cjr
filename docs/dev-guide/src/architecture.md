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

SDK 版本来自 PE `.cjsdkv`、ELF `.cjmetadata` 或 Mach-O `__CJ_METADATA,__cjsdkversion`。它标识具体的 SDK 发行版本，作为发行包的选择依据。所有解析必须校验边界，未知布局返回错误。

Mach-O 的 chained fixups 由 `object` 解析，`inspect::macho::fixups` 将 rebase 结果映射到版本表。支持 `DYLD_CHAINED_PTR_64`、`64_OFFSET`、`ARM64E`、`ARM64E_USERLAND` 和 `ARM64E_USERLAND24`，包括认证指针。解析以静态文件读取方式解码指针位字段；版本条目仅接受指向本文件字符串的 rebase 引用。链遍历有工作量上限，并检查文件与虚拟地址映射、页边界和重复条目。

## 缓存生命周期

`StoreTransaction` 持有存储排他锁。元数据、准备、发布、使用登记和清理通过同一把锁协调。各版本的首次准备串行执行，启动程序前释放锁。

运行库发布到 `runtimes/<bundle-id>/`，包含 `receipt.json` 和 `payload/`。内容清单记录文件摘要、权限和链接目标。发布通过同卷 rename 完成；已发布运行库保持原样，每次使用前重新校验。

下载由 reqwest 执行，重试由 backon 管理。归档校验 SHA-256 后，只提取 runtime 目录和许可通知；拒绝越界路径、重复条目、特殊文件及逃出 payload 的链接。

cjv 受管安装读取按 `.lifecycle.lock`、`.lifecycle.lookup.lock` 的顺序加锁。使用归档的来源标为 `archive-verified`，受管安装快照标为 `cjv-managed`。安装记录标识 cjv 管理的来源；与官方归档逐字节一致性的验证以归档摘要为准。

## 加载和进程

环境修改只作用于子进程。参数通过 `OsString` 直接传给进程接口。Windows 检查应用目录 DLL 遮蔽，Linux 检查 RPATH/RUNPATH 和预加载冲突，macOS 展开 dyld 的路径占位符并拒绝已识别的受限程序。

Windows 使用 Job Object，Linux 使用 subreaper。Unix 转发终止信号并恢复前台终端。派生进程退出状态待确认时保留运行库使用记录；同一主机重启后可以回收旧会话记录。

监督涉及进程级状态，同一进程内只允许一个监督实例。嵌入该 library 的宿主需协调进程级信号 disposition。缓存的信任边界是当前用户的本地文件系统；同一 UID 的进程可以修改缓存，网络文件系统的锁语义需由宿主另行适配。
