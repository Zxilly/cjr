# Platforms and troubleshooting

## Platforms

| Platform | Binary format | Runtime search environment |
| --- | --- | --- |
| Windows x64 | PE | Child process `PATH` |
| Linux x64 | ELF | `LD_LIBRARY_PATH` |
| macOS ARM64 / x64 | Mach-O | `DYLD_LIBRARY_PATH` |

Availability also requires an official SDK asset for the exact version and architecture with a SHA-256 checksum. Linux supports x64. macOS supports native command-line programs and selects the matching architecture slice. Application distributors handle `.app` bundles and Rosetta translation.

## Check SDK version metadata

Run `cjr inspect <binary>` first. ELF requires section headers. Mach-O supports both ordinary version pointers and chained fixups. Version detection requires an intact version table with a supported layout. Parsing failures produce a diagnostic. Ask the application distributor for a build that preserves version metadata.

## Check official release records

The complete SDK version, platform and official checksum must match. Check that the manifest includes the target version and its checksum; cjr uses an exact version match. Offline use also requires local metadata or an existing runtime record.

## Runtime present, launch still fails

Check application-local libraries and embedded search paths. Windows application-directory DLLs, old ELF RPATH entries, preload variables and explicit library paths can take precedence over the chosen runtime. cjr rejects detected conflicts. The application distributor should correct the conflicting files or paths.

Application distributors provide application libraries and stdx; the system package manager supplies glibc and other system dependencies. Use the system loader's diagnostic to identify missing dependencies.

## Restricted macOS programs

cjr rejects loading-environment injection for recognized hardened or restricted programs. Signatures, entitlements and SIP stay intact. Applications distributed this way should bundle and sign their own runtimes.

## Non-ASCII arguments

cjr passes native operating-system arguments. Some Cangjie Windows SDKs may fail to decode Chinese arguments in `main(args)`. If direct execution fails in the same way, the SDK or application needs to address it.

Static dependency checks read dependencies declared in the binary. The system loader or application reports errors from runtime plugins, delay loading and custom loaders.
