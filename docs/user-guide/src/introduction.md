# cjr 用户手册

cjr 为仓颉程序找到对应版本的运行库，然后启动程序。

```sh
cjr ./app
```

仓颉程序通常依赖 SDK 中的动态运行库，不同 SDK 版本之间不保证 ABI 兼容。cjr 读取程序中的完整 SDK 版本，通过 [官方发行索引](https://github.com/Zxilly/cangjie-version-manifest) 选择运行库，不尝试相邻版本。

已经准备好的 cjr 运行库会直接复用。首次准备时，cjr 优先读取 cjv 缓存和官方受管安装；本地没有匹配内容时，再下载官方 SDK、校验摘要并提取运行库。无需安装或配置 cjv。

## 适用范围

cjr 面向由同一官方 SDK 正确构建的命令行程序。应用自己的动态库、stdx 和系统库由应用分发者或系统提供。cjr 不导入自编 SDK，不修复跨版本混编，也不提供手动覆盖版本的选项。

下一步：[安装与运行](installation.md)。运行失败时，先用 [inspect](commands.md#查看程序) 查看版本和依赖，再对照[平台说明](platforms.md)。
