use super::Store;
use crate::{Error, Result, util};
use serde::Serialize;
use std::path::PathBuf;
use std::{fs, path::Path};
impl Store {
    pub fn clean(&self, all: bool, dry_run: bool) -> Result<Vec<CleanEntry>> {
        if !self.root.exists() {
            return Ok(vec![]);
        }
        self.check_owner()?;
        // A preview is read-only: do not initialize store directories or lock files.
        let _guard = if dry_run { None } else { Some(self.guard()?) };
        let mut out = vec![];
        for name in ["downloads", "metadata", "staging", "runtimes"] {
            if name == "runtimes" && !all {
                continue;
            }
            let dir = self.root.join(name);
            if !dir.exists() {
                continue;
            }
            if !util::plain_dir(&dir) {
                out.push(CleanEntry {
                    path: dir,
                    status: CleanStatus::Skipped("linked directory".into()),
                });
                continue;
            }
            for p in util::children(&dir)? {
                let leaf = p.file_name().unwrap_or_default().to_string_lossy();
                if name == "runtimes" {
                    if !util::is_digest(&leaf) {
                        out.push(CleanEntry {
                            path: p,
                            status: CleanStatus::Skipped("unknown object".into()),
                        });
                        continue;
                    }
                    let state = self.usage(&leaf);
                    if state != super::UsageState::Unused {
                        out.push(CleanEntry {
                            path: p,
                            status: CleanStatus::Skipped(state.to_string()),
                        });
                        continue;
                    }
                }
                let status = if dry_run {
                    CleanStatus::WouldRemove
                } else {
                    match remove_owned(&dir, &p) {
                        Ok(()) => CleanStatus::Removed,
                        Err(e) => CleanStatus::Failed(e.to_string()),
                    }
                };
                if name == "runtimes" && !dry_run && matches!(status, CleanStatus::Removed) {
                    let u = self.root.join("usage").join(leaf.as_ref());
                    if u.exists() {
                        remove_owned(&self.root.join("usage"), &u)?;
                    }
                }
                out.push(CleanEntry { path: p, status });
            }
        }
        Ok(out)
    }
}
fn remove_owned(parent: &Path, path: &Path) -> Result<()> {
    // Only direct children, never recursively follow a reparse point/symlink.
    if path.parent() != Some(parent) || !util::plain_dir(parent) {
        return Err(Error::new("UnsafeCache", "delete outside owned directory"));
    }
    let m = fs::symlink_metadata(path)?;
    if util::is_link(&m) {
        #[cfg(windows)]
        {
            if m.is_dir() {
                fs::remove_dir(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
        #[cfg(not(windows))]
        fs::remove_file(path)?;
    } else if m.is_dir() {
        // std handles recursive deletion without following symlinks or reparse points.
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CleanEntry {
    pub path: PathBuf,
    pub status: CleanStatus,
}
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "reason", rename_all = "snake_case")]
pub enum CleanStatus {
    Removed,
    WouldRemove,
    Skipped(String),
    Failed(String),
}
impl CleanStatus {
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}
impl std::fmt::Display for CleanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Removed => f.write_str("removed"),
            Self::WouldRemove => f.write_str("would remove"),
            Self::Skipped(s) => write!(f, "skipped: {s}"),
            Self::Failed(s) => write!(f, "failed: {s}"),
        }
    }
}
