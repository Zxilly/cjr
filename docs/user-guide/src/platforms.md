# 平台与常见问题

## 平台支持

| 平台 | 目标格式 | 运行库加载环境 |
| --- | --- | --- |
| Windows x64 | PE | 子进程 `PATH` |
| Linux x64 | ELF | `LD_LIBRARY_PATH` |
| macOS ARM64 / x64 | Mach-O | `DYLD_LIBRARY_PATH` |

平台可用性还取决于官方清单是否提供该版本、该架构且具有 SHA-256 摘要的 SDK。Linux ARM64 当前未实现。macOS 选择与宿主原生架构一致的 slice，不通过 Rosetta 转换架构，也不管理 `.app`。

## 找不到 SDK 版本

先运行 `cjr inspect <binary>`。ELF 需要保留 section headers。Mach-O 支持普通版本指针和 chained fixups。版本表缺失、损坏或布局未知时，cjr 不会通过扫描任意字符串来猜测版本。请向程序分发者索取保留版本元数据的构建产物。

## 官方资产不可用

完整 SDK 版本、平台和官方摘要必须匹配。清单中没有该版本或摘要为空时，不会尝试较新或较旧的运行库。离线情况下还需要相应的本地元数据或已准备好的运行库记录。

## 已有运行库却仍然报错

检查应用目录和嵌入的搜索路径。Windows 应用目录的同名 DLL、Linux 旧式 RPATH、预加载变量或显式库路径可能覆盖 cjr 选择的运行库。发现冲突时，cjr 拒绝启动。应由程序分发者移除或修正冲突文件与路径。

应用自己的动态库、stdx、系统 glibc 和其他系统依赖不会由 cjr 安装。使用系统加载器诊断定位这些缺失项。

## macOS 受限程序

cjr 对已识别的 hardened 或 restricted 程序拒绝注入加载环境。它不修改程序签名、entitlements 或 SIP。需要此类分发方式的程序应自行携带并签名其运行库。

## 非 ASCII 参数

cjr 使用操作系统原生参数接口。部分仓颉 Windows SDK 的 `main(args)` 在解码中文参数时可能报错；若直接运行程序也有同样错误，应在 SDK 或应用中处理。

静态依赖检查不覆盖运行时插件、延迟加载或任意自定义加载器。此类错误由系统加载器或目标程序报告。
