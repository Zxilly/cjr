# Install and run

## Install from source

Install Rust 1.89 or newer, a C compiler and CMake. Use the MSVC Rust toolchain on Windows.

```sh
cargo install --git https://github.com/Zxilly/cjr --locked
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
