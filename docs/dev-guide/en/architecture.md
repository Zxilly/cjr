# Architecture

## Modules

| Module | Responsibility |
| --- | --- |
| `app`, `cli` | Command parsing and orchestration |
| `inspect` | PE, ELF and Mach-O version and dependency metadata |
| `catalog` | Exact SDK version and platform matching in official manifests |
| `cjv` | cjv configuration, archives and managed installations |
| `provision` | Downloads, verification and selective extraction |
| `store` | Immutable runtime snapshots, usage records and cleanup |
| `loader` | Host compatibility, search paths and dependencies |
| `process` | Launch, signals, exit status and descendant supervision |
| `platform` | Native operations behind safe interfaces |

## Launch flow

cjr reads metadata and checks host architecture and dependencies. When a runtime is needed, it selects an exact official asset with a checksum, reuses local content or downloads an archive, then extracts and publishes a runtime snapshot. After final dependency checks, it registers usage, releases the storage lock and launches the program.

The SDK version comes from PE `.cjsdkv`, ELF `.cjmetadata` or Mach-O `__CJ_METADATA,__cjsdkversion`. It identifies a release, not a universal ABI fingerprint. Parsers validate bounds and reject unknown layouts.

Mach-O chained fixups are decoded by `object`; `inspect::macho::fixups` maps rebases into the version table. Supported formats are `DYLD_CHAINED_PTR_64`, `64_OFFSET`, `ARM64E`, `ARM64E_USERLAND` and `ARM64E_USERLAND24`, including authenticated pointers. Inspection reads the file without loading it or authenticating pointers. Version entries must reference local strings, not external symbol bindings. Traversal has a work limit and checks file-to-virtual-address mappings, page boundaries and duplicate entries.

## Cache lifecycle

`StoreTransaction` holds an exclusive storage lock for metadata, preparation, publication, usage registration and cleanup. Initial preparation of different versions is serialized. The lock is released while the application runs.

Runtimes live under `runtimes/<bundle-id>/` with `receipt.json` and `payload/`. The manifest records content hashes, permissions and link targets. Publication uses a same-volume rename. Published runtimes are immutable and verified before each use.

reqwest performs downloads and backon manages retries. After SHA-256 verification, extraction selects runtime directories and license notices, rejecting unsafe paths, duplicate entries, special files and links escaping the payload.

cjv installation reads acquire `.lifecycle.lock` before `.lifecycle.lookup.lock`. Archive-derived snapshots are marked `archive-verified`; managed-installation snapshots are `cjv-managed`. An installation record does not prove byte-for-byte equality with the official archive.

## Loading and supervision

Environment changes apply only to the child process. Arguments use `OsString` without shell concatenation. Windows checks application-directory DLL shadowing, Linux checks RPATH/RUNPATH and preload conflicts, and macOS expands dyld path tokens and rejects recognized restricted programs.

Windows uses Job Objects and Linux uses a subreaper. Unix forwards termination signals and restores terminal foreground ownership. Usage records are retained when all descendants cannot be proven to have exited; records from a previous boot on the same host can be reclaimed.

Supervision changes process-wide state and permits only one supervisor per process. The library does not guarantee unchanged signal dispositions in arbitrary embedding hosts. Storage targets a user-owned local filesystem, without isolation against malicious same-UID replacement or network-filesystem locking differences.
