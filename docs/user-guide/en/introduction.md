# cjr User Guide

cjr finds the runtime that matches a Cangjie program and launches it.

```sh
cjr ./app
```

Cangjie programs usually depend on dynamic libraries shipped with their SDK. Runtime ABI compatibility is not guaranteed across SDK versions. cjr reads the full SDK version from the binary and selects an official asset using [cangjie-version-manifest](https://github.com/Zxilly/cangjie-version-manifest). It does not substitute a nearby version.

Prepared cjr runtimes are reused directly. When preparing a runtime, cjr first checks cjv archives and official managed installations, then its own archives. If none are available, it downloads the official SDK, verifies its checksum and extracts the runtime. Installing or configuring cjv is optional.

## Scope

cjr supports command-line programs and dependencies correctly built with the same official SDK. The application distributor remains responsible for application libraries, stdx and system dependencies. cjr does not import custom SDKs, repair mixed-version builds or accept a manual version override.

Continue with [installation](installation.md), or use [inspect](commands.md#inspect-a-program) and the [platform guide](platforms.md) to diagnose a launch failure.
