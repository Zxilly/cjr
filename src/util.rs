use crate::{Error, Result};
pub fn file_name(path: &Path) -> Result<&std::ffi::OsStr> {
    path.file_name().ok_or_else(|| {
        Error::new(
            "InvalidPath",
            format!("{} has no file name", path.display()),
        )
    })
}
pub fn parent(path: &Path) -> Result<&Path> {
    path.parent().ok_or_else(|| {
        Error::new(
            "InvalidPath",
            format!("{} has no parent directory", path.display()),
        )
    })
}
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub fn hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}
pub fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
pub fn is_digest(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn hash_file(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path).map_err(|e| Error::at(path, e))?;
    let mut h = Sha256::new();
    let mut b = [0u8; 128 * 1024];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(hex(&h.finalize()))
}
pub fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let f = fs::File::open(path).map_err(|e| Error::at(path, e))?;
    if f.metadata()?.len() > limit {
        return Err(Error::new(
            "ResourceLimit",
            format!("{} exceeds {limit} bytes", path.display()),
        ));
    }
    let mut bytes = Vec::new();
    f.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error::new("ResourceLimit", "file grew while reading"));
    }
    Ok(bytes)
}
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&read_limited(
        path,
        32 * 1024 * 1024,
    )?)?)
}
pub fn atomic_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("InvalidPath", "missing parent"))?;
    fs::create_dir_all(parent)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut f, value)?;
    f.write_all(b"\n")?;
    f.as_file().sync_all()?;
    f.persist(path).map_err(|e| Error::at(path, e.error))?;
    Ok(())
}
/// Portable archive/receipt path, including rejection of Windows alternate streams.
pub fn relative(s: &str) -> Result<PathBuf> {
    if s.is_empty() || s.contains(['\\', ':', '\0']) || s.starts_with('/') {
        return Err(Error::new("UnsafePath", s));
    }
    let p = Path::new(s);
    if p.components().any(|c| !matches!(c, Component::Normal(_)))
        || s.split('/')
            .any(|x| x == ".." || x == "." || x.is_empty() || x.ends_with(['.', ' ']))
    {
        return Err(Error::new("UnsafePath", s));
    }
    for c in s.split('/') {
        let stem = c.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4
                && stem.as_bytes()[3].is_ascii_digit())
        {
            return Err(Error::new("UnsafePath", s));
        }
    }
    Ok(p.to_owned())
}
pub fn plain_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !is_link(&m))
}
pub fn is_link(m: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}
pub fn children(path: &Path) -> Result<Vec<PathBuf>> {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut v = entries
                .map(|e| e.map(|e| e.path()))
                .collect::<std::io::Result<Vec<_>>>()?;
            v.sort();
            Ok(v)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(Error::at(path, e)),
    }
}
pub fn mode(m: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        m.permissions().mode() & 0o777
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        0
    }
}
pub fn set_mode(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777))?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
    Ok(())
}
