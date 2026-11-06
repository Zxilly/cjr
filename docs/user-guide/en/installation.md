# Install and run

## Download a prebuilt binary

Download the archive for your system and architecture from [GitHub Releases](https://github.com/Zxilly/cjr/releases/latest). Rust is not required.

| System | v0.0.1 download |
| --- | --- |
| Windows x64 | [ZIP](https://github.com/Zxilly/cjr/releases/download/v0.0.1/cjr-v0.0.1-x86_64-pc-windows-msvc.zip) |
| Linux x64 | [tar.gz](https://github.com/Zxilly/cjr/releases/download/v0.0.1/cjr-v0.0.1-x86_64-unknown-linux-musl.tar.gz) |
| macOS Apple Silicon | [tar.gz](https://github.com/Zxilly/cjr/releases/download/v0.0.1/cjr-v0.0.1-aarch64-apple-darwin.tar.gz) |
| macOS Intel | [tar.gz](https://github.com/Zxilly/cjr/releases/download/v0.0.1/cjr-v0.0.1-x86_64-apple-darwin.tar.gz) |

Each archive has a `.sha256` checksum file in the release. Verify it with `sha256sum -c <filename>.sha256` on Linux or `shasum -a 256 -c <filename>.sha256` on macOS. On Windows, use `Get-FileHash <archive-path> -Algorithm SHA256` and compare the result with the checksum file.

Extract the archive and add the directory containing `cjr` (`cjr.exe` on Windows) to `PATH`. On Linux / macOS, you can install it in your user command directory:

```sh
mkdir -p ~/.local/bin
install -m 755 cjr ~/.local/bin/cjr
```

Ensure `~/.local/bin` is on `PATH`. On Windows, place `cjr.exe` in a permanent directory and add that directory to your user `Path` through Environment Variables. Open a new terminal and run:

```sh
cjr --version
# cjr 0.0.1
```

Windows binaries include the C/C++ runtime and do not require a separate Visual C++ Redistributable installation. The macOS binaries require macOS 13 or newer. The Linux binary is statically linked with musl; the Cangjie application and its runtime still have their own system requirements.

## Install from source

Install Rust 1.89 or newer, a C compiler and CMake. Use the MSVC Rust toolchain on Windows.

```sh
cargo install --git https://github.com/Zxilly/cjr --tag v0.0.1 --locked
```

Ensure Cargo's `bin` directory is on your `PATH`, then run:

```sh
cjr --version
```

To install from a local checkout:

```sh
git clone https://github.com/Zxilly/cjr.git
cd cjr
cargo install --path . --locked
```

## Run a program

Linux / macOS:

```sh
cjr ./app --port 8080
```

Windows PowerShell:

```powershell
cjr .\app.exe --port 8080
```

The first launch may download a full SDK archive. Only the runtime is extracted; later launches reuse it. cjr leaves the executable at its original location and preserves the working directory, standard streams and application arguments.

## Offline use

Run the program once while online, then use:

```sh
cjr --offline ./app
```

Offline mode forbids network requests. Existing runtime records, or local metadata with matching SDK content, must identify an official asset. Missing content produces an error.

Put `--offline` and `-v` before the executable path. Everything after that path belongs to the application.
