# Cache and cjv

## Cache location

cjr uses `.cjr` in the user home directory by default. Set `CJR_HOME` to choose another directory:

```sh
CJR_HOME=/path/to/cjr-cache cjr ./app
```

```powershell
$env:CJR_HOME = 'D:\Caches\cjr'
cjr .\app.exe
```

Use independent, non-overlapping paths for the cjr and cjv directories. Use a local filesystem directory writable by the current user.

## Automatic cjv reuse

An existing prepared cjr runtime is used directly. When preparing a new one, cjr tries:

1. cjv archives matching the official checksum.
2. cjv managed SDK installations with matching official source records.
3. Archives stored by cjr.
4. The official download URL.

cjr uses compatible locks when reading a cjv installation and copies runtime files into its own storage. cjv-owned content stays intact.

An absolute `CJV_HOME` takes precedence. Otherwise, cjr reads the `home` setting from the cjv user configuration, then the system configuration, and finally falls back to `.cjv` in the user home. `CJV_CONFIG_FILE` and `CJV_FALLBACK_SETTINGS` can select those configuration files.

cjr uses official SDK caches and managed installations with compatible layouts and complete source records. It skips other content and proceeds to the next source automatically.

## Content retained by cleanup

cjr registers runtime use before launching the application. `cache clean --all` skips runtimes that may still be used by the program or its descendants.

On macOS, confirming that every descendant has exited may require the same host to restart; usage records remain until confirmation. Forced termination of cjr and unreadable records can also delay cleanup.

Cleanup retains coordination locks and operates on files and links themselves within cjr storage. Use `cjr cache clean` to select reclaimable runtimes based on usage records.
