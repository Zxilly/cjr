pub mod archive;
mod download;
mod validation;
use crate::{
    Error, Result,
    config::Config,
    model::Artifact,
    store::{Ready, StoreTransaction},
    util,
};
use download::download;
use std::{fs, path::Path};
use validation::{installed_layout, validate_runtime};

pub fn ensure(
    config: &Config,
    store: &StoreTransaction<'_>,
    a: &Artifact,
    offline: bool,
    verbose: bool,
) -> Result<Ready> {
    a.validate()?;
    if let Some(ready) = store.find(a)? {
        if verbose {
            crate::diagnostics::emit(format_args!(
                "cjr: using verified runtime {}",
                ready.runtime.display()
            ));
        }
        return Ok(ready);
    }
    let stage = tempfile::Builder::new()
        .prefix("prepare-")
        .tempdir_in(store.root().join("staging"))?;
    if let Some(home) = &config.cjv_home {
        match crate::cjv::archives(home, a) {
            Ok(paths) => {
                for p in paths {
                    let private = stage.path().join("archive");
                    match copy_verified(&p, &private, &a.sha256) {
                        Ok(()) => {
                            if verbose {
                                crate::diagnostics::emit(format_args!(
                                    "cjr: reusing cjv archive {}",
                                    p.display()
                                ));
                            }
                            return from_archive(store, a, &private, &p, stage.path());
                        }
                        Err(e) => {
                            if verbose {
                                crate::diagnostics::emit(format_args!(
                                    "cjr: skipping cjv archive: {e}"
                                ));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if verbose {
                    crate::diagnostics::emit(format_args!("cjr: cjv archives unavailable: {e}"));
                }
            }
        }
        // A bounded, read-only lock adapter. Never recover an interrupted cjv install.
        let installed = (|| -> Result<Option<Ready>> {
            let _stable = crate::cjv::stable_read(home)?;
            for (i, source) in crate::cjv::installations(home)? {
                if !i.matches(a) {
                    continue;
                }
                let payload = stage.path().join("installed");
                fs::create_dir_all(payload.join("cangjie/runtime/lib"))?;
                let source_layout = installed_layout(&source, &a.platform)?;
                let dest = payload
                    .join("cangjie/runtime/lib")
                    .join(util::file_name(&source_layout)?);
                crate::cjv::snapshot(&source_layout, &dest)?;
                for p in util::children(&source)? {
                    let name = util::file_name(&p)?.to_string_lossy().to_ascii_lowercase();
                    if p.is_file()
                        && (name.contains("license")
                            || name.contains("notice")
                            || name.contains("copyright"))
                    {
                        fs::copy(&p, payload.join("cangjie").join(util::file_name(&p)?))?;
                    }
                }
                validate_runtime(&dest, a)?;
                if verbose {
                    crate::diagnostics::emit(format_args!(
                        "cjr: snapshotting cjv installation {}",
                        source.display()
                    ));
                }
                return Ok(Some(store.publish(&payload, a, &source, "cjv-managed")?));
            }
            Ok(None)
        })();
        match installed {
            Ok(Some(r)) => return Ok(r),
            Ok(None) => {}
            Err(e) => {
                if verbose {
                    crate::diagnostics::emit(format_args!("cjr: skipping installed SDK: {e}"));
                }
            }
        }
    }
    let own = store.root().join("downloads").join(&a.sha256);
    if own.is_file() {
        match copy_verified(&own, &stage.path().join("archive"), &a.sha256) {
            Ok(()) => {
                return from_archive(store, a, &stage.path().join("archive"), &own, stage.path());
            }
            Err(e) => {
                if verbose {
                    crate::diagnostics::emit(format_args!("cjr: skipping damaged archive: {e}"));
                }
            }
        }
    }
    if offline {
        return Err(Error::new(
            "OfflineCacheMiss",
            format!("no local official runtime for {}/{}", a.version, a.platform),
        ));
    }
    crate::diagnostics::emit(format_args!(
        "cjr: downloading official SDK {} ({})",
        a.version, a.platform
    ));
    let archive = stage.path().join("download");
    download(a, &archive)?;
    // Publication is serialized with cleanup by the store guard held by the caller.
    if own.exists() {
        fs::remove_file(&own)?;
    }
    fs::rename(&archive, &own)?;
    from_archive(store, a, &own, &own, stage.path())
}
pub fn copy_verified(source: &Path, dest: &Path, digest: &str) -> Result<()> {
    fs::copy(source, dest).map_err(|e| Error::at(source, e))?;
    if util::hash_file(dest)? != digest {
        return Err(Error::new("ChecksumMismatch", source.display().to_string()));
    }
    Ok(())
}
fn from_archive(
    store: &StoreTransaction<'_>,
    a: &Artifact,
    archive: &Path,
    source: &Path,
    stage: &Path,
) -> Result<Ready> {
    let payload = stage.join("payload");
    archive::extract(archive, &a.name, &a.platform, &payload)?;
    let runtime = archive::layout(&payload, &a.platform)?;
    validate_runtime(&runtime, a)?;
    store.publish(&payload, a, source, "archive-verified")
}
