use super::*;
use std::collections::BTreeMap;
pub(super) fn versions(file: &object::File<'_>) -> Result<Vec<String>> {
    let Some(s) = file.section_by_name(".cjmetadata") else {
        return Ok(vec![]);
    };
    let data = s.data()?;
    if u32_at(data, 0)? != 0x12345678 {
        return Err(Error::new(
            "UnsupportedMetadata",
            "unknown .cjmetadata magic",
        ));
    }
    let start = u32_at(data, 16)? as usize;
    let end = u32_at(data, 20)? as usize;
    if start < 24 || !start.is_multiple_of(8) || end < start {
        return Err(Error::new(
            "UnsupportedMetadata",
            "invalid SDK version table bounds",
        ));
    }
    let table = bounds(data, start, end - start)?;
    let mut rel = BTreeMap::new();
    for rs in file.sections() {
        if rs.name().unwrap_or("").starts_with(".rela") {
            let raw = rs.data()?;
            if raw.len() % 24 != 0 {
                return Err(Error::new("MalformedBinary", "invalid RELA table"));
            }
            for entry in raw.chunks_exact(24) {
                let address = u64_at(entry, 0)?;
                let kind = u64_at(entry, 8)? as u32;
                let base = s
                    .address()
                    .checked_add(start as u64)
                    .ok_or_else(|| Error::new("MalformedBinary", "address overflow"))?;
                if address >= base && address - base < table.len() as u64 {
                    let expected = if file.architecture() == object::Architecture::X86_64 {
                        8
                    } else {
                        1027
                    };
                    if kind != expected {
                        return Err(Error::new(
                            "UnsupportedMetadata",
                            "unsupported SDK version relocation",
                        ));
                    }
                    rel.insert((address - base) as usize, u64_at(entry, 16)?);
                }
            }
        }
    }
    pointers(file, table, Some(&rel))
}
pub(super) fn dynamic(file: &object::File<'_>, r: &mut InspectReport) -> Result<()> {
    r.executable = file.kind() == object::ObjectKind::Executable;
    if let Some(s) = file.section_by_name(".interp") {
        r.interpreter = Some(cstr(s.data()?, 0, 4096)?);
        r.executable = true;
    }
    let Some(dynamic) = file.section_by_name(".dynamic") else {
        return Ok(());
    };
    let strings = file
        .section_by_name(".dynstr")
        .ok_or_else(|| Error::new("UnsupportedMetadata", "missing ELF .dynstr"))?;
    let raw = dynamic.data()?;
    let strs = strings.data()?;
    if raw.len() % 16 != 0 {
        return Err(Error::new("MalformedBinary", "invalid dynamic section"));
    }
    for entry in raw.chunks_exact(16) {
        let tag = u64_at(entry, 0)?;
        let value = u64_at(entry, 8)?;
        if tag == 0x6ffffffb && value & 0x08000000 != 0 {
            r.executable = true;
        }
        if tag == 0 {
            break;
        }
        if matches!(tag, 1 | 15 | 29) {
            let name = cstr(
                strs,
                usize::try_from(value)
                    .map_err(|_| Error::new("MalformedBinary", "string overflow"))?,
                32768,
            )?;
            match tag {
                1 => r.dependencies.push(Dependency { name, weak: false }),
                15 => r.rpaths.extend(name.split(':').map(Into::into)),
                29 => r.runpaths.extend(name.split(':').map(Into::into)),
                _ => {}
            }
        }
    }
    if let Some(section) = file.section_by_name(".gnu.version_r") {
        let bytes = section.data()?;
        let mut offset = 0;
        let mut count = 0;
        while offset < bytes.len() {
            count += 1;
            if count > 4096 {
                return Err(Error::new(
                    "ResourceLimit",
                    "too many ELF symbol version records",
                ));
            }
            let library = cstr(strs, u32_at(bytes, offset + 4)? as usize, 32768)?;
            let n = u16_at(bytes, offset + 2)? as usize;
            let mut aux = offset
                .checked_add(u32_at(bytes, offset + 8)? as usize)
                .ok_or_else(|| Error::new("MalformedBinary", "version offset overflow"))?;
            if n > 4096 {
                return Err(Error::new("ResourceLimit", "too many ELF symbol versions"));
            }
            for _ in 0..n {
                r.symbol_versions
                    .entry(library.clone())
                    .or_default()
                    .push(cstr(strs, u32_at(bytes, aux + 8)? as usize, 4096)?);
                let next = u32_at(bytes, aux + 12)? as usize;
                if next == 0 {
                    break;
                }
                aux = aux
                    .checked_add(next)
                    .ok_or_else(|| Error::new("MalformedBinary", "version offset overflow"))?;
            }
            let next = u32_at(bytes, offset + 12)? as usize;
            if next == 0 {
                break;
            }
            offset = offset
                .checked_add(next)
                .ok_or_else(|| Error::new("MalformedBinary", "version offset overflow"))?;
        }
    }
    Ok(())
}
