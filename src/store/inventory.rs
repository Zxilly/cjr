use super::RECIPE;
use crate::{
    Error, Result,
    model::{Artifact, FileEntry, Receipt},
    util,
};
use std::{fs, path::Path};
pub fn tree_id(a: &Artifact, files: &[FileEntry]) -> Result<String> {
    Ok(util::digest(&serde_json::to_vec(&(
        RECIPE,
        &a.sha256,
        &a.platform,
        files,
    ))?))
}
pub fn inventory(root: &Path) -> Result<Vec<FileEntry>> {
    let mut files = vec![];
    if !util::plain_dir(root) {
        return Err(Error::new(
            "UnsafeCache",
            "inventory requires a plain directory",
        ));
    }
    let canonical = root.canonicalize()?;
    let mut entries = 0usize;
    let mut total_bytes = 0u64;
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .follow_root_links(false)
        .min_depth(1)
        .max_open(16)
    {
        let entry = entry.map_err(|e| {
            Error::at(
                e.path().unwrap_or(root),
                std::io::Error::other(e.to_string()),
            )
        })?;
        let p = entry.path();
        entries += 1;
        if entries > 100_000 {
            return Err(Error::new(
                "ResourceLimit",
                "runtime contains too many files",
            ));
        }
        let m = fs::symlink_metadata(p)?;
        let path = p
            .strip_prefix(root)
            .map_err(|_| Error::new("UnsafeCache", "file escaped runtime root"))?
            .to_str()
            .ok_or_else(|| Error::new("UnsafeCache", "non-UTF8 runtime path"))?
            .replace('\\', "/");
        util::relative(&path)?;
        if util::is_link(&m) {
            let target = fs::read_link(p)?;
            crate::provision::archive::safe_link(&target)?;
            if !p.canonicalize()?.starts_with(&canonical) || !p.is_file() {
                return Err(Error::new("UnsafeCache", "runtime link escapes bundle"));
            }
            files.push(FileEntry {
                path,
                sha256: None,
                link: Some(target.to_string_lossy().into_owned()),
                size: 0,
                mode: 0,
            });
        } else if m.is_dir() {
            continue;
        } else if m.is_file() {
            total_bytes = total_bytes.saturating_add(m.len());
            if total_bytes > 4 * 1024 * 1024 * 1024 {
                return Err(Error::new("ResourceLimit", "runtime exceeds 4 GiB"));
            }
            files.push(FileEntry {
                path,
                sha256: Some(util::hash_file(p)?),
                link: None,
                size: m.len(),
                mode: util::mode(&m),
            });
        } else {
            return Err(Error::new("UnsafeCache", "special runtime file"));
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
pub fn verify(root: &Path, r: &Receipt) -> Result<()> {
    if r.schema_version != 1
        || r.recipe != RECIPE
        || !util::plain_dir(root)
        || !util::plain_dir(&root.join("payload"))
    {
        return Err(Error::new("CorruptCache", "invalid runtime root"));
    }
    r.artifact.validate()?;
    util::relative(&r.runtime_dir)?;
    let actual = inventory(&root.join("payload"))?;
    if tree_id(&r.artifact, &actual)? != r.bundle_id
        || tree_id(&r.artifact, &r.files)? != r.bundle_id
    {
        return Err(Error::new(
            "CorruptCache",
            format!(
                "{} failed integrity verification; clean this inactive cache with cjr cache clean --all",
                root.display()
            ),
        ));
    }
    Ok(())
}
