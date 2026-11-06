# cjr 可行性研究汇总

2026-11-07 的 SDK 实验记录。当前 Rust 实现的测试结果见 [验收记录](../docs/implementation.md)。

## 结论

Windows x64 与 Linux x64 在已测试官方 SDK 1.0.5、1.1.3、1.2.0 上，已经跑通“读取产物版本 → 精确选择官方运行库 → 独立 runtime-only 环境执行”。两平台自动启动原型各9项通过，并取得实际加载库路径证据。

macOS ARM64 与 x64 的真实 Mach-O 版本读取和strip后保留已验证；没有Mac宿主，未完成原生执行。不能将macOS标记为端到端支持。

实验包括跨版本与混编负样本。cjr 的支持范围是同一官方 SDK 正确构建的程序，混编结果只用于研究。



## 资产与平台

| 平台 | SDK | 验证层级 |
| --- | --- | --- |
| Windows x64 | 1.0.5、1.1.3、1.2.0 | 官方摘要、真实编译、实际运行、库路径、strip、混版本、runtime-only |
| Linux x64 | 1.0.5、1.1.3、1.2.0 | 官方摘要、WSL2真实ELF编译执行、库路径、strip、混版本、runtime-only、RPATH冲突 |
| macOS ARM64 | 1.0.5、1.2.0 | 官方摘要、真实SDK产物静态读取、strip、依赖闭包 |
| macOS x64 | 1.3.0-alpha.20261107001016 | 研究下载，缺上游摘要；真实产物静态读取、strip、依赖闭包；不满足当前自动支持策略 |

共研究9份SDK归档：8份本次新下载、Windows1.2.0复用已有cjv归档缓存。8份有官方预期摘要并验证通过；macOS x64 Nightly仅有官方URL来源与本地计算摘要，单独标注未认证。

## 实验发现

1. 精确匹配必要：错误runtime可导致真实访问冲突、SIGSEGV或缺符号；“启动过一次”不能证明ABI普遍兼容。
2. 主程序元数据用于确定SDK，依赖图用于定位运行库与加载路径。混编实验曾观察到多个版本记录，但当前范围不要求跨SDK编译审计或兼容求解。
3. ELF PIE版本表需要重定位处理和null padding处理；Mach-O实际节为 `__CJ_METADATA,__cjsdkversion`；不可以跨格式硬套PE布局。
4. Windows同目录DLL和Linux旧RPATH确实能够压过正确环境变量，预检冲突是核心功能。
5. runtime提取必须保留符号链接：macOS新SDK的PCRE2使用两级链接。
6. 清单摘要需要大小写归一化；当前macOS x64 Nightly缺摘要，按既定策略拒绝自动准备。
7. 静态链接能力随平台和版本变化：Windows1.0.5忽略该选项，1.1.3静态线程最小样本崩溃，1.2.0通过。Linux仓颉静态化后仍依赖系统libc等。

## 依赖与报告

- [依赖候选与 features/MSRV 测试](dependencies.md)：17项候选逐项查询crates.io，选定组合cargo check通过、7测试通过；首版可省semver/tracing直接依赖。
- [Windows主矩阵与自动启动](windows-report.md)：12构建、48运行矩阵，另9项runtime-only自动启动及4项静态问题最小化。
- [Windows自定义DLL与Rust读取](windows-dylib-report.md)：27组合、3项strip同版、3项实际库路径，12个PE的Rust解码。
- [Linux矩阵与自动启动](linux-report.md)：22构建、52运行组合，另9项runtime-only原型及RPATH对照。
- [macOS静态研究](macos-report.md)：三SDK产物、版本与库依赖，不含原生执行。

研究脚本与小型结果在各子目录。大型SDK、归档、runtime快照和Cargo构建目录已列入根`.gitignore`。Windows/macOS资料保留在研究目录；Linux展开SDK与runtime快照保留在WSL `/root/.cache/cjr-research-linux/`，归档与报告在workspace。

## 未覆盖的场景

这些实验未覆盖旧 glibc、musl、ARM64 Linux、打包器改写产物和任意动态插件。macOS 的 dyld 与签名行为需要原生主机测试。
