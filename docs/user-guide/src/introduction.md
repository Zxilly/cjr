# cjr 用户手册

cjr 为仓颉程序找到对应版本的运行库，然后启动程序。

```sh
cjr ./app
```

仓颉程序通常依赖对应 SDK 中的动态运行库，运行库的 ABI 随 SDK 版本变化。cjr 读取程序中的完整 SDK 版本，通过 [官方发行索引](https://github.com/Zxilly/cangjie-version-manifest) 选择版本完全匹配的运行库。

已经准备好的 cjr 运行库会直接复用。首次准备时，cjr 优先读取 cjv 缓存和官方受管安装；本地没有匹配内容时，再下载官方 SDK、校验摘要并提取运行库。cjr 可以独立使用，也会自动识别已有的 cjv 安装。

## 适用范围

cjr 面向由同一官方 SDK 正确构建的命令行程序。应用自己的动态库、stdx 和系统库由应用分发者或系统提供。运行库版本由程序元数据自动确定，来源限定为官方发行版。

下一步：[安装与运行](installation.md)。运行失败时，先用 [inspect](commands.md#查看程序) 查看版本和依赖，再对照[平台说明](platforms.md)。
