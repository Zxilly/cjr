# macOS SDK 可行性研究

日期：2026-11-07。执行宿主：Windows x64。此报告是实际下载文件的静态验证，**没有在 macOS 原生执行仓颉程序**。

## 结论

从官方 macOS SDK 的现有仓颉程序与动态标准库读取准确 SDK 版本可行。三个发行物的 `cjpm` 都能通过 Mach-O 的 `__CJ_METADATA,__cjsdkversion` 节离线读取完整版本，去除符号后仍可读取。运行库的 `@rpath` 依赖在归档内部可以按文件名闭合，必须保留 PCRE2 符号链接。

实验没有 Mac 宿主，因此未编译自定义 macOS 程序，也未验证实际 dyld 加载路径、代码签名/SIP 或跨版本运行。

另外，当前官方版本清单的 `darwin-x64` 只有 Nightly 发行物，查询到的 30 条记录 SHA-256 全部为空；最新下载 URL 对应 `.sha256` 返回 HTTP 404。按照目前 cjr 的“必须有官方预期摘要”策略，这些发行物不能自动准备，应报告 `MissingChecksum`。本次仅为研究从清单提供的官方 URL 下载了一个 x64 Nightly，记录本地摘要，上游摘要缺失，无法校验。

## 下载资产

清单来源：

- `https://raw.githubusercontent.com/Zxilly/cangjie-version-manifest/master/versions.json`
- `https://raw.githubusercontent.com/Zxilly/cangjie-version-manifest/master/nightly.json`

| 目标 | 版本 | 实际字节数 | 清单摘要校验 |
| --- | --- | ---: | --- |
| darwin-arm64 | 1.0.5 | 186714867 | 通过 |
| darwin-arm64 | 1.2.0 | 163787681 | 通过 |
| darwin-x64 | 1.3.0-alpha.20261107001016 | 179298314 | 无上游摘要，未验证 |

归档共 529800862 字节，约 505.3 MiB。归档仍保存在 `research/macos/archives/`，没有改动全局 SDK。

完整摘要：

```text
mac-arm64 1.0.5:
e07d22584237f065b30a771b7110a87a8931334c40be3a148090944ae916ed1f

mac-arm64 1.2.0:
de88d6ba2185c94979e0ab2fe8ac1d81535dfdff3fcb51fbba71e4b99d851f97

mac-x64 1.3.0-alpha.20261107001016 (仅本地计算):
985adf4ebe5bbbad18cde75004aa5d71ec38fcf9a0ee0be4fc8c1940b6876058
```

## 元数据实测

样本来自每个官方 SDK 的 `tools/bin/cjpm` 和 `runtime/lib/*/*.dylib`，版本通过专用元数据节读取。

| SDK | cjpm 版本记录数量 | 唯一版本 | strip 后读取 | 含版本元数据的动态库数量 |
| --- | ---: | --- | --- | ---: |
| arm64 1.0.5 | 25 | `1.0.5` | 25 条，准确 | 45 |
| arm64 1.2.0 | 50 | `1.2.0` | 50 条，准确 | 46 |
| x64 Nightly | 50 | `1.3.0-alpha.20261107001016` | 50 条，准确 | 47 |

所有上述版本条目的解析错误数为 0。读取过程：

1. 解析 little-endian 64-bit Mach-O header 和 `LC_SEGMENT_64`。
2. 定位 segment `__CJ_METADATA` 下 section `__cjsdkversion`。
3. 按 8 字节读取各个地址条目。
4. 通过 segment 的 `vmaddr/fileoff/filesize` 将地址映射到文件偏移。
5. 读取有界 NUL 结尾版本串，保留完整 prerelease/build 内容。

`__CJMETAHEADER,__cjmetaheader` 也实际存在：1.0.5 中大小 0x140，1.2.0 与本次 Nightly 中大小 0x150。不能将 header 固定大小写死为一种布局。

`cjc` 和 `libcangjie-runtime.dylib` 本体没有 `__cjsdkversion`，因此没有从它们提取版本。这是检测输出，不能将“缺少该节”直接解释为无效 SDK。`libcangjie-std-core.dylib` 则每个样本都有一条对应版本记录。

