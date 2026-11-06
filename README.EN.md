# cjr — Cangjie Runtime Manager

[![CI](https://img.shields.io/github/actions/workflow/status/Zxilly/cjr/ci.yml?branch=master&style=flat-square)](https://github.com/Zxilly/cjr/actions/workflows/ci.yml)

English | [中文](README.md)

cjr matches a Cangjie program to its official runtime and launches it. It reads the full SDK version from binary metadata, reuses cjv content where available and downloads an SDK only when matching local content is missing.

```sh
cjr ./app --port 8080
```

## Install

Download the archive for your platform from [GitHub Releases](https://github.com/Zxilly/cjr/releases/latest), extract it and add `cjr` to `PATH`. Prebuilt binaries are available for Windows x64, Linux x64 and macOS ARM64/x64. Each archive includes a SHA-256 checksum alongside it.

To build from source:

Requires Rust 1.89+, a C compiler and CMake. Use MSVC on Windows.

```sh
cargo install --git https://github.com/Zxilly/cjr --tag v0.0.1 --locked
```

See [Install and run](docs/user-guide/en/installation.md) for installation and PATH details.

## Common commands

```sh
cjr --offline ./app
cjr inspect ./app --json
cjr list
cjr cache clean --dry-run
cjr cache clean --all
```

Arguments after the executable path go directly to the application. `inspect` and `list` read local content only. Cleanup affects only cjr-owned storage, never cjv files; `--all` still retains runtimes with uncertain usage state.

Runtime availability for Windows x64, Linux x64 and macOS ARM64/x64 depends on the [official release manifest](https://github.com/Zxilly/cangjie-version-manifest). Programs must be correctly built with one official SDK. Application libraries, stdx and system dependencies must be provided separately. See [Platforms and troubleshooting](docs/user-guide/en/platforms.md).

## Documentation

- [User guide](docs/user-guide/en/introduction.md): installation, commands, cache and troubleshooting.
- [Command reference](docs/user-guide/en/commands.md): argument boundaries, JSON output and cleanup.
- [Developer guide](docs/dev-guide/en/introduction.md): builds, module responsibilities and maintenance.
- [Documentation and website](docs/dev-guide/en/website.md): local development and builds for the landing page and bilingual mdBooks.

## Development

```sh
cargo build --release --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

The website lives in `web/`, and the manuals in `docs/`. Report problems through [Issues](https://github.com/Zxilly/cjr/issues).
