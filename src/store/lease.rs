//! Persistent execution leases: uncertainty retains a runtime rather than deleting it.
use super::{Store, StoreTransaction};
use crate::{Error, Result, process::BootIdentity, util};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageState {
    Unused,
    ActiveOrRetained,
    Unknown,
    OtherHost,
}
impl std::fmt::Display for UsageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unused => "unused",
            Self::ActiveOrRetained => "active or retained session",
            Self::Unknown => "unknown",
            Self::OtherHost => "unknown (another host)",
        })
    }
}
#[derive(Serialize, Deserialize)]
struct Usage {
    schema_version: u32,
    boot: Option<BootIdentity>,
    pid: u32,
}

impl Store {
    pub fn usage(&self, id: &str) -> UsageState {
        if !util::is_digest(id) {
            return UsageState::Unknown;
        }
        let dir = self.root.join("usage").join(id);
        if fs::symlink_metadata(&dir).is_ok() && !util::plain_dir(&dir) {
            return UsageState::Unknown;
        }
        let boot = crate::process::boot_identity();
        let Ok(entries) = util::children(&dir) else {
            return UsageState::Unknown;
        };
        for path in entries {
            let Ok(record) = util::read_json::<Usage>(&path) else {
                return UsageState::Unknown;
            };
            let (Some(now), Some(before)) = (&boot, &record.boot) else {
                return UsageState::Unknown;
            };
            if record.schema_version != 1 {
                return UsageState::Unknown;
            }
            if now.host != before.host {
                return UsageState::OtherHost;
            }
            if now.session == before.session {
                return UsageState::ActiveOrRetained;
            }
        }
        UsageState::Unused
    }
}
impl StoreTransaction<'_> {
    pub fn begin_use(&self, id: &str) -> Result<Lease> {
        if !util::is_digest(id) || !util::plain_dir(&self.root().join("runtimes").join(id)) {
            return Err(Error::new(
                "UnsafeCache",
                "lease requires a published runtime",
            ));
        }
        let dir = self.root().join("usage").join(id);
        if fs::symlink_metadata(&dir).is_ok() && !util::plain_dir(&dir) {
            return Err(Error::new("UnsafeCache", "linked runtime usage directory"));
        }
        fs::create_dir_all(&dir)?;
        let mut file = tempfile::Builder::new()
            .prefix("session-")
            .suffix(".json")
            .tempfile_in(&dir)?;
        let marker = Usage {
            schema_version: 1,
            boot: crate::process::boot_identity(),
            pid: std::process::id(),
        };
        serde_json::to_writer(file.as_file_mut(), &marker)?;
        file.as_file().sync_all()?;
        let (_, path) = file.keep().map_err(|e| e.error)?;
        Ok(Lease {
            path: Some(path),
            started: false,
        })
    }
}

/// Before spawning, dropping cancels registration. After a spawn attempt,
/// dropping conservatively retains the record until successful termination proof.
#[must_use]
pub struct Lease {
    path: Option<PathBuf>,
    started: bool,
}
impl Lease {
    pub(crate) fn mark_started(&mut self) {
        self.started = true;
    }
    pub fn finish(mut self, descendants_gone: bool) -> Result<()> {
        self.started = true;
        if descendants_gone && let Some(path) = self.path.take() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if !self.started
            && let Some(path) = self.path.take()
        {
            let _ = fs::remove_file(path);
        }
    }
}
