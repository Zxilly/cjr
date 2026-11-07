# Command reference

## Run a program

```text
cjr [--offline] [-v] <binary> [args...]
```

| Argument | Behavior |
| --- | --- |
| `--offline` | Use only local metadata, caches and managed installations |
| `-v`, `--verbose` | Print runtime preparation details |
| `<binary>` | Path to the executable |
| `[args...]` | Pass unchanged to the application |

`cjr ./app --help` requests the application's help. Use `cjr --help` for cjr itself. If an executable is named `inspect`, `list` or `cache`, specify a path such as `./inspect`, or use `cjr -- inspect`.

cjr preserves the child's exit status. Preparation and launch errors print a diagnostic and return a nonzero status.

## Inspect a program

```sh
cjr inspect ./app
cjr inspect ./app --json
```

Show the binary format, architecture, SDK version, dynamic dependencies and local runtime candidates. Inspection is a read-only operation on local files.

## List local content

```sh
cjr list
cjr list --json
```

List visible cjr and cjv assets with their source and usage state. The query covers local content.

## Clean the cache

```sh
cjr cache clean --dry-run
cjr cache clean
cjr cache clean --all --dry-run
cjr cache clean --all
```

The default removes archives, metadata and abandoned staging content while retaining prepared runtimes. `--all` also removes runtimes known to be unused. `--dry-run` previews the operation. Cleanup only affects cjr-owned content.

## JSON output

`inspect --json` and `list --json` include a `schema_version` field. Automation should check the exit status and schema version, accept additional fields and handle warnings and unknown states explicitly. Treat a match as successful only when the reported state confirms it.
