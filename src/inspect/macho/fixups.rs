//! Resolve on-disk fixups without loading or modifying the executable.
//!
//! `object` owns chain traversal and pointer decoding. This module maps its
//! results to the Cangjie version table and enforces our metadata boundaries.
use crate::{Error, Result};
use object::{Object, ObjectSection, ObjectSegment, macho, read::macho::Fixup};
use std::collections::BTreeMap;

pub(super) fn version_pointers(
    file: &object::File<'_>,
    section: &object::Section<'_, '_>,
) -> Result<BTreeMap<usize, u64>> {
    let mut replacements = BTreeMap::new();
    let object::File::MachO64(mach) = file else {
        return Err(invalid("expected 64-bit Mach-O"));
    };
    let endian = mach.endian();
    let mut commands = mach.macho_load_commands()?;
    let mut command = None;
    while let Some(load) = commands.next()? {
        if let Some(fixups) = load.dyld_chained_fixups()?
            && command.replace(fixups).is_some()
        {
            return Err(invalid("duplicate chained fixups command"));
        }
    }
    let Some(command) = command else {
        return Ok(replacements);
    };
    let segments: Vec<_> = mach.segments().collect();
    let base = segments
        .iter()
        .find(|s| s.name_bytes().ok() == Some(Some(b"__TEXT")))
        .ok_or_else(|| invalid("chained fixups require a __TEXT segment"))?
        .address();
    let table_start = section.address();
    let table_end = table_start
        .checked_add(section.size())
        .ok_or_else(|| invalid("version table address overflow"))?;
    let table_file = section
        .file_range()
        .ok_or_else(|| invalid("version table is not backed by file data"))?;
    let fixups = command.chained_fixups(endian, mach.data())?;
    // Bound work even when malformed multi-start lists revisit long chains.
    let mut remaining = 1_000_000usize;
    for starts in fixups.segments(endian)? {
        let starts = starts?;
        let segment = segments
            .get(starts.index() as usize)
            .ok_or_else(|| invalid("chained fixups reference a missing segment"))?;
        let segment_end = segment
            .address()
            .checked_add(segment.size())
            .ok_or_else(|| invalid("segment address overflow"))?;
        if table_start >= segment_end || table_end <= segment.address() {
            continue;
        }
        if table_start < segment.address() || table_end > segment_end {
            return Err(invalid("version table crosses segment bounds"));
        }
        let table_offset = table_start - segment.address();
        if segment.file_range().0.checked_add(table_offset) != Some(table_file.0) {
            return Err(invalid("version table file and virtual addresses disagree"));
        }
        let header = starts.header();
        if base.checked_add(header.segment_offset.get(endian)) != Some(segment.address()) {
            return Err(invalid(
                "chained fixups segment offset disagrees with load commands",
            ));
        }
        let page_size = u64::from(header.page_size.get(endian));
        if !matches!(page_size, 0x1000 | 0x4000) {
            return Err(invalid("invalid chained fixups page size"));
        }
        let format = header.pointer_format.get(endian);
        if !matches!(
            format,
            macho::DYLD_CHAINED_PTR_64
                | macho::DYLD_CHAINED_PTR_64_OFFSET
                | macho::DYLD_CHAINED_PTR_ARM64E
                | macho::DYLD_CHAINED_PTR_ARM64E_USERLAND
                | macho::DYLD_CHAINED_PTR_ARM64E_USERLAND24
        ) {
            return Err(invalid("unsupported version pointer format"));
        }
        let data = segment.data()?;
        if table_offset
            .checked_add(section.size())
            .is_none_or(|end| end > data.len() as u64)
        {
            return Err(invalid("version table exceeds segment file data"));
        }
        for entry in starts.fixups(endian, base, data) {
            remaining = remaining
                .checked_sub(1)
                .ok_or_else(|| Error::new("ResourceLimit", "too many version segment fixups"))?;
            let (offset, fixup) = entry?;
            // The library bounds reads to the segment. Also reject a link that
            // crosses a page or overlaps its preceding eight-byte pointer.
            let raw = super::u64_at(
                data,
                usize::try_from(offset).map_err(|_| invalid("fixup offset overflow"))?,
            )?;
            let next = match format {
                macho::DYLD_CHAINED_PTR_64 | macho::DYLD_CHAINED_PTR_64_OFFSET => {
                    macho::DyldChainedPtr64(raw).next() * 4
                }
                _ => macho::DyldChainedPtrArm64e(raw).next() * 8,
            };
            if offset % page_size + 8 > page_size
                || (next != 0 && (next < 8 || offset % page_size + next + 8 > page_size))
            {
                return Err(invalid(
                    "chained fixup crosses a page or overlaps a pointer",
                ));
            }
            let address = segment
                .address()
                .checked_add(offset)
                .ok_or_else(|| invalid("fixup address overflow"))?;
            let end = address
                .checked_add(8)
                .ok_or_else(|| invalid("fixup address overflow"))?;
            if address >= table_end || end <= table_start {
                continue;
            }
            if address < table_start
                || end > table_end
                || !(address - table_start).is_multiple_of(8)
            {
                return Err(invalid("chained fixup is not aligned to a version entry"));
            }
            let Fixup::Rebase(rebase) = fixup else {
                return Err(invalid(
                    "version entry binds an external symbol instead of a local string",
                ));
            };
            let target = base
                .checked_add(rebase.target_offset)
                .ok_or_else(|| invalid("version rebase target overflow"))?;
            let index = usize::try_from(address - table_start)
                .map_err(|_| invalid("version entry offset overflow"))?;
            if replacements.insert(index, target).is_some() {
                return Err(invalid("duplicate chained fixup for a version entry"));
            }
        }
    }
    Ok(replacements)
}

fn invalid(message: &str) -> Error {
    Error::new("UnsupportedMetadata", message)
}
