//! Runtime store boundary: queries are read-only; mutations require a transaction.
mod clean;
mod inventory;
mod lease;
pub use clean::{CleanEntry, CleanStatus};
pub use inventory::{inventory, tree_id, verify};
pub use lease::{Lease, UsageState};

use crate::{Error, Result, config::Config, lock::Lock, model::Receipt, util};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
pub const RECIPE: u32 = 1;
pub struct Store {
    pub root: PathBuf,
}
pub struct Ready {
    pub root: PathBuf,
    pub runtime: PathBuf,
    pub receipt: Receipt,
}
impl Store {
    pub fn new(config: &Config) -> Self {
        Self {
            root: config.home.clone(),
        }
    }
    pub fn guard(&self) -> Result<StoreTransaction<'_>> {
        fs::create_dir_all(&self.root)?;
        if !util::plain_dir(&self.root) {
            return Err(Error::new(
                "UnsafeCache",
                "CJR_HOME must not be a symlink or junction",
            ));
        }
        // Own the namespace explicitly before treating `downloads` etc. as cache.
        // In particular CJR_HOME pointing at a user's home must not claim Downloads.
        let guard = Lock::acquire(
            &self.root.join(".cjr-store.lock"),
            Duration::from_secs(1800),
            true,
        )?;
        let marker = self.root.join(".cjr-store.json");
        if !marker.exists() {
            if util::children(&self.root)?
                .iter()
                .any(|p| p.file_name().is_none_or(|n| n != ".cjr-store.lock"))
            {
                return Err(Error::new(
                    "UnsafeCache",
                    "CJR_HOME is nonempty and has no cjr ownership marker; use an empty dedicated directory",
                ));
            }
            util::atomic_json(
                &marker,
                &serde_json::json!({"schema_version":1,"kind":"cjr-runtime-store"}),
            )?;
        }
        self.check_owner()?;
        for name in [
            "locks",
            "metadata",
            "downloads",
            "runtimes",
            "staging",
            "usage",
        ] {
            let p = self.root.join(name);
            if p.exists() && !util::plain_dir(&p) {
                return Err(Error::new(
                    "UnsafeCache",
                    format!("{} is not an owned directory", p.display()),
                ));
            }
            fs::create_dir_all(p)?;
        }
        Ok(StoreTransaction {
            store: self,
            _lock: guard,
        })
    }
    fn check_owner(&self) -> Result<()> {
        let path = self.root.join(".cjr-store.json");
        let v: serde_json::Value = util::read_json(&path)?;
        if v["schema_version"] != 1 || v["kind"] != "cjr-runtime-store" {
            return Err(Error::new("UnsafeCache", "invalid cjr ownership marker"));
        }
        Ok(())
    }
    pub fn receipts(&self, warnings: &mut Vec<String>) -> Vec<(PathBuf, Receipt)> {
        let mut out = vec![];
        let entries = match util::children(&self.root.join("runtimes")) {
            Ok(v) => v,
            Err(e) => {
                warnings.push(e.to_string());
                return out;
            }
        };
        for p in entries {
            if !util::plain_dir(&p) {
                warnings.push(format!("skipped linked/unknown runtime {}", p.display()));
                continue;
            }
            match util::read_json::<Receipt>(&p.join("receipt.json")) {
                Ok(r)
                    if r.schema_version == 1
                        && r.recipe == RECIPE
                        && util::is_digest(&r.bundle_id)
                        && p.file_name().is_some_and(|s| s == r.bundle_id.as_str()) =>
                {
                    out.push((p, r))
                }
                Ok(_) => warnings.push(format!("invalid runtime receipt: {}", p.display())),
                Err(e) => warnings.push(e.to_string()),
            }
        }
        out
    }
}
/// Capability proving exclusive ownership of the store's mutation lock.
/// There is intentionally no unlocked publication or lease-registration API.
#[must_use]
pub struct StoreTransaction<'a> {
    store: &'a Store,
    _lock: Lock,
}
impl StoreTransaction<'_> {
    pub(crate) fn root(&self) -> &Path {
        &self.store.root
    }
}

