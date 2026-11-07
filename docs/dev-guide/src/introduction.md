# cjr 开发指南

cjr 使用 Rust 实现。一个 Cargo package 同时包含 CLI 和 library，用户接口限定为程序执行、`inspect`、`list` 和 `cache clean`。

修改命令行为时，同步更新中英文用户手册。通用能力优先使用标准库或维护中的依赖；版本识别、官方资产选择、cjv 适配和缓存策略由项目自身实现。

仓库保存源码、常规测试和可维护的文档。研究目录、SDK 下载、探测脚本、审计或验收报告及构建产物保留在本地忽略目录中。

问题和改进建议可以提交到 [GitHub Issues](https://github.com/Zxilly/cjr/issues)。提交前读取最近的 commit 信息，保持仓库现有风格。