三个 cjpm 都是单一架构 Mach-O，不是 universal/fat 文件。arm64 CPU type 为 `0x0100000c`，x64 为 `0x01000007`。尚未验证 fat slice 选择。

这三个 cjpm 使用 `LC_DYLD_INFO_ONLY`，没有 `LC_DYLD_CHAINED_FIXUPS`；runtime 本体则确实存在 chained fixups，但本体没有需要解码的 SDK 节。因此当前原型不能据此声称支持“SDK 版本指针本身采用 chained fixups 编码”的一般情况。

## 依赖与布局

runtime 目录实际为：

```text
arm64: cangjie/runtime/lib/darwin_aarch64_cjnative/
x64:   cangjie/runtime/lib/darwin_x86_64_cjnative/
```

三个 cjpm 均通过 `@rpath/libcangjie-runtime.dylib`、`@rpath/libboundscheck.dylib` 引用运行库。对应 LC_RPATH 是：

```text
@loader_path/../../runtime/lib/darwin_aarch64_cjnative
@loader_path/../../runtime/lib/darwin_x86_64_cjnative
```

runtime 自身 LC_ID_DYLIB 为 `@rpath/libcangjie-runtime.dylib`，LC_RPATH 为 `@loader_path`，依赖包括 `@rpath/libboundscheck.dylib`、`/usr/lib/libc++.1.dylib` 和 `/usr/lib/libSystem.B.dylib`。系统库依赖留给 macOS 提供，未在 Windows 上判断可加载性。

归档内所有普通 dylib 的 `@rpath/<basename>` 依赖都能找到同目录文件或归档中的有效相对符号链接。这里验证的是静态候选闭包，不能替代 dyld 搜索优先级验证。

重要的实际差异：1.0.5 的 PCRE2 别名是普通文件；1.2.0 和 x64 Nightly 使用以下链接链：

```text
libpcre2-8.dylib -> libpcre2-8.0.dylib -> libpcre2-8.0.14.0.dylib
```

`libcangjie-std-regex.dylib` 导入的是 `@rpath/libpcre2-8.0.dylib`。只复制普通文件会造成表面上的依赖缺失；cjr 提取 runtime 时必须保留链接或安全地等价物化。此次 Windows 研究脚本只提取普通文件，另从 tar header 建立链接映射验证目标与循环，未创建系统 symlink。

## strip 与签名边界

对三个 cjpm 运行 `llvm-strip --strip-all -o <copy> <original>`，输出退出码均为 0，原始文件保持不变；版本元数据均保留。

两个 arm64 cjpm 含 `LC_CODE_SIGNATURE`，strip 后文件仍显示该 load command；本次未运行 Apple codesign 工具或验证签名有效性。x64 Nightly cjpm 没有该 load command。不能将 strip 静态读取成功当成签名可执行性测试。

cjr 正常运行不需要 strip、重写 Mach-O 或移动应用；读取原文件版本后准备 runtime 即可。macOS 签名、DYLD 环境、SIP、hardened runtime、第三方插件动态加载仍须原生宿主验证。

## 复现与证据

在仓库根目录执行：

```powershell
python research/macos/download.py
python research/macos/extract.py
python research/macos/inspect_macho.py
python research/macos/inspect_libraries.py
```

`download.py` 消费保存的清单快照，流式下载并验证有预期摘要的归档；x64 研究例外在结果中记录 `checksum_verified: false`。其他脚本读取已保存的资产。

证据文件：

- `research/macos/downloads.json`：资产、字节数、预期/实际摘要、校验状态。
- `research/macos/inspection.json`：cjpm/cjc/runtime/core 与 strip 样本的 section、版本、load commands、依赖和 RPATH。
- `research/macos/library-summary.json`：所有 runtime dylib 的版本解析汇总与链接感知依赖闭包。
- `research/macos/extraction.json`：提取的普通文件和从归档读取的符号链接记录。
- `research/macos/versions.json`、`nightly.json`：本次查询的版本清单快照。

解析脚本用于已知 SDK 样本。Rust 实现的输入检查、fat slice 测试和 fixup 限制见当前实现文档。
