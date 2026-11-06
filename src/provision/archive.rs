use crate::{Error, Result, util};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_FILE: u64 = 1024 * 1024 * 1024;
const MAX_TOTAL: u64 = 4 * 1024 * 1024 * 1024;
pub fn runtime_names(platform: &str) -> Result<Vec<String>> {
    let stem = match platform {
        "win32-x64" => "windows_x86_64",
        "linux-x64" => "linux_x86_64",
        "darwin-arm64" => "darwin_aarch64",
        "darwin-x64" => "darwin_x86_64",
        _ => return Err(Error::new("HostIncompatible", platform)),
    };
    Ok(["cjnative", "llvm"]
        .iter()
        .map(|s| format!("{stem}_{s}"))
        .collect())
}
fn selected(name: &str, platform: &str) -> Result<Option<PathBuf>> {
    let name = name.trim_end_matches('/');
    let clean = name.strip_prefix("./").unwrap_or(name);
    // Validate even entries not extracted: no traversal, ADS or ambiguous names.
    let relative = util::relative(clean)?;
    let components: Vec<_> = clean.split('/').collect();
    if components.first() != Some(&"cangjie") {
        return Ok(None);
    }
    if components.len() == 2
        && (components[1].to_ascii_lowercase().contains("license")
            || components[1].to_ascii_lowercase().contains("notice")
            || components[1].to_ascii_lowercase().contains("copyright"))
    {
        return Ok(Some(relative));
    }
    if components.len() >= 4
        && components[1] == "runtime"
        && components[2] == "lib"
        && runtime_names(platform)?
            .iter()
            .any(|s| s.eq_ignore_ascii_case(components[3]))
    {
        return Ok(Some(relative));
    }
    Ok(None)
}
/// Runtime dylib aliases are local relative names, never an absolute or parent path.
pub fn safe_link(target: &Path) -> Result<()> {
    let s = target
        .to_str()
        .ok_or_else(|| Error::new("UnsafeArchive", "non-UTF8 link"))?;
    util::relative(s)?;
    Ok(())
}
pub fn symlink(target: &Path, path: &Path) -> Result<()> {
    safe_link(target)?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, path)?;
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, path)?;
    }
    Ok(())
}
struct Extraction<'a> {
    root: &'a Path,
    seen: BTreeSet<String>,
    links: Vec<(PathBuf, PathBuf)>,
    total: u64,
    count: usize,
}
impl<'a> Extraction<'a> {
    fn entry(
        &mut self,
        path: PathBuf,
        size: u64,
        mode: u32,
        link: Option<PathBuf>,
        reader: &mut dyn Read,
    ) -> Result<()> {
        self.count += 1;
        if self.count > 100_000
            || size > MAX_FILE
            || self.total.checked_add(size).is_none_or(|s| s > MAX_TOTAL)
        {
            return Err(Error::new(
                "ResourceLimit",
                "runtime archive extraction limit exceeded",
            ));
        }
        self.total += size;
        let key = path.to_string_lossy().replace('\\', "/");
        let key = if cfg!(windows) {
            key.to_lowercase()
        } else {
            key
        };
        if !self.seen.insert(key) {
            return Err(Error::new(
                "UnsafeArchive",
                "duplicate or case-colliding runtime entry",
            ));
        }
        let dest = self.root.join(&path);
        fs::create_dir_all(util::parent(&dest)?)?;
        if let Some(link) = link {
            safe_link(&link)?;
            self.links.push((path, link));
            return Ok(());
        }
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest)?;
        let copied = std::io::copy(&mut reader.take(size + 1), &mut out)?;
        if copied != size {
            return Err(Error::new(
                "UnsafeArchive",
                "entry size differs from its header",
            ));
        }
        out.flush()?;
        util::set_mode(&dest, mode)?;
        Ok(())
    }
    fn finish(self) -> Result<()> {
        // Links are created last; no write can traverse an archive-created link.
        for (path, link) in &self.links {
            symlink(link, &self.root.join(path))?;
        }
        let root = self.root.canonicalize()?;
        for (path, _) in &self.links {
            let target = self.root.join(path).canonicalize()?;
            if !target.starts_with(&root) || !target.is_file() {
                return Err(Error::new(
                    "UnsafeArchive",
                    "dangling, cyclic or external runtime link",
                ));
            }
        }
        Ok(())
    }
}
pub fn extract(archive: &Path, name: &str, platform: &str, root: &Path) -> Result<()> {
    fs::create_dir_all(root)?;
    let mut x = Extraction {
        root,
        seen: BTreeSet::new(),
        links: vec![],
        total: 0,
        count: 0,
    };
    let file = fs::File::open(archive)?;
    if name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(file)?;
        if zip.len() > 200_000 {
            return Err(Error::new("ResourceLimit", "too many archive entries"));
        }
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let Some(path) = selected(&entry.name()?, platform)? else {
                continue;
            };
            if entry.is_dir() {
                continue;
            }
            let mode = entry.unix_mode().unwrap_or(0o644);
            let size = entry.size();
            let link = if mode & 0o170000 == 0o120000 {
                if size > 4096 {
                    return Err(Error::new("UnsafeArchive", "overlong symlink"));
                }
                let mut target = String::new();
                entry.read_to_string(&mut target)?;
                Some(PathBuf::from(target))
            } else {
                None
            };
            x.entry(path, size, mode, link, &mut entry)?;
        }
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
        let mut count = 0;
        for entry in tar.entries()? {
            count += 1;
            if count > 200_000 {
                return Err(Error::new("ResourceLimit", "too many archive entries"));
            }
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().into_owned();
            let Some(path) = selected(&path, platform)? else {
                continue;
            };
            let kind = entry.header().entry_type();
            if kind.is_dir() {
                continue;
            }
            if !kind.is_file() && !kind.is_symlink() {
                return Err(Error::new(
                    "UnsafeArchive",
                    "runtime entry is not a file or symlink",
                ));
            }
            let link = if kind.is_symlink() {
                Some(
                    entry
                        .link_name()?
                        .ok_or_else(|| Error::new("UnsafeArchive", "missing symlink target"))?
                        .into_owned(),
                )
            } else {
                None
            };
            x.entry(path, entry.size(), entry.header().mode()?, link, &mut entry)?;
        }
    } else {
        return Err(Error::new("UnsupportedArchive", name));
    }
    x.finish()
}
pub fn layout(root: &Path, platform: &str) -> Result<PathBuf> {
    let dir = root.join("cangjie/runtime/lib");
    for expected in runtime_names(platform)? {
        for candidate in util::children(&dir)? {
            if candidate
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .eq_ignore_ascii_case(&expected)
                && util::plain_dir(&candidate)
            {
                return Ok(candidate);
            }
        }
    }
    Err(Error::new(
        "InvalidRuntime",
        "official SDK has no supported runtime layout",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsafe_paths() {
        for s in [
            "../x",
            "/tmp/x",
            "a/../../b",
            "C:/foo",
            "x:ads",
            "a\\b",
            "a/CON.txt",
        ] {
            assert!(util::relative(s).is_err(), "{s}");
        }
        assert!(safe_link(Path::new("libpcre2-8.0.dylib")).is_ok());
        assert!(safe_link(Path::new("../escape")).is_err());
    }
    #[test]
    fn traversal_zip_rejected() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("a.zip");
        let mut z = zip::ZipWriter::new(fs::File::create(&p).unwrap());
        z.start_file("../escape", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(b"bad").unwrap();
        z.finish().unwrap();
        assert!(extract(&p, "a.zip", "win32-x64", &t.path().join("out")).is_err());
        assert!(!t.path().join("escape").exists());
    }
}