use crate::model::Artifact;

impl StoreTransaction<'_> {
    pub fn find(&self, a: &Artifact) -> Result<Option<Ready>> {
        let mut warnings = vec![];
        for (root, r) in self.store.receipts(&mut warnings) {
            if r.artifact.sha256 == a.sha256
                && r.artifact.platform == a.platform
                && r.artifact.version == a.version
            {
                verify(&root, &r)?;
                let runtime = root.join("payload").join(util::relative(&r.runtime_dir)?);
                return Ok(Some(Ready {
                    root,
                    runtime,
                    receipt: r,
                }));
            }
        }
        Ok(None)
    }
    pub fn publish(
        &self,
        payload: &Path,
        a: &Artifact,
        source: &Path,
        provenance: &str,
    ) -> Result<Ready> {
        a.validate()?;
        // Publication consumes a payload. It may only consume private staging data,
        // never a caller's SDK directory, another bundle, or a linked directory.
        let staging_root = self.store.root.join("staging").canonicalize()?;
        let canonical = payload.canonicalize()?;
        if canonical == staging_root || !canonical.starts_with(&staging_root) {
            return Err(Error::new(
                "UnsafeCache",
                "publication payload must be inside this store's staging directory",
            ));
        }
        let mut ancestor = payload;
        while ancestor != self.store.root.join("staging") {
            if !util::plain_dir(ancestor) {
                return Err(Error::new("UnsafeCache", "linked publication path"));
            }
            ancestor = util::parent(ancestor)?;
        }
        let runtime = crate::provision::archive::layout(payload, &a.platform)?;
        let runtime_dir = runtime
            .strip_prefix(payload)
            .map_err(|_| Error::new("UnsafeCache", "runtime path escapes payload"))?
            .to_string_lossy()
            .replace('\\', "/");
        let files = inventory(payload)?;
        let id = tree_id(a, &files)?;
        let receipt = Receipt {
            schema_version: 1,
            recipe: RECIPE,
            bundle_id: id.clone(),
            artifact: a.clone(),
            provenance: provenance.into(),
            source: source.to_owned(),
            runtime_dir: runtime_dir.clone(),
            files,
        };
        let dest = self.store.root.join("runtimes").join(&id);
        if dest.exists() {
            let existing: Receipt = util::read_json(&dest.join("receipt.json"))?;
            verify(&dest, &existing)?;
            if existing.bundle_id != id || existing.runtime_dir != runtime_dir {
                return Err(Error::new(
                    "CorruptCache",
                    "published bundle identity mismatch",
                ));
            }
            return Ok(Ready {
                runtime: dest.join("payload").join(runtime_dir),
                root: dest,
                receipt: existing,
            });
        }
        let staging = tempfile::Builder::new()
            .prefix("publish-")
            .tempdir_in(self.store.root.join("staging"))?;
        fs::rename(payload, staging.path().join("payload"))?;
        util::atomic_json(&staging.path().join("receipt.json"), &receipt)?;
        fs::rename(staging.path(), &dest)?;
        Ok(Ready {
            runtime: dest.join("payload").join(runtime_dir),
            root: dest,
            receipt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clean_scope() {
        let t = tempfile::tempdir().unwrap();
        let s = Store {
            root: t.path().join("cjr"),
        };
        let _g = s.guard().unwrap();
        fs::write(s.root.join("downloads/a"), "x").unwrap();
        fs::create_dir(s.root.join("runtimes").join("a".repeat(64))).unwrap();
        drop(_g);
        assert_eq!(s.clean(false, true).unwrap().len(), 1);
        assert!(s.root.join("downloads/a").exists());
        s.clean(false, false).unwrap();
        assert!(!s.root.join("downloads/a").exists());
        assert!(s.root.join("runtimes").join("a".repeat(64)).exists());
    }
}
