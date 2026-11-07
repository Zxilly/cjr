# 命令参考

## 执行程序

```text
cjr [--offline] [-v] <binary> [args...]
```

| 参数 | 行为 |
| --- | --- |
| `--offline` | 只使用本地元数据、缓存或受管安装 |
| `-v`、`--verbose` | 输出运行准备信息 |
| `<binary>` | 目标程序路径 |
| `[args...]` | 原样传给目标程序 |

`cjr ./app --help` 查看的是应用帮助。查看 cjr 帮助使用 `cjr --help`。若程序名恰好是 `inspect`、`list` 或 `cache`，使用路径写法 `./inspect`，或用 `cjr -- inspect` 明确执行入口。

cjr 保留目标程序的退出状态，准备或启动失败会输出诊断并以非零状态退出。

## 查看程序

```sh
cjr inspect ./app
cjr inspect ./app --json
```

读取程序格式、架构、SDK 版本及动态依赖，并显示本地候选运行库。该命令以只读方式检查本地文件。

## 查看缓存

```sh
cjr list
cjr list --json
```

列出本地可见的 cjr 和 cjv 资产，包括来源及使用状态。查询范围为本地内容。

## 清理缓存

```sh
cjr cache clean --dry-run
cjr cache clean
cjr cache clean --all --dry-run
cjr cache clean --all
```

默认清理下载归档、元数据和废弃暂存内容，保留已准备的运行库。`--all` 还会清理能确认未使用的运行库。`--dry-run` 预览清理结果并保留所有文件。清理始终只作用于 cjr 自己的目录。

## JSON 输出

`inspect --json` 和 `list --json` 输出带 `schema_version` 的 JSON。自动化使用时应检查命令退出码和该字段，并容忍新增字段。警告与未知状态应保留并交由调用方处理；成功匹配以明确的匹配状态为准。
