# Windows x64 可行性实测

日期：2026-11-07。Windows 原生编译与执行；使用 Python 研究脚本。

## SDK 来源

| SDK | 归档字节数 | 来源 | 官方清单 SHA-256 |
| --- | ---: | --- | --- |
| 1.0.5 | 297274602 | 本次官方下载 | 通过，清单大写摘要先归一化 |
| 1.1.3 | 392128226 | 本次官方下载 | 通过 |
| 1.2.0 | 262872802 | 自动命中现有 cjv 持久归档缓存 | 重新计算完整摘要，通过 |

三套 SDK 均从以上已校验归档独立解包到 `windows/sdks/`，本主矩阵的编译器、标准库与运行库来自这些研究副本。没有改动全局 SDK 或 PATH。

1.0.5 与 1.1.3 的下载最终由 WSL Python 完成，仍是 Windows 官方 ZIP；下载宿主不改变最终编译执行平台。可复现下载脚本为 `windows/download_archives.py`，主测试脚本也包含官方下载和 cjv 缓存查找逻辑。第一次下载脚本因未归一化大写摘要需要修正；最终归档均按归一化摘要验证。

## 负载与执行条件

`windows/workload.cj` 检查：20,000 项字符串集合、抛出/捕获异常并验证 message、spawn/get 求和、100 批共 200,000 次字符串分配及集合长度校验。最后等待一秒以便系统枚举已加载库。该测试统计分配操作，未测量 GC 次数。

每个 SDK 编译默认、`--dy-std`、`--dy-std -s`、`--static` 四种模式。子进程环境只保留必需 Windows 变量，PATH 仅包含本次 runtime 目录及 System32。缺库和崩溃样本禁止系统错误弹窗，设置超时，返回真实退出状态。

## 实际运行矩阵

前三种动态运行模式的结果相同：

| 编译 SDK | 无 runtime | runtime 1.0.5 | runtime 1.1.3 | runtime 1.2.0 |
| --- | --- | --- | --- | --- |
| 1.0.5 | `0xC0000135` 缺 DLL | 通过 | `0xC0000005` 访问冲突 | `0xC0000005` 访问冲突 |
| 1.1.3 | `0xC0000135` 缺 DLL | `0xC0000139` 缺入口 | 通过 | 通过 |
| 1.2.0 | `0xC0000135` 缺 DLL | `0xC0000139` 缺入口 | 通过 | 通过 |

三版本 × 三种动态运行模式的精确匹配 **9/9 通过**。主矩阵共 12 次构建、48 次运行；包含有意构造的失败与下述静态链接实验，不能把48项统称全部通过。完整输出、编译命令、退出状态和实际模块路径在 `windows/results.json`。

### --static 的版本差异

| SDK | 编译行为 | 干净环境运行 |
| --- | --- | --- |
| 1.0.5 | 明确警告 `--static` 仅对 Linux 有效，PE 仍导入 runtime/boundscheck | 缺库；配匹配 runtime 才通过 |
| 1.1.3 | 生成不依赖仓颉 runtime DLL 的 PE | 压力样本访问冲突 |
| 1.2.0 | 生成不依赖仓颉 runtime DLL 的 PE | 压力样本通过 |

针对 1.1.3 的异常做了最小化复现，见 `windows/static_isolation.py` 和 `static-isolation.json`：静态 Hello World 退出0；仅 `spawn { => 42 }` 加 `get()` 的静态样本退出 `0xC0000005`；1.2.0 的两项均退出0。没有调试根因，不能进一步归咎具体 runtime 实现。该失败发生于不使用外部仓颉 runtime 的程序，cjr 切换 DLL 无法修复；动态模式同版本测试正常。

因此不能仅凭手册或 `--static` 参数认定所有版本都不需要 runtime，也不能将 Windows 不同 SDK 的静态能力一概而论。实际导入依赖才是检查依据。

## PE 元数据与自动启动

12 个主构建产物均能从 `.cjsdkv` 的地址条目读到版本；普通模式含多个相同版本条目，动态标准库模式只保留应用自己的相关记录。指针通过 image base/RVA/section raw offset 映射到文件，不使用普通字符串搜索。编译器 `-s` 后仍保留版本记录。

`windows/auto_launch.py` 是自动流程原型：

1. 从目标 PE 内容提取唯一版本，而非根据文件名指定版本。
2. 查官方清单中的 Windows x64 资产与研究归档 receipt。
3. 将该发行物的全部 runtime DLL 准备到独立 `windows/runtimes/<version>/`，不包含编译器、cjo 或开发工具。
4. 用只含这个 runtime-only 目录与 System32 的 PATH 启动。
5. 通过 Windows Toolhelp32 获取实际模块路径。

`windows/run_auto_matrix.py` 的三版本 × default/dynamic-std/stripped **9/9 成功**，并断言枚举出的所有仓颉/边界检查模块实际来自 `windows/runtimes/`。结果在 `windows/auto-launch-results.json`。这是自动选择和运行的证明，不是完整缓存适配或生产级 parser。

另将 1.0.5 `libcangjie-runtime.dll` 放到 1.2.0 动态样本的 EXE 同目录，PATH 指向正确1.2.0，仍失败（`0xC0000139`）。结果保存在主结果的 `shadowing` 字段。应用目录的旧 DLL 会遮蔽选定环境；cjr 必须识别这种冲突。

## 补充 Rust 与动态库验证

独立研究见 `windows-dylib-report.md`：3个SDK构建自定义仓颉DLL，27个App/DLL/runtime混合组合；匹配与strip后的匹配全部成功，错配出现访问冲突/缺入口。Rust `object 0.40.0` 对12个EXE/DLL准确提取版本和导入表，证明 Rust 基础库可以承担该任务。

## 复现与边界

```powershell
python research/windows/probe.py
python research/windows/run_auto_matrix.py
python research/windows/static_isolation.py
```

需要 Windows Python、llvm-readobj 和网络或已有归档；脚本是研究用途，格式支持、解包策略和超时边界不等同于生产实现。

尚未验收：cjv 安装目录并发更新协议、CLI分流、缓存清理租约、完整动态依赖搜索、未知metadata布局、任意delay-load/LoadLibrary插件、恶意输入和旧Windows版本。没有向用户机器安装全局 runtime。
