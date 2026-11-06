use super::archive;
use crate::{Error, Result, model::Artifact, util};
use std::path::Path;
pub(super) fn installed_layout(source: &Path, platform: &str) -> Result<std::path::PathBuf> {
    for name in archive::runtime_names(platform)? {
        for p in util::children(&source.join("runtime/lib"))? {
            if p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .eq_ignore_ascii_case(&name)
                && util::plain_dir(&p)
            {
                return Ok(p);
            }
        }
    }
    Err(Error::new("CjvUnavailable", "no runtime in managed SDK"))
}
pub(super) fn validate_runtime(runtime: &Path, a: &Artifact) -> Result<()> {
    let ext = if a.platform.starts_with("win32") {
        "dll"
    } else if a.platform.starts_with("darwin") {
        "dylib"
    } else {
        "so"
    };
    let core = runtime.join(format!("libcangjie-std-core.{ext}"));
    let runtime_lib = runtime.join(format!("libcangjie-runtime.{ext}"));
    let report = crate::inspect::inspect(&core)?;
    if report.sdk_version.as_deref() != Some(&a.version)
        || report.target.platform()? != a.platform
        || !runtime_lib.is_file()
    {
        return Err(Error::new(
            "InvalidRuntime",
            "SDK runtime version, architecture or required files do not match",
        ));
    }
    let rt = crate::inspect::inspect(&runtime_lib)?;
    if rt.target != report.target {
        return Err(Error::new(
            "InvalidRuntime",
            "runtime architecture mismatch",
        ));
    }
    Ok(())
}
