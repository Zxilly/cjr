use crate::{
    Error, Result, catalog, cli::Action, config::Config, inspect, loader, provision, store::Store,
    util,
};
use std::io::Write;
mod listing;
pub use listing::local_report;

pub fn execute(action: Action) -> Result<crate::process::Outcome> {
    execute_with_output(action, &mut std::io::stdout().lock())
}

/// Write reports through a fallible writer, including closed pipes and full disks.
pub fn execute_with_output(
    action: Action,
    output: &mut impl Write,
) -> Result<crate::process::Outcome> {
    let config = Config::discover()?;
    match action {
        Action::Run {
            binary,
            args,
            offline,
            verbose,
        } => {
            let path = loader::resolve_binary(&binary)?;
            let report = inspect::inspect(&path)?;
            if report.target != crate::model::Target::host() || !report.executable {
                return Err(Error::new(
                    "HostIncompatible",
                    "target must be a native executable for this host",
                ));
            }
            let version = report.sdk_version.as_deref().ok_or_else(|| {
                Error::new(
                    "UnknownSdkVersion",
                    format!("{}: {}", path.display(), report.diagnostics.join("; ")),
                )
            })?;
            let initial = loader::graph(&report, None, false)?;
            if verbose {
                crate::diagnostics::emit(format_args!(
                    "cjr: SDK {version}, {}",
                    report.target.platform()?
                ));
                for w in &config.warnings {
                    crate::diagnostics::emit(format_args!("cjr: {w}"));
                }
            }
            if !initial.needs_runtime {
                let plan = loader::plan(&report, None, args)?;
                return crate::process::launch(plan, None);
            }
            let store = Store::new(&config);
            let guard = store.guard()?;
            // Verified receipts retain official asset identity even after metadata cleanup.
            let mut warnings = vec![];
            let receipts = store.receipts(&mut warnings);
            let identities = receipts
                .iter()
                .map(|(_, r)| r.artifact.clone())
                .collect::<Vec<_>>();
            let artifact =
                if let Some(a) = catalog::exact(&identities, version, report.target.platform()?)? {
                    a.validate()?;
                    a
                } else {
                    catalog::resolve(
                        &config,
                        &guard,
                        version,
                        report.target.platform()?,
                        offline,
                        verbose,
                    )?
                };
            let ready = provision::ensure(&config, &guard, &artifact, offline, verbose)?;
            let plan = loader::plan(&report, Some(&ready), args)?;
            let lease = guard.begin_use(&ready.receipt.bundle_id)?;
            drop(guard);
            crate::process::launch(plan, Some(lease))
        }
        Action::Inspect { binary, json } => {
            let path = loader::resolve_binary(binary.as_os_str())?;
            let report = inspect::inspect(&path)?;
            let local = local_report(&config)?;
            let mut runtime = None;
            if let (Some(v), Ok(p)) = (&report.sdk_version, report.target.platform()) {
                for (_, r) in Store::new(&config).receipts(&mut vec![]) {
                    if r.artifact.version == *v
                        && r.artifact.platform == p
                        && let Ok(rel) = util::relative(&r.runtime_dir)
                    {
                        runtime = Some(
                            config
                                .home
                                .join("runtimes")
                                .join(&r.bundle_id)
                                .join("payload")
                                .join(rel),
                        );
                        break;
                    }
                }
            }
            let graph = loader::graph(&report, runtime.as_deref(), false).unwrap_or_else(|e| {
                loader::Graph {
                    needs_runtime: report.needs_runtime(),
                    diagnostics: vec![e.to_string()],
                    ..Default::default()
                }
            });
            let candidates = local
                .entries
                .iter()
                .filter(|e| {
                    report.sdk_version.as_ref() == Some(&e.version)
                        && report.target.platform().is_ok_and(|p| p == e.platform)
                })
                .collect::<Vec<_>>();
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({"schema_version":1,"binary":report,"dependency_graph":graph,"local_candidates":candidates,"warnings":local.warnings})
                    )?
                )?;
            } else {
                writeln!(
                    output,
                    "{}\n  {} {}/{}\n  SDK: {}\n  Metadata: {} ({} entries)\n  Dynamic Cangjie runtime: {}",
                    report.path.display(),
                    report.format,
                    report.target.os,
                    report.target.arch,
                    report.sdk_version.as_deref().unwrap_or("unknown"),
                    report.metadata_decoder,
                    report.version_entries,
                    if graph.needs_runtime {
                        "required"
                    } else {
                        "not detected"
                    }
                )?;
                if let Some(v) = &report.minimum_os {
                    writeln!(output, "  Minimum OS: {v}")?;
                }
                for edge in &graph.edges {
                    writeln!(
                        output,
                        "  {} -> {}",
                        edge.library,
                        edge.resolved
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| edge.status.clone())
                    )?;
                }
                for c in candidates {
                    for s in &c.sources {
                        writeln!(
                            output,
                            "  Cache: {} [{}] {}",
                            s.kind,
                            s.status,
                            s.path.display()
                        )?;
                    }
                }
                for d in report
                    .diagnostics
                    .iter()
                    .chain(&graph.diagnostics)
                    .chain(&local.warnings)
                {
                    writeln!(output, "  Note: {d}")?;
                }
            }
            Ok(crate::process::Outcome::Exited(0))
        }
        Action::List { json } => {
            let report = local_report(&config)?;
            if json {
                writeln!(output, "{}", serde_json::to_string_pretty(&report)?)?;
            } else {
                if report.entries.is_empty() {
                    writeln!(output, "No local runtime resources found.")?;
                }
                for e in &report.entries {
                    writeln!(output, "{}  {}", e.version, e.platform)?;
                    for s in &e.sources {
                        writeln!(
                            output,
                            "  {} [{}] {}{}{}",
                            s.kind,
                            s.status,
                            s.path.display(),
                            s.bytes.map(|n| format!(" [{n} bytes]")).unwrap_or_default(),
                            s.usage
                                .as_ref()
                                .map(|s| format!(" ({s})"))
                                .unwrap_or_default()
                        )?;
                    }
                }
                for warning in &report.warnings {
                    crate::diagnostics::emit(format_args!("cjr: {warning}"));
                }
            }
            Ok(crate::process::Outcome::Exited(0))
        }
        Action::Clean { all, dry_run } => {
            let result = Store::new(&config).clean(all, dry_run)?;
            for item in &result {
                writeln!(output, "{}: {}", item.status, item.path.display())?;
            }
            Ok(crate::process::Outcome::Exited(
                if result.iter().any(|e| e.status.is_failure()) {
                    1
                } else {
                    0
                },
            ))
        }
    }
}
