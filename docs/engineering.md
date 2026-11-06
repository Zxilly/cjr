# 维护说明

## 依赖

通用基础设施优先使用标准库或维护中的成熟依赖。禁止在业务模块新增重试框架、信号处理框架、递归遍历器、HTTP 测试服务器或自行管理原生句柄生命周期。项目代码负责仓颉元数据、官方资产身份、cjv 协议、加载顺序与缓存使用状态等业务规则。

以下依赖版本在 2026-11-07 核对，并由 Cargo.toml / Cargo.lock 固定：

| 能力 | 实现 | 项目负责的规则 |
| --- | --- | --- |
| 下载退避与锁等待 | backon 1.6.0 | 暂时性错误分类、次数/时间上限 |
| 目录遍历 | walkdir 2.5.0 | 不跟随链接、快照范围、数量与体积限制 |
| 十六进制编码 | hex 0.4.3 | 无自定义编码器 |
| Unix 系统接口 | rustix 1.1.6 | 锁协议、进程组、前台终端、subreaper 策略 |
| Unix 信号通知 | signal-hook 0.4.6 | 转发哪些信号、何时返回退出状态 |
| 终端操作的线程信号掩码 | nix 0.31.3、scopeguard 1.2.0 | 临时阻塞 SIGTTOU，作用域结束恢复原掩码 |
| HTTP 故障测试 | mockito 1.7.2（开发依赖） | 断流、503、404、摘要不符的验收断言 |
| 原子临时文件、句柄所有权、递归删除 | tempfile、std::os、std::fs | 所有权校验与发布顺序 |

HTTP 客户端为 reqwest。下载重试覆盖响应体读完前的断流，因此使用 backon 包裹完整下载操作；关闭 reqwest 内部重试，避免两层重试叠加。最多发出三次应用层下载请求，使用指数退避与 jitter。404、摘要不符、文件大小超限、磁盘写入失败不重试。HTTP 408、429、5xx、连接/超时和响应体读取故障允许重试。

全部 Rust 直接依赖（包括开发和平台依赖）的锁定版本与最新稳定版本对照见 [dependency-audit.json](dependency-audit.json)。TOML 的 `+spec-1.1.0` 是版本构建元数据，审计使用 Cargo.lock 实际解析版本比较。Dependabot 配置每周检查 Cargo 与 GitHub Actions 更新，升级仍需通过 CI。

## 模块

```text
cli                  参数边界与命令契约
app                  用例编排、可失败的输出接口
  listing            本地资源查询与聚合
inspect              PE / ELF / Mach-O 元数据解析
catalog              官方索引、精确资产身份
http                 共用 HTTP 客户端配置
cjv                  cjv 配置与存储协议适配、受管 SDK 快照
provision            来源选择与准备流程
  download           HTTP 下载、退避策略、摘要验证
  archive            选择性提取、安全路径与链接策略
  validation         官方 runtime 布局与版本验证
store                存储所有权与事务能力
  inventory          内容树与完整性验证
  lease              使用登记、撤销与保留状态
  clean              清理范围与类型化结果
loader               依赖图、宿主约束、启动计划
process              子进程监督、状态返回、RAII 清理
platform             私有 OS 边界与必要 FFI
```

缓存逻辑不直接下载；平台层不决定 SDK 版本；二进制解析不执行目标。查询不触发安装。各层以 Result 返回失败，进程退出只由 CLI 入口处理。

## 编译约束

library 根默认 `deny(unsafe_code)`，只有私有 `platform` 模块允许必要的原生调用。每个 unsafe 块要求安全前提说明，`unsafe_op_in_unsafe_fn` 和 Clippy `undocumented_unsafe_blocks` 为 deny。生产代码禁止 `unwrap`、`expect`、`panic!`、`unreachable!`；测试断言不受此限制。

Windows 使用标准库 OwnedHandle 拥有 Win32 句柄，拒绝 NULL / INVALID_HANDLE_VALUE。Unix 进程监督与文件锁使用 rustix / signal-hook 的安全接口。必要 FFI 仅包括 Windows Job/控制台/线程恢复/兼容 cjv 的单字节锁，以及平台启动身份、glibc 版本查询。

## 缓存与进程

1. `StoreTransaction` 持有排他锁。准备、元数据写入、发布、启动前登记都要求该能力；clean 使用相同锁。目标运行期间释放存储锁。
2. 发布只消费本存储 staging 内的普通目录。外部 SDK、链接路径、已有 runtime 不得被移动。已有同 ID bundle 必须验证其实际 receipt 和内容树。
3. 归档与 metadata 先写入临时位置，验证后发布。runtime 发布失败最多留下可清理的 staging，不将未完成目录当作有效 runtime。
4. 未进入 spawn 的租约在 Drop 时撤销；spawn 失败主动撤销。开始执行后，只有获得派生进程全部退出的证明才移除记录。未知、跨主机、无法解析的记录保守保留。
5. 子进程创建后，内部错误路径由 RAII 终止并等待直属子进程；无法证明没有后台派生进程时保留租约。清理失败记录诊断，不覆盖目标程序退出码。
6. Unix 前台终端、线程信号掩码与 Linux subreaper 状态有作用域恢复；SIGCHLD 驱动退出等待。tcsetpgrp 期间通过 nix 阻塞 SIGTTOU，避免后台终端恢复失败。检查派生进程时使用 NOWAIT，不消费其他组件的子进程退出状态。
7. `Outcome` 区分退出码和 Unix 信号。只有 CLI 决定自身退出。报告通过 fallible Write 输出，关闭管道不会触发 println panic。

监督接口是 CLI 的内部执行边界，涉及进程级控制台、信号和 subreaper 状态，同一进程内拒绝并发监督。它不是任意宿主程序均可无副作用嵌入的进程库：signal-hook 注销回调不保证恢复原始 OS signal disposition。CLI 完成监督后即退出。此约束不影响目标程序的参数、cwd、标准流和退出状态透传。

## 验证与运行假设

常规 CI 执行格式检查、严格 Clippy、单元/集成测试、release 构建；官方 SDK 的真实工作负载另行验收。回归覆盖网络断流与有限重试、禁止永久错误重试、外部目录保护、损坏缓存拒绝、使用中清理跳过、启动失败、信号退出、非本组件子进程状态和输出失败。

`scripts/terminal_acceptance.py` 使用 pexpect 创建真实控制终端，再运行进程回归，检查前台进程组、线程信号掩码与 subreaper 状态恢复。测试要求存在控制终端。CI 的 Unix job 包含该检查，另有 Rust 1.89 最低工具链编译检查。

缓存是当前用户独占管理的本地目录，不提供针对同一 UID 恶意并发替换文件、网络文件系统锁语义或断电持久化顺序的隔离保证。原子 rename 保证正常文件系统上的发布可见性，不等于所有平台断电后仍已落盘。启动时仍需再次检查内容完整性。

macOS 解析和官方归档提取有真实样本回归，ARM64 原生 SDK 执行由 CI 验收；Intel job 检查编译、常规测试和终端行为。无法证明后代进程全部退出时保留租约；此保守策略可能让 clean 暂时无法回收某些 runtime。
