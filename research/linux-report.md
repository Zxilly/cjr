# Linux x64 可行性实测

日期：2026-11-07。环境：WSL2 Ubuntu 25.04 x86_64，glibc 2.41；真实 Linux ELF loader 执行，不是模拟输出。以下是研究原型，不是已完成的 Rust cjr。

## 结论

**Linux x64 的核心路线已实际跑通：静态识别产物内 SDK 版本 → 精确选择官方发行版 → 从已校验 SDK 归档提取 runtime-only 目录 → 用干净子进程环境运行。**

三套官方 SDK 的匹配运行都通过。交叉版本测试存在真实 SIGSEGV 和缺符号错误，证明只匹配主版本或“有同名库即可”不可靠。RPATH 实测进一步证明仅设置 LD_LIBRARY_PATH 不能覆盖所有二进制。

## SDK 与来源

直接在线读取 [官方发行资产索引](https://github.com/Zxilly/cangjie-version-manifest/blob/master/versions.json)，按清单 URL 下载 Linux x64 SDK，完整 SHA-256 校验通过。

| SDK | 原始归档字节数 | runtime-only 文件字节数合计 |
| --- | ---: | ---: |
| 1.0.5 | 281,715,855 | 43,904,238 |
| 1.1.3 | 403,288,797 | 50,944,740 |
| 1.2.0 | 201,602,997 | 32,690,188 |

字节合计是文件逻辑大小，不等于磁盘实际分配空间。1.0.5 清单摘要为大写十六进制，需要大小写归一化；实际内容匹配，非上游校验错误。

下载清单、选择资产、摘要验证结果分别保存在 `linux/manifest.json`、`selected-assets.json`、`download-results.json`。压缩归档在 `linux/sdk-archives/`，解包 SDK 与运行库快照在 WSL `/root/.cache/cjr-research-linux/`，未改动系统 SDK 或全局环境。

## 样本与矩阵

`linux/probe.cj` 测试：

- ArrayList<Int64> 保存 10,000 个整数，校验和 `49,995,000`。
- 抛出并捕获 Exception，检查 message。
- 300 轮、每轮 2,000 个 Unicode 字符串，累计 600,000 次字符串构造，校验累计长度 `13,247,000`。这是分配压力测试，未单独测量或证明 GC 次数。
- `spawn` / `get` 返回并校验 `42`。

每个版本编译 default、`--dy-std`、`--static` 三种模式，随后 `strip --strip-all`，分别用无 runtime、1.0.5、1.1.3、1.2.0 runtime 执行。执行环境仅保留 PATH、LANG、HOME 及本次指定的 LD_LIBRARY_PATH。

另编译 `probelib` 动态库与调用它的应用，测试跨库 String 与 Int64 返回；追加不同版本的静态/动态库混合构建。

| 应用 SDK / 模式 | 无 runtime | runtime 1.0.5 | runtime 1.1.3 | runtime 1.2.0 |
| --- | --- | --- | --- | --- |
| 1.0.5 default | 缺库，127 | 通过 | 通过 | 通过 |
| 1.0.5 `--dy-std` | 缺库，127 | 通过 | **SIGSEGV，-11** | **SIGSEGV，-11** |
| 1.1.3 default | 缺库，127 | **缺符号，127** | 通过 | 通过 |
| 1.1.3 `--dy-std` | 缺库，127 | **缺符号，127** | 通过 | 通过 |
| 1.2.0 default | 缺库，127 | **缺符号，127** | 通过 | 通过 |
| 1.2.0 `--dy-std` | 缺库，127 | **缺符号，127** | 通过 | 通过 |
| 三版本 `--static` | 均通过 | 均通过 | 均通过 | 均通过 |

简单动态库样本在匹配 runtime 时全部通过，一些错配版本也通过；不能将简单样本通过扩大为 ABI 兼容保证。

主矩阵共 22 次成功构建、52 个运行组合，结果为 37 个退出 0、13 个退出 127、2 个 SIGSEGV。失败项是特意构造的缺库或错配测试，不是匹配路径失败。完整 stdout/stderr 和退出码在 `linux/results.json`。

## ELF 版本识别

实测 ELF 为 x86_64 little-endian PIE。版本信息位于 `.cjmetadata` 的 SDK version 子表，三个版本均可读取：

- 头部 magic 为 `0x12345678`。
- 已测布局的 SDK version 子表起始偏移从 metadata 头部读取，样本为 `0xb0`。
- 子表保存指向版本字符串的地址；对 PIE / .so 必须解释对应的 `R_X86_64_RELATIVE` 重定位 addend。
- 后续表起点按 16 字节对齐，SDK 表尾可能有 null padding；必须跳过零地址，不能将它映射到非加载节的地址 0。
- `strip --strip-all` 后 `.cjmetadata` 与所需动态重定位仍存在，读取不依赖 `.symtab`。

SDK 自带的 `lib/linux_x86_64_cjnative/cjld.lds` 明确包含 `__CJSDKVersion`、`__CJSDKVersionEnd`、`KEEP(*(.cjmetadata.sdk.version))`，对应提取依据保存在 `linux/<version>-linker-metadata.txt`。原型使用下一表起点界定扫描范围，其中包括对齐填充。完整 header 定义和异常输入需由 Rust 解析器单独检查。

混合静态库的结果很关键：

| 主程序编译版本 | 静态库编译版本 | 同一 ELF 中读到的版本序列 |
| --- | --- | --- |
| 1.2.0 | 1.1.3 | `1.2.0, 1.1.3, 1.2.0` |
| 1.1.3 | 1.0.5 | `1.1.3, 1.0.5, 1.1.3` |

这些简单混编程序执行成功，说明 MixedSdkVersions 是保守策略拒绝，不应描述为已证明不能运行。混合动态库则保留各自文件版本，主 EXE 不能替代依赖 .so 的检查。

## 自动启动原型与实际库路径

`linux/launcher_probe.py` 自动提取版本、查本地官方清单与已校验归档、仅提取 `runtime/lib/linux_x86_64_cjnative/` 和 LICENSE，然后生成干净运行环境。它没有使用开发工具链 PATH。

三版本的 default、dy-std、static 共 9 条路径全部退出 0；动态程序选中自己的精确版本，static 自动判为不需要仓颉 runtime。两个混合静态库样本被原型以 MixedSdkVersions 拒绝。结果保存在 `linux/launcher-results.json`。

`LD_DEBUG=libs` 记录实际初始化的库路径，确认 runtime、std-core、std-collection 等来自所选版本的 runtime-only 快照。完整记录在 `linux/*-runtime-only-loader.txt`，

`--static` 只表示仓颉库静态化：`readelf -d` 仍列出 `libstdc++.so.6`、`libm.so.6`、`libc.so.6`、`ld-linux-x86-64.so.2`。不能把它标为整个 Linux 程序完全静态或跨 glibc 通用。

## 加载优先级反例

用 1.0.5 编译 dy-std 压力程序，嵌入 1.1.3 runtime 路径，再设置 LD_LIBRARY_PATH 指向正确的 1.0.5：

| 嵌入类型 | 结果 |
| --- | --- |
| 旧式 `DT_RPATH` (`--disable-new-dtags`) | 实际选择冲突路径，SIGSEGV，-11 |
| `DT_RUNPATH` (`--enable-new-dtags`) | LD_LIBRARY_PATH 生效，退出 0 |

构建参数通过 cjc 1.0.5 的 `--link-options` 传入；该版本不支持后续版本新增的单数 `--link-option`。原始动态段与运行结果在 `linux/rpath-results.json`，loader 轨迹在 `linux/rpath-*-loader.txt`。

该反例支持 cjr 在运行前识别 RPATH 冲突并拒绝，不能承诺环境变量一定能纠正所有错误 runtime。

## 复现

在 WSL 中运行以下脚本（路径按 workspace 调整）：

```sh
cd /mnt/e/Project/CS_Project/2026/cjr/research/linux
python3 download_sdks.py
python3 extract_sdks.py
python3 run_matrix.py
python3 additional_checks.py
python3 run_launcher_probe.py
python3 rpath_probe.py
python3 record_provenance.py
```

小型可执行产物和动态库另保存在 `linux/artifacts/<version>/`。研究原型依赖 readelf 并只识别本次 ELF 布局，Rust 实现使用 object 解析动态依赖，另有畸形输入测试。

## 限制

- 仅本机 WSL2 Ubuntu 25.04 / glibc 2.41 / x86_64 实际运行；没有证明旧发行版、musl、ARM64 或其他内核组合兼容。
- 没有完成 cjv 缓存适配、CLI、缓存清理租约、并发事务、下载恢复或恶意文件 fuzz。这些仍需实现与测试。
- 未验证删除 ELF section headers、打包器重写、自定义 loader、任意 dlopen 插件和全部标准库 API。
- 跨版本成功仅针对样本；项目应继续默认 exact SDK 匹配。
