# 构建与测试

## 环境

需要 Rust 1.89 或更新版本、C 编译器和 CMake。Windows 使用 MSVC。TLS 依赖中的 aws-lc 会编译本地代码。

```sh
cargo build --release --locked
```

可执行文件位于 `target/release/cjr`，Windows 为 `target/release/cjr.exe`。`Cargo.lock` 固定完整依赖图；更新依赖时同时提交锁文件。

## 常规检查

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

测试覆盖二进制解析、命令参数、下载错误、缓存生命周期和进程状态。HTTP 场景使用 mockito，任意输入测试使用 proptest。常规测试不下载官方 SDK。

CI 在 Windows、Linux、macOS ARM64 和 Intel 上运行这些检查并构建 release，另在 Linux 检查最低 Rust 版本。网站任务构建 landing 页和文档。

## 实现约束

- 业务模块不直接写平台 FFI。必要的 unsafe 集中在私有 `platform` 模块，并说明调用前提。
- 缓存变更和使用登记必须持有 `StoreTransaction`；不得修改 cjv 所有的文件。
- 下载退避使用 backon，遍历目录使用 walkdir，二进制解析使用 object。
- 不确定派生进程是否退出时保留使用记录。清理失败不得覆盖目标程序退出状态。
