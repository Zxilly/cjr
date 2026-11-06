# Rust 依赖版本与可编译性调查

这是依赖选型阶段的记录。当前依赖版本见 [审计记录](../docs/dependency-audit.json)，最终选择以根目录 Cargo.toml 为准。
调查日期：2026-11-07。查询的是 crates.io API 的 `crate.max_stable_version`，然后查对应 version 的 `rust_version`；不是按发布列表第一条或 docs.rs 搜索摘要推断。该选择排除 prerelease 和 yanked 版本，保留 `toml` 的 SemVer build metadata。

## 当前最新稳定版本

| 依赖 | 最新稳定版本 | 声明的 MSRV | 用途与选择 |
| --- | --- | --- | --- |
| [`clap`](https://crates.io/crates/clap/4.6.7) | `4.6.7` | `1.85` | CLI 与默认执行/子命令解析 |
| [`object`](https://crates.io/crates/object/0.40.0) | `0.40.0` | `1.85` | PE/ELF/Mach-O 基础读取 |
| [`serde`](https://crates.io/crates/serde/1.0.229) | `1.0.229` | `1.56` | 配置/清单模型 |
| [`serde_json`](https://crates.io/crates/serde_json/1.0.154) | `1.0.154` | `1.71` | 版本清单、receipt、inspect/list JSON |
| [`toml`](https://crates.io/crates/toml/1.2.1+spec-1.1.0) | `1.2.1+spec-1.1.0` | `1.85` | 只读兼容 cjv 设置与安装记录 |
| [`reqwest`](https://crates.io/crates/reqwest/0.13.5) | `0.13.5` | `1.85.0` | 同步下载与元数据 HTTP |
| [`zip`](https://crates.io/crates/zip/9.4.1) | `9.4.1` | `1.88` | Windows SDK ZIP |
| [`tar`](https://crates.io/crates/tar/0.4.46) | `0.4.46` | `1.63` | Linux/macOS SDK TAR |
| [`flate2`](https://crates.io/crates/flate2/1.1.11) | `1.1.11` | `1.67.0` | Gzip 与 ZIP Deflate |
| [`sha2`](https://crates.io/crates/sha2/0.11.1) | `0.11.1` | `1.88` | SHA-256 校验与内容身份 |
| [`tempfile`](https://crates.io/crates/tempfile/3.27.0) | `3.27.0` | `1.63` | 私有 staging |
| [`semver`](https://crates.io/crates/semver/1.0.28) | `1.0.28` | `1.68` | 延后；精确版本匹配并不需要范围运算 |
| [`thiserror`](https://crates.io/crates/thiserror/2.0.21) | `2.0.21` | `1.77` | 结构化错误 |
| [`tracing`](https://crates.io/crates/tracing/0.1.44) | `0.1.44` | `1.65.0` | 延后；首版 EventSink 与 CLI stderr 足够 |
| [`windows-sys`](https://crates.io/crates/windows-sys/0.61.2) | `0.61.2` | `1.71` | Windows loader/进程/锁 API |
| [`libc`](https://crates.io/crates/libc/0.2.191) | `0.2.191` | `1.65` | Unix 信号与 flock 等平台 API |
| [`proptest`](https://crates.io/crates/proptest/1.12.0) | `1.12.0` | `1.88` | 仅开发依赖，解析和路径边界测试 |

原始查询结果保存在 `dependency-versions.json`，包含版本、MSRV、发布日期、features 与官方仓库 URL。证据可通过 `https://crates.io/api/v1/crates/<name>` 重新取得；版本查询时间为 2026-11-07。

## 推荐直接依赖范围

首版使用 clap、object、serde、serde_json、toml、reqwest、zip、tar、flate2、sha2、tempfile、thiserror；windows-sys/libc 按目标平台引入；proptest 仅作开发依赖。semver 和 tracing 的最新版本也已核对并纳入编译探针，但首版可以不直接依赖：SDK 版本身份用完整原串精确匹配，结构化 progress/error 由内部 EventSink 返回。reqwest 自身仍可能传递依赖 tracing。

不额外引入 openssl、native-tls、tokio 应用编排、数据库、外部 LLVM 工具绑定。reqwest blocking 内部仍使用 Tokio，这与“应用代码同步”并不矛盾。

建议 features：

- clap：`derive`。
- object：`default-features = false`，`read, std`，不需要写入或压缩调试段解析。
- serde：`derive`。
- toml：`default-features = false`，`std, serde, parse`，当前只读 cjv 文件，不需要 display/writer。
- reqwest 0.13：`default-features = false`，`blocking, rustls, system-proxy`。不启用 HTTP body 自动解压，避免下载字节与归档摘要发生隐式变化；JSON 可读取原始响应后用 serde_json 解析。
- zip：`default-features = false`，`deflate-flate2`；flate2 显式启用 `rust_backend`。暂不引入 SDK 不需要的 AES、Bzip2、Zstd、XZ 等格式。
- windows-sys：按实际 API 开启 Foundation、Storage_FileSystem、System_Threading、System_Console 等；不是启用全部 Win32。

上述 feature 组合保存在 `dependency-probe/Cargo.toml`，所有直接依赖用 `=version` 固定，完整传递依赖在 Cargo.lock 中。toml 要求写 `=1.2.1`，Cargo.lock 记录实际 `1.2.1+spec-1.1.0`；Cargo 的版本约束不使用 build metadata 区分兼容范围。

## 实测发现

1. 最新 TOML 是 `1.2.1+spec-1.1.0`，不能沿用旧设计印象中的 0.9。按包含连字符就判为 prerelease 会错误排除 `+spec-1.1.0`，因此使用注册表权威字段。
2. reqwest 0.13 的 feature 名是 `rustls`，不是旧版本示例中的 `rustls-tls`。默认 Rustls provider 为 aws-lc，这会引入 aws-lc-sys/C 构建工具；本机 Windows MSVC 环境已完成该 provider 编译。目标程序不依赖仓颉 DLL，但 cjr 本身的 Windows CRT/最低系统要求仍需发行构建验证。
3. sha2 0.11 的 digest 输出类型不能直接用于旧式 `format!("{:x}", digest)`；探针初次编译抓到这个 API 差异，改为逐字节十六进制编码后通过 check。无需为了编码再加一个库。
4. ZIP 的 `enclosed_name()` 在 Windows 返回原生路径分隔符，路径验证应比较 Path/组件而非直接比较带 `/` 的显示字符串；探针修正路径断言后通过。`enclosed_name()` 本身不能替代 symlink、文件覆盖、展开大小等外层检查。
5. 当前锁定图中已声明的最高 MSRV 为 1.88，包含 zip、sha2、proptest 和 ICU 传递依赖。本机是 rustc/cargo 1.97.1。声明门槛不等于已在 1.88 实测；本次只能确认 1.97.1 的 Windows MSVC 编译与运行结果。

## 验证范围

`dependency-probe` 是研究用小工程，不是 cjr 产品实现。命令：

```powershell
cargo check --manifest-path research/dependency-probe/Cargo.toml --all-targets --locked
cargo test --manifest-path research/dependency-probe/Cargo.toml --locked
```

check：已通过。test：7 passed，0 failed（Windows x86_64-pc-windows-msvc，rustc 1.97.1）。

测试覆盖 SHA-256 已知向量、TOML/JSON 解析、ZIP Deflate 往返、TAR/Gzip 往返、object 读取当前 Windows PE、Rustls blocking client 构建、临时文件读写。它验证依赖组合与相关基础 API，不代表 ABI decoder、网络下载重试、归档路径安全或 Linux/macOS 平台执行已完成。

所有 17 项都进入依赖解析；当前 Windows 条件编译不会证明 Unix libc 调用或 Windows 系统调用语义正确。跨平台 SDK 研究由其他并行工作单独报告。

## 官方资料

- [reqwest 0.13 文档与 feature](https://docs.rs/reqwest/0.13.5/reqwest/)
- [object 0.40 文档](https://docs.rs/object/0.40.0/object/)
- [zip 9.4.1 文档](https://docs.rs/zip/9.4.1/zip/)
- [clap 4.6.7 文档](https://docs.rs/clap/4.6.7/clap/)
