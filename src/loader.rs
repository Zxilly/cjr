use crate::{
    Error, Result,
    inspect::{self, InspectReport},
    store::Ready,
    util,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
pub struct Edge {
    pub from: PathBuf,
    pub library: String,
    pub resolved: Option<PathBuf>,
    pub status: String,
}
#[derive(Debug, Default, Serialize)]
pub struct Graph {
    pub modules: Vec<InspectReport>,
    pub edges: Vec<Edge>,
    pub diagnostics: Vec<String>,
    pub needs_runtime: bool,
}
pub struct LaunchPlan {
    pub binary: PathBuf,
    pub args: Vec<OsString>,
    pub environment: Vec<(String, OsString)>,
    pub checked: Vec<(PathBuf, String)>,
}

pub fn resolve_binary(input: &OsStr) -> Result<PathBuf> {
    let path = Path::new(input);
    let has_path = path.is_absolute()
        || path.components().count() > 1
        || input.to_string_lossy().contains(['/', '\\']);
    let mut candidates = vec![];
    if has_path {
        candidates.push(path.to_owned());
    } else {
        // Windows searches cwd for a bare executable, Unix follows PATH only.
        if cfg!(windows) {
            candidates.push(std::env::current_dir()?.join(path));
        }
        for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
            candidates.push(dir.join(path));
        }
    }
    for p in candidates {
        let mut possibilities = vec![p.clone()];
        if cfg!(windows) && p.extension().is_none() {
            possibilities.push(p.with_extension("exe"));
        }
        for candidate in possibilities {
            if candidate.is_file() {
                return Ok(candidate.canonicalize()?);
            }
        }
    }
    Err(Error::new("BinaryNotFound", path.display().to_string()))
}
fn environment_paths(key: &str) -> Vec<PathBuf> {
    std::env::var_os(key)
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default()
}
fn basename(s: &str) -> &str {
    s.rsplit(['/', '\\']).next().unwrap_or(s)
}
fn expand(s: &str, image: &Path, main: &Path) -> Result<PathBuf> {
    let loader = image.parent().unwrap_or(Path::new("."));
    let executable = main.parent().unwrap_or(Path::new("."));
    let text = s
        .replace("${ORIGIN}", &loader.to_string_lossy())
        .replace("$ORIGIN", &loader.to_string_lossy())
        .replace("@loader_path", &loader.to_string_lossy())
        .replace("@executable_path", &executable.to_string_lossy());
    if text.contains('$') || text.starts_with('@') {
        return Err(Error::new(
            "LoaderPolicyDenied",
            format!("unsupported loader token in {s}"),
        ));
    }
    Ok(PathBuf::from(text))
}
fn system(name: &str, os: &str) -> bool {
    if os == "macos" {
        return name.starts_with("/usr/lib/") || name.starts_with("/System/Library/");
    }
    if os == "windows" {
        let n = name.to_ascii_lowercase();
        return n.starts_with("api-ms-win-")
            || n.starts_with("ext-ms-win-")
            || std::env::var_os("SystemRoot")
                .is_some_and(|p| PathBuf::from(p).join("System32").join(name).is_file());
    }
    false
}
fn system_path(p: &Path) -> bool {
    p.starts_with("/lib")
        || p.starts_with("/lib64")
        || p.starts_with("/usr/lib")
        || p.starts_with("/usr/lib64")
}
fn identical(p: &Path, selected: &Path) -> Result<()> {
    if p.canonicalize()? != selected.canonicalize()?
        && util::hash_file(p)? != util::hash_file(selected)?
    {
        return Err(Error::new(
            "LibraryShadowing",
            format!(
                "{} shadows selected runtime {}",
                p.display(),
                selected.display()
            ),
        ));
    }
    Ok(())
}
fn runtime_libraries(runtime: Option<&Path>) -> Result<BTreeMap<String, PathBuf>> {
    let mut out = BTreeMap::new();
    if let Some(dir) = runtime {
        for p in util::children(dir)? {
            if p.is_file() {
                let name = util::file_name(&p)?.to_string_lossy();
                if name.ends_with(".dll") || name.contains(".so") || name.ends_with(".dylib") {
                    out.insert(name.to_ascii_lowercase(), p);
                }
            }
        }
    }
    Ok(out)
}
pub fn graph(main: &InspectReport, runtime: Option<&Path>, strict: bool) -> Result<Graph> {
    let mut graph = Graph::default();
    let mut seen = BTreeSet::new();
    let mut queued = BTreeSet::from([main.path.clone()]);
    let mut shadow_checked = BTreeSet::new();
    let mut pending = vec![(main.clone(), Vec::<PathBuf>::new())];
    let managed = runtime_libraries(runtime)?;
    let app = util::parent(&main.path)?;
    while let Some((report, inherited)) = pending.pop() {
        if !seen.insert(report.path.clone()) {
            continue;
        }
        if seen.len() > 1024 {
            return Err(Error::new(
                "ResourceLimit",
                "dependency graph exceeds 1024 modules",
            ));
        }
        graph.needs_runtime |= report.needs_runtime();
        if report.target != main.target && strict {
            return Err(Error::new(
                "HostIncompatible",
                format!("{} has a different architecture", report.path.display()),
            ));
        }
        let os = report.target.os.as_str();
        if strict {
            minimum_glibc(&report)?;
        }
        let mut before = vec![];
        let mut after = vec![];
        let mut next_inherited = inherited.clone();
        if os == "windows" {
            before.push(app.to_owned());
            if let Some(system) = std::env::var_os("SystemRoot") {
                before.push(PathBuf::from(&system).join("System32"));
                before.push(PathBuf::from(system));
            }
            before.push(std::env::current_dir()?);
            after.extend(environment_paths("PATH"));
        } else if os == "linux" {
            if report.runpaths.is_empty() {
                for s in &report.rpaths {
                    before.push(expand(s, &report.path, &main.path)?);
                }
                next_inherited.extend(before.clone());
            }
            before.extend(inherited.clone());
            after.extend(environment_paths("LD_LIBRARY_PATH"));
            for s in &report.runpaths {
                after.push(expand(s, &report.path, &main.path)?);
            }
            after.extend(
                [
                    "/lib/x86_64-linux-gnu",
                    "/usr/lib/x86_64-linux-gnu",
                    "/lib64",
                    "/usr/lib64",
                    "/lib",
                    "/usr/lib",
                ]
                .map(PathBuf::from),
            );
        } else {
            after.extend(environment_paths("DYLD_LIBRARY_PATH"));
            for s in &report.rpaths {
                after.push(expand(s, &report.path, &main.path)?);
            }
            after.extend(inherited.clone());
            next_inherited = after.clone();
        }
        // Check all selected SDK library names against locations that precede the
        // injected directory, including inherited old-style ELF RPATHs.
        for dir in &before {
            let resolved = dir.canonicalize().unwrap_or_else(|_| dir.clone());
            if !shadow_checked.insert(resolved.clone())
                || runtime.is_some_and(|r| r.canonicalize().is_ok_and(|r| r == resolved))
            {
                continue;
            }
            for (name, selected) in &managed {
                let local = dir.join(name);
                if local.is_file()
                    && let Err(e) = identical(&local, selected)
                {
                    if strict {
                        return Err(e);
                    }
                    graph.diagnostics.push(e.to_string());
                }
            }
        }
        for d in &report.dependencies {
            let name = basename(&d.name);
            if system(&d.name, os) {
                graph.edges.push(Edge {
                    from: report.path.clone(),
                    library: d.name.clone(),
                    resolved: None,
                    status: "system loader".into(),
                });
                continue;
            }
            let selected = managed.get(&name.to_ascii_lowercase());
            let explicit = if os == "macos" {
                if d.name.starts_with("@rpath/") {
                    None
                } else if d.name.starts_with('@') {
                    Some(expand(&d.name, &report.path, &main.path)?)
                } else if d.name.contains('/') {
                    Some(PathBuf::from(&d.name))
                } else {
                    None
                }
            } else if d.name.contains(['/', '\\']) {
                Some(expand(&d.name, &report.path, &main.path)?)
            } else {
                None
            };
            let mut candidates = vec![];
            if let Some(p) = explicit {
                if let Some(s) = selected
                    && p.is_file()
                    && let Err(e) = identical(&p, s)
                {
                    if strict {
                        return Err(e);
                    }
                    graph.diagnostics.push(e.to_string());
                }
                candidates.push(p);
            } else {
                candidates.extend(before.iter().map(|p| p.join(name)));
                if let Some(rt) = runtime {
                    candidates.push(rt.join(name));
                }
                candidates.extend(after.iter().map(|p| p.join(name)));
            }
            let found = candidates.into_iter().find(|p| p.is_file());
            if let Some(p) = found {
                let path = p.canonicalize()?;
                if let Some(s) = selected
                    && let Err(e) = identical(&path, s)
                {
                    if strict {
                        return Err(e);
                    }
                    graph.diagnostics.push(e.to_string());
                }
                graph.edges.push(Edge {
                    from: report.path.clone(),
                    library: d.name.clone(),
                    resolved: Some(path.clone()),
                    status: "resolved".into(),
                });
                if os == "linux" && system_path(&path) {
                    continue;
                }
                if !queued.insert(path.clone()) {
                    continue;
                }
                match inspect::inspect(&path) {
                    Ok(dep) => pending.push((dep, next_inherited.clone())),
                    Err(e) => {
                        if strict {
                            return Err(e);
                        }
                        graph.diagnostics.push(e.to_string());
                    }
                }
            } else {
                graph.edges.push(Edge {
                    from: report.path.clone(),
                    library: d.name.clone(),
                    resolved: None,
                    status: if d.weak {
                        "weak, unavailable"
                    } else {
                        "unresolved"
                    }
                    .into(),
                });
                if strict && !d.weak {
                    return Err(Error::new(
                        "DependencyMissing",
                        format!("{} requires {}", report.path.display(), d.name),
                    ));
                }
            }
        }
        graph.modules.push(report);
    }
    Ok(graph)
}
pub fn plan(
    main: &InspectReport,
    ready: Option<&Ready>,
    args: Vec<OsString>,
) -> Result<LaunchPlan> {
    if main.target != crate::model::Target::host() {
        return Err(Error::new(
            "HostIncompatible",
            format!(
                "target {}/{} cannot execute natively on this host",
                main.target.os, main.target.arch
            ),
        ));
    }
    if !main.executable {
        return Err(Error::new(
            "HostIncompatible",
            "target is not an executable",
        ));
    }
    let mut environment = vec![];
    if let Some(ready) = ready {
        for key in [
            "LD_PRELOAD",
            "LD_AUDIT",
            "DYLD_INSERT_LIBRARIES",
            "DYLD_FORCE_FLAT_NAMESPACE",
        ] {
            if std::env::var_os(key).is_some_and(|v| !v.is_empty()) {
                return Err(Error::new(
                    "LoaderPolicyDenied",
                    format!("{key} prevents a verified runtime loading plan"),
                ));
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if std::fs::metadata(&main.path)?.mode() & 0o6000 != 0 {
                return Err(Error::new(
                    "LoaderPolicyDenied",
                    "setuid/setgid programs cannot use an injected runtime path",
                ));
            }
        }
        #[cfg(target_os = "linux")]
        {
            let file = std::fs::File::open(&main.path)?;
            let capabilities = match rustix::fs::fgetxattr_query(&file, "security.capability") {
                Ok(size) => size > 0,
                Err(rustix::io::Errno::NODATA | rustix::io::Errno::OPNOTSUPP) => false,
                Err(error) => return Err(Error::at(&main.path, error.into())),
            };
            if capabilities
                || rustix::process::getuid() != rustix::process::geteuid()
                || rustix::process::getgid() != rustix::process::getegid()
            {
                return Err(Error::new(
                    "LoaderPolicyDenied",
                    "privileged execution cannot use an injected runtime path",
                ));
            }
        }

        if main.hardened {
            return Err(Error::new(
                "LoaderPolicyDenied",
                "hardened Mach-O requires entitlement/library-validation analysis; runtime injection is not supported for this binary",
            ));
        }
        let key = match main.target.os.as_str() {
            "windows" => "PATH",
            "linux" => "LD_LIBRARY_PATH",
            _ => "DYLD_LIBRARY_PATH",
        };
        let mut paths = vec![ready.runtime.clone()];
        paths.extend(environment_paths(key));
        let value =
            std::env::join_paths(paths).map_err(|e| Error::new("InvalidPath", e.to_string()))?;
        environment.push((key.to_owned(), value));
    }
    minimum_os(main)?;
    let graph = graph(main, ready.map(|r| r.runtime.as_path()), true)?;
    let checked = graph
        .modules
        .into_iter()
        .map(|r| (r.path, r.sha256))
        .collect();
    Ok(LaunchPlan {
        binary: main.path.clone(),
        args,
        environment,
        checked,
    })
}
pub fn recheck(plan: &LaunchPlan) -> Result<()> {
    for (path, digest) in &plan.checked {
        if util::hash_file(path)? != *digest {
            return Err(Error::new("BinaryChanged", path.display().to_string()));
        }
    }
    Ok(())
}
fn minimum_os(_report: &InspectReport) -> Result<()> {
    #[cfg(target_os = "macos")]
    if let Some(required) = &_report.minimum_os {
        let output = std::process::Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()?;
        if !output.status.success() {
            return Err(Error::new(
                "HostIncompatible",
                "could not determine macOS version",
            ));
        }
        fn version(v: &str) -> Vec<u32> {
            let mut x = v
                .trim()
                .split('.')
                .map(|s| s.parse().unwrap_or(0))
                .collect::<Vec<_>>();
            x.resize(3, 0);
            x
        }
        if version(&String::from_utf8_lossy(&output.stdout)) < version(required) {
            return Err(Error::new(
                "HostIncompatible",
                format!("binary requires macOS {required}"),
            ));
        }
    }
    Ok(())
}
fn minimum_glibc(_report: &InspectReport) -> Result<()> {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        let Some(host) = crate::platform::glibc_version() else {
            return Ok(());
        };
        fn version(s: &str) -> Vec<u32> {
            let mut parts = s
                .split('.')
                .map(|s| s.parse().unwrap_or(0))
                .collect::<Vec<_>>();
            parts.resize(3, 0);
            parts
        }
        for versions in _report.symbol_versions.values() {
            for required in versions {
                if let Some(v) = required.strip_prefix("GLIBC_")
                    && v.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                    && version(v) > version(&host)
                {
                    return Err(Error::new(
                        "HostIncompatible",
                        format!(
                            "{} requires GLIBC_{v}; host glibc is {host}",
                            _report.path.display()
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}
