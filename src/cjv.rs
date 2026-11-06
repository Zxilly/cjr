use crate::{Error, Result, config::user_home, lock::Lock, model::Artifact, util};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

fn settings_home(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = util::read_limited(path, 1024 * 1024)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| Error::new("InvalidCjvConfig", e.to_string()))?;
    let v: toml::Value =
        toml::from_str(text).map_err(|e| Error::new("InvalidCjvConfig", e.to_string()))?;
    if v.get("version").and_then(|v| v.as_integer()).unwrap_or(1) > 1 {
        return Err(Error::new(
            "InvalidCjvConfig",
            "unsupported settings version",
        ));
    }
    match v.get("home") {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_owned()))
            .ok_or_else(|| Error::new("InvalidCjvConfig", "home must be a string")),
    }
}
pub fn discover_home() -> Result<PathBuf> {
    if let Some(home) =
        std::env::var_os("CJV_HOME").filter(|s| !s.to_string_lossy().trim().is_empty())
    {
        let home = PathBuf::from(home.to_string_lossy().trim());
        if !home.is_absolute() {
            return Err(Error::new("InvalidCjvConfig", "CJV_HOME must be absolute"));
        }
        return Ok(home);
    }
    let user = user_home()?;
    let settings = std::env::var_os("CJV_CONFIG_FILE")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or(user.join(".cjv/settings.toml"));
    let fallback = std::env::var_os("CJV_FALLBACK_SETTINGS")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(windows) {
                PathBuf::from(std::env::var_os("ProgramData").unwrap_or_default())
                    .join("cjv/settings.toml")
            } else {
                PathBuf::from("/etc/cjv/settings.toml")
            }
        });
    let home = settings_home(&settings)?.or_else(|| settings_home(&fallback).ok().flatten());
    let home = home
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| p.is_absolute())
        .unwrap_or(user.join(".cjv"));
    Ok(std::path::absolute(home)?)
}
#[derive(Debug, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub size: u64,
}
pub fn cache_entries(home: &Path) -> Result<Vec<(CacheEntry, PathBuf)>> {
    let mut entries = vec![];
    for path in util::children(&home.join("cache"))? {
        if path.extension().is_none_or(|s| s != "json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !util::is_digest(stem) {
            continue;
        }
        let Ok(mut e) = util::read_json::<CacheEntry>(&path) else {
            continue;
        };
        if e.key != stem || !matches!(e.kind.as_str(), "artifact" | "metadata") {
            continue;
        }
        e.sha256.make_ascii_lowercase();
        let data = path.with_extension("data");
        if data.is_file() {
            entries.push((e, data));
        }
    }
    Ok(entries)
}
pub fn archives(home: &Path, a: &Artifact) -> Result<Vec<PathBuf>> {
    let mut out = cache_entries(home)?
        .into_iter()
        .filter(|(e, _)| e.kind == "artifact" && e.sha256 == a.sha256)
        .map(|(_, p)| p)
        .collect::<Vec<_>>();
    let completed = home.join("downloads").join(&a.sha256);
    if completed.is_file() {
        out.push(completed);
    }
    Ok(out)
}
#[derive(Debug, Clone, Deserialize)]
pub struct Installation {
    pub release: String,
    pub tuple: String,
    pub sha256: String,
}
impl Installation {
    pub fn matches(&self, a: &Artifact) -> bool {
        // Exact release; do not interpret a directory named lts/sts as the release.
        let base = format!("{}-{}", a.channel, a.version);
        self.tuple == a.platform
            && self.sha256.eq_ignore_ascii_case(&a.sha256)
            && (self.release == base || self.release == format!("{base}-{}", a.platform))
    }
}
pub fn installations(home: &Path) -> Result<Vec<(Installation, PathBuf)>> {
    let dir = home.join("toolchains");
    if !util::plain_dir(&dir) {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for p in util::children(&dir)? {
        let n = p.file_name().unwrap_or_default().to_string_lossy();
        if scratch(&n) || !util::plain_dir(&p) {
            continue;
        }
        let path = p.join(".cjv/toolchain.toml");
        let Ok(raw) = util::read_limited(&path, 65536) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&raw) else {
            continue;
        };
        let Ok(i) = toml::from_str::<Installation>(text) else {
            continue;
        };
        if util::is_digest(&i.sha256) {
            out.push((i, p));
        }
    }
    Ok(out)
}
fn scratch(n: &str) -> bool {
    n.ends_with(".staging") || n.ends_with(".old") || n.starts_with(".fstx-")
}
pub struct StableRead {
    _home: Lock,
    _lookup: Lock,
}
pub fn stable_read(home: &Path) -> Result<StableRead> {
    // Only known, migrated layouts. No creation, recovery, marker mutation or migration.
    if !home.join(".cjv/layout-v2").is_file() {
        return Err(Error::new(
            "CjvUnavailable",
            "unknown cjv installation layout",
        ));
    }
    let first = Lock::acquire(&home.join(".lifecycle.lock"), Duration::from_secs(2), false)?;
    let second = Lock::acquire(
        &home.join(".lifecycle.lookup.lock"),
        Duration::from_secs(2),
        false,
    )?;
    for base in [home.to_owned(), home.join("toolchains")] {
        for p in util::children(&base)? {
            if scratch(&p.file_name().unwrap_or_default().to_string_lossy()) {
                return Err(Error::new(
                    "CjvUnavailable",
                    "unfinished cjv transaction; installed SDK snapshot skipped",
                ));
            }
        }
    }
    Ok(StableRead {
        _home: first,
        _lookup: second,
    })
}
pub fn snapshot(source: &Path, dest: &Path) -> Result<()> {
    if !util::plain_dir(source) {
        return Err(Error::new(
            "CjvUnavailable",
            "SDK directory is a link or is missing",
        ));
    }
    fs::create_dir_all(dest)?;
    let mut total = 0u64;
    for (index, entry) in walkdir::WalkDir::new(source)
        .follow_links(false)
        .follow_root_links(false)
        .min_depth(1)
        .max_open(16)
        .into_iter()
        .enumerate()
    {
        let entry = entry.map_err(|e| {
            Error::at(
                e.path().unwrap_or(source),
                std::io::Error::other(e.to_string()),
            )
        })?;
        let p = entry.path();
        let m = fs::symlink_metadata(p)?;
        total = total.saturating_add(m.len());
        if index >= 100_000 || total > 4 * 1024 * 1024 * 1024 {
            return Err(Error::new(
                "ResourceLimit",
                "managed runtime snapshot exceeds limits",
            ));
        }
        let relative = p
            .strip_prefix(source)
            .map_err(|_| Error::new("UnsafeCache", "snapshot escaped source"))?;
        let out = dest.join(relative);
        if util::is_link(&m) {
            let target = fs::read_link(p)?;
            crate::provision::archive::safe_link(&target)?;
            crate::provision::archive::symlink(&target, &out)?;
        } else if m.is_dir() {
            fs::create_dir(&out)?;
        } else if m.is_file() {
            fs::copy(p, &out)?;
            util::set_mode(&out, util::mode(&m))?;
        } else {
            return Err(Error::new("CjvUnavailable", "non-regular runtime file"));
        }
    }
    Ok(())
}
