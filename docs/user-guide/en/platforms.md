# Platforms and troubleshooting

## Platforms

| Platform | Binary format | Runtime search environment |
| --- | --- | --- |
| Windows x64 | PE | Child process `PATH` |
| Linux x64 | ELF | `LD_LIBRARY_PATH` |
| macOS ARM64 / x64 | Mach-O | `DYLD_LIBRARY_PATH` |

Availability also requires an official SDK asset for the exact version and architecture with a SHA-256 checksum. Linux ARM64 is not implemented. macOS selects a native architecture slice, does not use Rosetta and does not manage `.app` bundles.

## Unknown SDK version

Run `cjr inspect <binary>` first. ELF requires section headers. Mach-O supports both ordinary version pointers and chained fixups. Missing or damaged metadata and unknown layouts produce a diagnostic rather than a guessed version. Ask the application distributor for a build that preserves version metadata.

## Unavailable official asset

The complete SDK version, platform and official checksum must match. Missing versions or checksums do not trigger fallback to a newer or older runtime. Offline use also requires local metadata or an existing runtime record.

## Runtime present, launch still fails

Check application-local libraries and embedded search paths. Windows application-directory DLLs, old ELF RPATH entries, preload variables and explicit library paths can take precedence over the chosen runtime. cjr rejects detected conflicts. The application distributor should correct the conflicting files or paths.

cjr does not install application libraries, stdx, glibc or other system dependencies. Use the system loader's diagnostic to identify missing dependencies.

## Restricted macOS programs

cjr rejects loading-environment injection for recognized hardened or restricted programs. It does not change signatures, entitlements or SIP. Applications distributed this way should bundle and sign their own runtimes.

## Non-ASCII arguments

cjr passes native operating-system arguments. Some Cangjie Windows SDKs may fail to decode Chinese arguments in `main(args)`. If direct execution fails in the same way, the SDK or application needs to address it.

Static dependency checks cannot cover arbitrary plugins, delay loading or custom loaders. Those failures are reported by the system loader or application.
