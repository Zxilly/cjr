use super::*;
mod fixups;

pub(super) fn versions(file: &object::File<'_>) -> Result<Option<Vec<String>>> {
    let Some(section) = file.section_by_name("__cjsdkversion") else {
        return Ok(None);
    };
    let raw = section.data()?;
    if !raw.len().is_multiple_of(8) || raw.len() > 1024 * 1024 {
        return Err(Error::new(
            "UnsupportedMetadata",
            "invalid version table length",
        ));
    }
    let replacements = fixups::version_pointers(file, &section)?;
    pointers(file, raw, Some(&replacements)).map(Some)
}

fn be32(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(bounds(b, o, 4)?.try_into().map_err(
        |_| Error::new("MalformedBinary", "invalid integer width"),
    )?))
}
fn be64(b: &[u8], o: usize) -> Result<u64> {
    Ok(u64::from_be_bytes(bounds(b, o, 8)?.try_into().map_err(
        |_| Error::new("MalformedBinary", "invalid integer width"),
    )?))
}
pub(super) fn select_slice(b: &[u8]) -> Result<&[u8]> {
    if b.len() < 4 {
        return Ok(b);
    }
    let magic = be32(b, 0)?;
    if !matches!(magic, 0xcafebabe | 0xcafebabf) {
        return Ok(b);
    }
    let n = be32(b, 4)? as usize;
    if n == 0 || n > 128 {
        return Err(Error::new(
            "MalformedBinary",
            "invalid fat architecture count",
        ));
    }
    let width = if magic == 0xcafebabf { 32 } else { 20 };
    bounds(b, 8, n * width)?;
    let cpu = if cfg!(target_arch = "aarch64") {
        0x0100000c
    } else {
        0x01000007
    };
    for i in 0..n {
        let p = 8 + i * width;
        if be32(b, p)? == cpu {
            let (off, len) = if width == 32 {
                (be64(b, p + 8)?, be64(b, p + 16)?)
            } else {
                (be32(b, p + 8)? as u64, be32(b, p + 12)? as u64)
            };
            let off = usize::try_from(off)
                .map_err(|_| Error::new("MalformedBinary", "fat offset overflow"))?;
            let len = usize::try_from(len)
                .map_err(|_| Error::new("MalformedBinary", "fat size overflow"))?;
            if off < 8 + n * width {
                return Err(Error::new("MalformedBinary", "fat slice overlaps header"));
            }
            return bounds(b, off, len);
        }
    }
    Err(Error::new(
        "HostIncompatible",
        "universal binary has no native architecture slice",
    ))
}
pub(super) fn commands(b: &[u8], r: &mut InspectReport) -> Result<()> {
    let n = u32_at(b, 16)? as usize;
    let len = u32_at(b, 20)? as usize;
    let cmds = bounds(b, 32, len)?;
    r.executable = u32_at(b, 12)? == 2;
    if n > len / 8 {
        return Err(Error::new(
            "MalformedBinary",
            "invalid Mach-O load command count",
        ));
    }
    let mut off = 0;
    for _ in 0..n {
        let cmd = u32_at(cmds, off)?;
        let size = u32_at(cmds, off + 4)? as usize;
        if size < 8 {
            return Err(Error::new("MalformedBinary", "invalid Mach-O command size"));
        }
        let raw = bounds(cmds, off, size)?;
        match cmd {
            0xc | 0x80000018 | 0x8000001f | 0x20 | 0x80000023 => {
                let at = u32_at(raw, 8)? as usize;
                r.dependencies.push(Dependency {
                    name: cstr(raw, at, 32768)?,
                    weak: cmd == 0x80000018,
                });
            }
            0x8000001c => {
                r.rpaths.push(cstr(raw, u32_at(raw, 8)? as usize, 32768)?);
            }
            0x24 | 0x32 => {
                let v = u32_at(raw, if cmd == 0x32 { 12 } else { 8 })?;
                r.minimum_os = Some(format!("{}.{}.{}", v >> 16, (v >> 8) & 255, v & 255));
            }
            0x1d => {
                let sig = bounds(b, u32_at(raw, 8)? as usize, u32_at(raw, 12)? as usize)?;
                if sig.len() >= 12 && be32(sig, 0)? == 0xfade0cc0 {
                    let count = be32(sig, 8)? as usize;
                    bounds(
                        sig,
                        12,
                        count
                            .checked_mul(8)
                            .ok_or_else(|| Error::new("MalformedBinary", "signature overflow"))?,
                    )?;
                    for i in 0..count {
                        let at = be32(sig, 16 + i * 8)? as usize;
                        if be32(sig, at)? == 0xfade0c02 {
                            r.hardened |= be32(sig, at + 12)? & (0x10000 | 0x800 | 0x2000) != 0;
                        }
                    }
                }
            }
            _ => {}
        }
        off += size;
    }
    if off != len {
        return Err(Error::new(
            "MalformedBinary",
            "inconsistent Mach-O load command length",
        ));
    }
    Ok(())
}
