mod elf;
mod macho;
use crate::{Error, Result, model::Target, util};
use object::{Object, ObjectSection};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
pub struct Dependency {
    pub name: String,
    pub weak: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct InspectReport {
    pub schema_version: u32,
    pub path: PathBuf,
    pub sha256: String,
    pub format: String,
    pub target: Target,
    pub executable: bool,
    pub sdk_version: Option<String>,
    pub version_entries: usize,
    pub metadata_decoder: String,
    pub dependencies: Vec<Dependency>,
    pub rpaths: Vec<String>,
    pub runpaths: Vec<String>,
    pub interpreter: Option<String>,
    pub minimum_os: Option<String>,
    pub hardened: bool,
    pub symbol_versions: std::collections::BTreeMap<String, Vec<String>>,
    pub diagnostics: Vec<String>,
}
impl InspectReport {
    pub fn needs_runtime(&self) -> bool {
        self.dependencies.iter().any(|d| is_cangjie(&d.name))
    }
}
pub fn is_cangjie(name: &str) -> bool {
    let s = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase();
    s.contains("cangjie-runtime") || s.contains("cangjie-std-")
}
pub fn inspect(path: &Path) -> Result<InspectReport> {
    let path = path.canonicalize().map_err(|e| Error::at(path, e))?;
    let bytes = util::read_limited(&path, 512 * 1024 * 1024)?;
    inspect_bytes(&bytes, path)
}
pub fn inspect_bytes(bytes: &[u8], path: PathBuf) -> Result<InspectReport> {
    let data = macho::select_slice(bytes)?;
    let file = object::File::parse(data)?;
    if !file.is_64() || !file.is_little_endian() {
        return Err(Error::new(
            "UnsupportedMetadata",
            "only 64-bit little-endian native binaries are supported",
        ));
    }
    let arch = match file.architecture() {
        object::Architecture::X86_64 => "x86_64",
        object::Architecture::Aarch64 => "aarch64",
        _ => "unknown",
    };
    let (format, os, decoder) = match file.format() {
        object::BinaryFormat::Pe => ("PE", "windows", "pe-cjsdkv-v1"),
        object::BinaryFormat::Elf => ("ELF", "linux", "elf-cjmetadata-v1"),
        object::BinaryFormat::MachO => ("Mach-O", "macos", "macho-cjsdkversion-v1"),
        _ => {
            return Err(Error::new(
                "UnsupportedMetadata",
                "expected PE, ELF or Mach-O",
            ));
        }
    };
    let mut r = InspectReport {
        schema_version: 1,
        path,
        sha256: util::digest(bytes),
        format: format.into(),
        target: Target {
            os: os.into(),
            arch: arch.into(),
        },
        executable: matches!(
            file.kind(),
            object::ObjectKind::Executable | object::ObjectKind::Dynamic
        ),
        sdk_version: None,
        version_entries: 0,
        metadata_decoder: decoder.into(),
        dependencies: vec![],
        rpaths: vec![],
        runpaths: vec![],
        interpreter: None,
        minimum_os: None,
        hardened: false,
        symbol_versions: Default::default(),
        diagnostics: vec![],
    };
    let versions = match file.format() {
        object::BinaryFormat::Pe => {
            for import in file.imports()? {
                let import = import?;
                let name = String::from_utf8_lossy(import.library()).into_owned();
                if !r.dependencies.iter().any(|d| d.name == name) {
                    r.dependencies.push(Dependency { name, weak: false });
                }
            }
            // IMAGE_FILE_DLL must not be launched, even though it is a valid dependency.
            let pe = u32_at(data, 60)? as usize;
            r.executable = u16_at(data, pe + 22)? & 0x2000 == 0;
            file.section_by_name(".cjsdkv")
                .map(|s| pointers(&file, s.data()?, None))
                .transpose()
        }
        object::BinaryFormat::Elf => {
            elf::dynamic(&file, &mut r)?;
            elf::versions(&file).map(Some)
        }
        object::BinaryFormat::MachO => {
            macho::commands(data, &mut r)?;
            r.hardened |= file.section_by_name("__restrict").is_some();
            file.section_by_name("__cjsdkversion")
                .map(|s| pointers(&file, s.data()?, None))
                .transpose()
        }
        _ => {
            return Err(Error::new(
                "UnsupportedMetadata",
                "unsupported binary format",
            ));
        }
    };
    match versions {
        Ok(Some(v)) => {
            r.version_entries = v.len();
            let unique: BTreeSet<_> = v.into_iter().collect();
            if unique.len() == 1 {
                r.sdk_version = unique.into_iter().next();
            } else if unique.len() > 1 {
                r.diagnostics
                    .push("UnknownSdkVersion: metadata does not identify a single SDK".into());
            }
        }
        Ok(None) => {}
        Err(e) => r.diagnostics.push(format!("UnsupportedMetadata: {e}")),
    }
    if r.sdk_version.is_none() {
        r.diagnostics
            .push("UnknownSdkVersion: no supported SDK version evidence".into());
    }
    Ok(r)
}
fn pointers(
    file: &object::File<'_>,
    raw: &[u8],
    replacements: Option<&std::collections::BTreeMap<usize, u64>>,
) -> Result<Vec<String>> {
    if !raw.len().is_multiple_of(8) || raw.len() > 1024 * 1024 {
        return Err(Error::new(
            "UnsupportedMetadata",
            "invalid version table length",
        ));
    }
    let mut out = vec![];
    for (i, p) in raw.as_chunks::<8>().0.iter().enumerate() {
        let address = replacements
            .and_then(|m| m.get(&(i * 8)))
            .copied()
            .unwrap_or(u64::from_le_bytes(*p));
        if address == 0 {
            continue;
        }
        let s = file
            .sections()
            .find(|s| {
                s.address()
                    .checked_add(s.size())
                    .is_some_and(|end| address >= s.address() && address < end)
            })
            .ok_or_else(|| {
                Error::new(
                    "UnsupportedMetadata",
                    "unmapped version pointer (possibly unsupported fixups)",
                )
            })?;
        let d = s.data()?;
        let offset = usize::try_from(address - s.address())
            .map_err(|_| Error::new("UnsupportedMetadata", "pointer overflow"))?;
        let text = cstr(d, offset, 256)?;
        if !valid_version(&text) {
            return Err(Error::new(
                "UnsupportedMetadata",
                "invalid SDK version string",
            ));
        }
        out.push(text);
    }
    Ok(out)
}
fn valid_version(s: &str) -> bool {
    if s.len() > 255
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
    {
        return false;
    }
    let core = s.split(['-', '+']).next().unwrap_or("");
    let components: Vec<_> = core.split('.').collect();
    components.len() == 3
        && components
            .iter()
            .all(|c| !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit()))
}
fn bounds(b: &[u8], o: usize, n: usize) -> Result<&[u8]> {
    b.get(
        o..o.checked_add(n)
            .ok_or_else(|| Error::new("MalformedBinary", "offset overflow"))?,
    )
    .ok_or_else(|| Error::new("MalformedBinary", "truncated binary"))
}
fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bounds(b, o, 2)?.try_into().map_err(
        |_| Error::new("MalformedBinary", "invalid integer width"),
    )?))
}
fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bounds(b, o, 4)?.try_into().map_err(
        |_| Error::new("MalformedBinary", "invalid integer width"),
    )?))
}
fn u64_at(b: &[u8], o: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(bounds(b, o, 8)?.try_into().map_err(
        |_| Error::new("MalformedBinary", "invalid integer width"),
    )?))
}
fn cstr(b: &[u8], o: usize, max: usize) -> Result<String> {
    let rest = b
        .get(o..)
        .ok_or_else(|| Error::new("MalformedBinary", "string offset out of bounds"))?;
    let n = rest
        .iter()
        .take(max)
        .position(|x| *x == 0)
        .ok_or_else(|| Error::new("MalformedBinary", "unterminated string"))?;
    Ok(std::str::from_utf8(&rest[..n])
        .map_err(|_| Error::new("MalformedBinary", "non-UTF8 binary metadata"))?
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    proptest! { #[test] fn arbitrary_inputs_do_not_panic(data in prop::collection::vec(any::<u8>(),0..8192)) {let _=inspect_bytes(&data,PathBuf::from("input"));} }
    #[test]
    fn version_validation() {
        assert!(valid_version("1.3.0-alpha.20261107001016"));
        assert!(!valid_version("../../foo"));
        assert!(!valid_version("1.2"));
    }
}
