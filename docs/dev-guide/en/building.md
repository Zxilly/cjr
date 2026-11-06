# Build and test

## Requirements

Use Rust 1.89 or newer, a C compiler and CMake. Windows requires MSVC. The TLS dependency aws-lc compiles native code.

```sh
cargo build --release --locked
```

The executable is `target/release/cjr`, or `target/release/cjr.exe` on Windows. `Cargo.lock` pins the dependency graph; commit it together with dependency changes.

## Routine checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```

Tests cover binary parsing, CLI arguments, download failures, cache lifecycle and process state. HTTP scenarios use mockito; arbitrary-input tests use proptest. Ordinary tests do not download official SDKs.

CI runs these checks and release builds on Windows, Linux, macOS ARM64 and Intel. A separate Linux job checks the minimum Rust version. The website job builds the landing page and documentation.

## Implementation rules

- Keep FFI and necessary unsafe operations inside the private `platform` module, behind safe interfaces with documented preconditions.
- Hold `StoreTransaction` for cache mutations and usage registration. Never modify cjv-owned files.
- Use backon for retries, walkdir for directory traversal and object for binary formats.
- Retain usage records when descendant liveness is uncertain. Cleanup failures must not replace the application's exit status.
