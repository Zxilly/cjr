# Releasing

`package.version` in `Cargo.toml` is the version source. Clap embeds it at compile time; `cjr --version` and `cjr -V` report that value without Git or repository files at runtime.

## Publish a version

1. Update the package version in `Cargo.toml` and `Cargo.lock`, the website package version and installation examples in both languages.
2. Run formatting, Clippy, tests and `npm test` in `web/`. Commit and push the changes.
3. Create a matching stable tag, such as `git tag -a v0.0.1 -m "cjr v0.0.1"`, then run `git push origin v0.0.1`.

`release.yml` checks that the tag matches the package and calls the normal CI workflow. Once checks pass, it creates a draft release. `taiki-e/upload-rust-binary-action` builds and uploads four platform archives and their SHA-256 files:

- Windows x64: `x86_64-pc-windows-msvc`, ZIP.
- Linux x64: `x86_64-unknown-linux-musl`, tar.gz.
- macOS Apple Silicon: `aarch64-apple-darwin`, tar.gz.
- macOS Intel: `x86_64-apple-darwin`, tar.gz.

Archives are named `cjr-v<version>-<target>` and contain the executable and both READMEs. The minimum macOS version is 13.0. Each release binary is run with `--version`. Once all jobs succeed, `softprops/action-gh-release` publishes the draft. Failed jobs leave it unpublished and can be rerun in Actions. Do not move published tags; ship subsequent fixes in a new version.

Actions are pinned to full commit SHAs and tracked by Dependabot. The workflow uses GitHub's `GITHUB_TOKEN`, granting write access only to jobs that create, upload or publish the release.

## Static linking

`.cargo/config.toml` enables `crt-static` for Windows MSVC and Linux musl. Windows embeds the C/C++ runtime while retaining Windows system DLL imports. Linux release binaries must have neither dynamic dependencies nor an ELF interpreter. Rust dependencies and bundled native libraries are linked statically. macOS retains system libraries under `/usr/lib/` and `/System/Library/`; libSystem and system frameworks are not replaced with static libraries.

The release workflow reads each final binary's dependency table and rejects a dynamic Windows CRT, any Linux dynamic dependency, or non-system macOS libraries. This policy applies to cjr itself; Cangjie applications still use their matching dynamic runtime.
