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
