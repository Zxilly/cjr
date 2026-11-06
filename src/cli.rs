use clap::{Parser, Subcommand};
use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Run {
        binary: OsString,
        args: Vec<OsString>,
        offline: bool,
        verbose: bool,
    },
    Inspect {
        binary: PathBuf,
        json: bool,
    },
    List {
        json: bool,
    },
    Clean {
        all: bool,
        dry_run: bool,
    },
}
#[derive(Parser)]
#[command(
    name = "cjr",
    version,
    about = "Run Cangjie binaries with their exact official runtime",
    override_usage = "cjr [--offline] [-v] <binary> [args...]\n       cjr inspect <binary> [--json]\n       cjr list [--json]\n       cjr cache clean [--all] [--dry-run]"
)]
struct Run {
    #[arg(long)]
    offline: bool,
    #[arg(short, long)]
    verbose: bool,
    #[arg(required = true)]
    binary: OsString,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<OsString>,
}
#[derive(Parser)]
#[command(name = "cjr")]
struct Helpers {
    #[command(subcommand)]
    command: Helper,
}
#[derive(Subcommand)]
enum Helper {
    Inspect {
        binary: PathBuf,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Cache {
        #[command(subcommand)]
        command: Cache,
    },
}
#[derive(Subcommand)]
enum Cache {
    Clean {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        dry_run: bool,
    },
}

/// Deliberately stop parsing cjr options at the binary. Clap's trailing vararg
/// parser otherwise consumes the first literal `--` belonging to the application.
pub fn parse(args: Vec<OsString>) -> std::result::Result<Action, clap::Error> {
    let mut i = 1;
    let mut forced = false;
    while i < args.len() {
        if args[i] == "--" {
            forced = true;
            i += 1;
            break;
        }
        if args[i] == "--offline" || args[i] == "-v" || args[i] == "--verbose" {
            i += 1;
            continue;
        }
        if args[i].to_string_lossy().starts_with('-') {
            return Run::try_parse_from(args).map(|r| Action::Run {
                binary: r.binary,
                args: r.args,
                offline: r.offline,
                verbose: r.verbose,
            });
        }
        break;
    }
    if !forced
        && args
            .get(i)
            .is_some_and(|x| x == "inspect" || x == "list" || x == "cache")
    {
        return Helpers::try_parse_from(args).map(|h| match h.command {
            Helper::Inspect { binary, json } => Action::Inspect { binary, json },
            Helper::List { json } => Action::List { json },
            Helper::Cache {
                command: Cache::Clean { all, dry_run },
            } => Action::Clean { all, dry_run },
        });
    }
    if i >= args.len() {
        return Run::try_parse_from(args).map(|r| Action::Run {
            binary: r.binary,
            args: r.args,
            offline: r.offline,
            verbose: r.verbose,
        });
    }
    let app_args = args[i + 1..].to_vec();
    let run = Run::try_parse_from(args[..=i].to_vec())?;
    Ok(Action::Run {
        binary: run.binary,
        args: app_args,
        offline: run.offline,
        verbose: run.verbose,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundary() {
        for tail in [
            vec!["--", "a"],
            vec!["--help", "--offline", "--version"],
            vec!["a b", "中文"],
        ] {
            let mut a = vec!["cjr", "--offline", "app"];
            a.extend(tail.clone());
            let Action::Run { args, offline, .. } =
                parse(a.into_iter().map(Into::into).collect()).unwrap()
            else {
                panic!()
            };
            assert!(offline);
            assert_eq!(args, tail.iter().map(OsString::from).collect::<Vec<_>>());
        }
        assert!(matches!(
            parse(["cjr", "--", "inspect"].map(Into::into).to_vec()).unwrap(),
            Action::Run { .. }
        ));
        assert!(matches!(
            parse(["cjr", "inspect", "a", "--json"].map(Into::into).to_vec()).unwrap(),
            Action::Inspect { json: true, .. }
        ));
    }
}
