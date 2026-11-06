# Windows 动态仓颉库与混合版本可行性实验

日期：2026-11-07。实际执行平台：Windows x64。本实验不讨论 Windows 全静态链接支持。

## 结论

- 1.0.5、1.1.3、1.2.0 三组 `--dy-std` EXE + 自定义仓颉 DLL + 对应 SDK runtime 均实际运行成功。
- 三组 EXE/DLL 经 `llvm-strip --strip-all` 后仍保留 `.cjsdkv`，stripped 同版组合也全部成功。
- 独立 Rust `object 0.40.0` 探针对 12 个真实 EXE/DLL（3版本 × 2文件 × strip前后）准确提取版本，与 Python PE 解析一致。
- 动态链接产物的 EXE 版本表只记录自身版本，不替代依赖 DLL 的版本检查；自定义 DLL 有自己的版本表。应用依赖需要单独解析。
- 混合版本矩阵出现真实访问冲突和缺入口，部分跨版本组合通过了当前负载，但该结果不足以建立通用 ABI 兼容规则。

## 来源与编译

SDK 直接使用本机已有的 cjv 受管安装，未修改 SDK：

- `C:/Users/12009/.cjv/toolchains/lts-1.0.5`
- `C:/Users/12009/.cjv/toolchains/sts-1.1.3`
- `C:/Users/12009/.cjv/toolchains/sts-1.2.0`

该实验使用已有受管 SDK，没有逐文件对照官方归档。归档下载与摘要测试见 Windows 主矩阵报告。

库源码在 `windows-dylib/bridgeprobe.cj`：创建 1024 元素 Int64 数组并返回末项，以及构造一个 Exception 对象跨 DLL 返回。应用在 `windows-dylib/main.cj`：调用库函数，读取 Exception.message，打印结果，等待约一秒以便检查已加载模块。

每个版本使用自己的 cjo 编译配套应用，然后在运行目录替换为其他版本编译的同接口 DLL，以便真正测试运行期混用，不被 cjo 的编译期版本校验提前拦截。

编译命令模板：

```text
<SDK>/bin/cjc.exe bridgeprobe.cj --output-type=dylib --dy-std --output-dir <build> -o libbridgeprobe.dll
<SDK>/bin/cjc.exe main.cj --dy-std --import-path <build> -L <build> -lbridgeprobe -o <build>/app.exe
llvm-strip --strip-all -o <build>/stripped-app.exe <build>/app.exe
llvm-strip --strip-all -o <build>/stripped-libbridgeprobe.dll <build>/libbridgeprobe.dll
```

需要显式 `-lbridgeprobe` 链接自定义动态库；`--import-path` 只提供 cjo 信息。`--dy-libs` 在当前 SDK 已给出 deprecated 警告，因此最终可重复脚本不使用它。

应用导入表包含 `libbridgeprobe.dll`、`libcangjie-std-core.dll`、`libcangjie-runtime.dll`，库以动态方式链接。

## 运行矩阵

运行时清除继承的仓颉工具链环境，仅保留必要 Windows 变量；PATH 为指定 SDK 的 runtime 目录加 System32。工作目录为独立案例目录，没有全局 PATH 更改。通过 SetErrorMode 禁止缺 DLL/崩溃系统弹窗，防止负样本阻塞交互桌面。

每项成功的标准是退出码 0 且输出：

```text
1023
cross-library exception
```

| App 编译 SDK | 自定义 DLL 编译 SDK | runtime 1.0.5 | runtime 1.1.3 | runtime 1.2.0 |
| --- | --- | --- | --- | --- |
| 1.0.5 | 1.0.5 | 成功 | `0xC0000005` | `0xC0000005` |
| 1.0.5 | 1.1.3 | 成功 | `0xC0000005` | `0xC0000005` |
| 1.0.5 | 1.2.0 | 成功 | `0xC0000005` | `0xC0000005` |
| 1.1.3 | 1.0.5 | `0xC0000139` | `0xC0000005` | `0xC0000005` |
| 1.1.3 | 1.1.3 | `0xC0000139` | 成功 | 成功 |
| 1.1.3 | 1.2.0 | `0xC0000139` | 成功 | 成功 |
| 1.2.0 | 1.0.5 | `0xC0000139` | `0xC0000005` | `0xC0000005` |
| 1.2.0 | 1.1.3 | `0xC0000139` | 成功 | 成功 |
| 1.2.0 | 1.2.0 | `0xC0000139` | 成功 | 成功 |

共 27 项：11 项成功，10 项 `0xC0000005`（访问冲突），6 项 `0xC0000139`（入口未找到）。这些状态来自实际目标进程退出码，未将进程失败替换为 cjr 自己的诊断。

重要观察：

- App/DLL/runtime 同版的三项均成功。
- 1.0.5 App 放在 1.1.3 或 1.2.0 runtime 下，所有对应 DLL 组合均访问冲突。
- 1.1.3/1.2.0 App 用 1.0.5 runtime，均入口未找到。
- 1.1.3/1.2.0 App 在新 runtime 下混入 1.0.5 自定义 DLL，仍访问冲突；只检查主程序版本不足以防止这种情况。
- 1.1.3 与 1.2.0 之间的本次组合全部成功，且旧 App 配新 DLL 在旧 runtime 下也成功。但负载有限，不能推导整个 SDK 的兼容关系或放宽 exact 策略。
- 访问冲突样本没有成功打印业务输出。本次没有调试调用栈，不能进一步归因到某个对象字段、GC 或具体初始化步骤。

另有 3 项 stripped 同版运行，全部成功；3 项模块路径确认运行，也全部成功。

## 元数据与实际加载证据

对每个 `--dy-std` 产物：

| 产物 | 1.0.5 | 1.1.3 | 1.2.0 |
| --- | --- | --- | --- |
| App `.cjsdkv` | `["1.0.5"]` | `["1.1.3"]` | `["1.2.0"]` |
| DLL `.cjsdkv` | `["1.0.5"]` | `["1.1.3"]` | `["1.2.0"]` |
| strip 后 | 不变 | 不变 | 不变 |

Rust 探针使用 object 的 section VA 与 section data，将 `.cjsdkv` 的 64 位小端地址映射到版本字符串，执行边界/终止符校验；本次仅支持 PE32+ 小端格式，该原型仅适用于已知样本。源码与锁文件位于 `windows-dylib/rust-pe-probe/`，结果位于 `windows-dylib/rust-pe-results.json`。

Win32 Toolhelp32 实际模块枚举确认：自定义库来自各版本的研究 build 目录；`libcangjie-runtime.dll`、`libcangjie-std-core.dll`、`libboundscheck.dll` 来自本次指定 SDK 的 runtime 目录。对应记录在 results.json 的 `module_confirmation`。

## 重复实验与范围

```powershell
python research/windows-dylib/probe.py
$samples = Get-ChildItem research/windows-dylib/builds -Recurse -File | Where-Object { $_.Extension -in '.exe','.dll' } | ForEach-Object FullName
cargo run --manifest-path research/windows-dylib/rust-pe-probe/Cargo.toml -- @samples
```

`results.json` 保存编译命令、每个 PE 的版本/导入/摘要、27 项组合和 stripped 运行结果。附加模块确认记录是本轮额外检查；主脚本也已将自定义库纳入模块枚举范围。

本实验验证 Windows 动态链接场景和 Rust 元数据读取路径；不证明 Linux/macOS 执行、全部 SDK/编译选项、长期 GC/线程行为或任意动态插件均已覆盖。所有 runtime 切换发生在测试子进程环境，未修改 cjv SDK 或系统配置。
