use std::path::PathBuf;
fn put32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn put64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn put16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn thin(cpu: u32, version: &str) -> Vec<u8> {
    let mut b = vec![0u8; 512];
    put32(&mut b, 0, 0xfeedfacf);
    put32(&mut b, 4, cpu);
    put32(&mut b, 12, 2);
    put32(&mut b, 16, 1);
    put32(&mut b, 20, 72 + 80 * 2);
    let o = 32;
    put32(&mut b, o, 0x19);
    put32(&mut b, o + 4, 232);
    b[o + 8..o + 21].copy_from_slice(b"__CJ_METADATA");
    put64(&mut b, o + 24, 0x1000);
    put64(&mut b, o + 32, 512);
    put64(&mut b, o + 40, 0);
    put64(&mut b, o + 48, 512);
    put32(&mut b, o + 64, 2);
    let s = o + 72;
    b[s..s + 14].copy_from_slice(b"__cjsdkversion");
    b[s + 16..s + 29].copy_from_slice(b"__CJ_METADATA");
    put64(&mut b, s + 32, 0x1000 + 400);
    put64(&mut b, s + 40, 8);
    put32(&mut b, s + 48, 400);
    let s = s + 80;
    b[s..s + 9].copy_from_slice(b"__cstring");
    b[s + 16..s + 29].copy_from_slice(b"__CJ_METADATA");
    put64(&mut b, s + 32, 0x1000 + 420);
    put64(&mut b, s + 40, 80);
    put32(&mut b, s + 48, 420);
    put64(&mut b, 400, 0x1000 + 420);
    b[420..420 + version.len()].copy_from_slice(version.as_bytes());
    b
}
#[test]
fn thin_and_native_fat_slice() {
    let arm = thin(0x0100000c, "1.0.5");
    let x64 = thin(0x01000007, "1.2.0");
    assert_eq!(
        cjr::inspect::inspect_bytes(&arm, PathBuf::from("arm"))
            .unwrap()
            .sdk_version
            .as_deref(),
        Some("1.0.5")
    );
    let mut fat = vec![0u8; 2048];
    fat[..4].copy_from_slice(&0xcafebabeu32.to_be_bytes());
    fat[4..8].copy_from_slice(&2u32.to_be_bytes());
    for (i, cpu, off) in [(0, 0x0100000cu32, 512u32), (1, 0x01000007u32, 1024u32)] {
        let o = 8 + i * 20;
        fat[o..o + 4].copy_from_slice(&cpu.to_be_bytes());
        fat[o + 8..o + 12].copy_from_slice(&off.to_be_bytes());
        fat[o + 12..o + 16].copy_from_slice(&512u32.to_be_bytes());
    }
    fat[512..1024].copy_from_slice(&arm);
    fat[1024..1536].copy_from_slice(&x64);
    let expected = if cfg!(target_arch = "aarch64") {
        "1.0.5"
    } else {
        "1.2.0"
    };
    assert_eq!(
        cjr::inspect::inspect_bytes(&fat, PathBuf::from("fat"))
            .unwrap()
            .sdk_version
            .as_deref(),
        Some(expected)
    );
}
#[test]
fn malformed_version_pointer_is_not_guessed() {
    let mut b = thin(0x01000007, "1.2.0");
    put64(&mut b, 400, u64::MAX);
    let r = cjr::inspect::inspect_bytes(&b, "bad".into()).unwrap();
    assert!(r.sdk_version.is_none());
    assert!(
        r.diagnostics
            .iter()
            .any(|s| s.contains("UnsupportedMetadata"))
    );
}

// __PAGEZERO and deliberately different file/VM offsets exercise mapping by
// segment index, rather than assuming that image offsets are file offsets.
const BASE: u64 = 0x1_0000_0000;
const TABLE: usize = 0x1100;
const FIXUPS: usize = 0x3000;
const STARTS: usize = FIXUPS + 32;
const SEGMENT_STARTS: usize = STARTS + 32;

fn segment(b: &mut [u8], at: usize, name: &[u8], vm: u64, off: u64, size: u64, sections: u32) {
    put32(b, at, 0x19);
    put32(b, at + 4, 72 + 80 * sections);
    b[at + 8..at + 8 + name.len()].copy_from_slice(name);
    put64(b, at + 24, vm);
    put64(b, at + 32, size);
    put64(b, at + 40, off);
    put64(b, at + 48, if name == b"__PAGEZERO" { 0 } else { size });
    put32(b, at + 64, sections);
}

fn section(b: &mut [u8], at: usize, name: &[u8], segment: &[u8], vm: u64, off: u32, size: u64) {
    b[at..at + name.len()].copy_from_slice(name);
    b[at + 16..at + 16 + segment.len()].copy_from_slice(segment);
    put64(b, at + 32, vm);
    put64(b, at + 40, size);
    put32(b, at + 48, off);
}

fn chained(cpu: u32, format: u16, authenticated: bool) -> Vec<u8> {
    let mut b = vec![0; 0x3100];
    put32(&mut b, 0, 0xfeedfacf);
    put32(&mut b, 4, cpu);
    put32(&mut b, 12, 2);
    put32(&mut b, 16, 5);
    put32(&mut b, 20, 72 + 152 + 152 + 72 + 16);
    segment(&mut b, 32, b"__PAGEZERO", 0, 0, BASE, 0);
    segment(&mut b, 104, b"__TEXT", BASE, 0, 0x1000, 1);
    section(
        &mut b,
        176,
        b"__cstring",
        b"__TEXT",
        BASE + 0x800,
        0x800,
        16,
    );
    segment(
        &mut b,
        256,
        b"__CJ_METADATA",
        BASE + 0x4000,
        0x1000,
        0x2000,
        1,
    );
    section(
        &mut b,
        328,
        b"__cjsdkversion",
        b"__CJ_METADATA",
        BASE + 0x4100,
        TABLE as u32,
        16,
    );
    segment(
        &mut b,
        408,
        b"__LINKEDIT",
        BASE + 0x8000,
        FIXUPS as u64,
        0x100,
        0,
    );
    put32(&mut b, 480, 0x80000034);
    put32(&mut b, 484, 16);
    put32(&mut b, 488, FIXUPS as u32);
    put32(&mut b, 492, 96);
    b[0x800..0x806].copy_from_slice(b"1.2.0\0");

    put32(&mut b, FIXUPS + 4, 32);
    put32(&mut b, STARTS, 4);
    put32(&mut b, STARTS + 12, 32); // segment index 2
    put32(&mut b, SEGMENT_STARTS, 26);
    put16(&mut b, SEGMENT_STARTS + 4, 0x1000);
    put16(&mut b, SEGMENT_STARTS + 6, format);
    put64(&mut b, SEGMENT_STARTS + 8, 0x4000);
    put16(&mut b, SEGMENT_STARTS + 20, 2);
    put16(&mut b, SEGMENT_STARTS + 22, 0x80);
    put16(&mut b, SEGMENT_STARTS + 24, 0xffff);

    let arm = matches!(format, 1 | 9 | 12);
    let stride = if arm { 8 } else { 4 };
    let bind = 1u64 << if arm { 62 } else { 63 };
    put64(&mut b, 0x1080, bind | ((0x80 / stride) << 51));
    let pointer = if authenticated {
        // Authentication fields must not become part of the string address.
        (1 << 63) | (0x1234 << 32) | (1 << 48) | (2 << 49) | 0x800
    } else if matches!(format, 1 | 2) {
        BASE + 0x800
    } else {
        0x800
    };
    put64(&mut b, TABLE, pointer | ((8 / stride) << 51));
    put64(&mut b, TABLE + 8, pointer);
    b
}

fn assert_version(b: &[u8], entries: usize) {
    let report = cjr::inspect::inspect_bytes(b, "chained".into()).unwrap();
    assert_eq!(
        report.sdk_version.as_deref(),
        Some("1.2.0"),
        "{:?}",
        report.diagnostics
    );
    assert_eq!(report.version_entries, entries);
    assert!(report.diagnostics.is_empty());
}

fn assert_rejected(b: &[u8], reason: &str) {
    let report = cjr::inspect::inspect_bytes(b, "malformed".into()).unwrap();
    assert!(report.sdk_version.is_none(), "{reason}");
    assert!(
        report.diagnostics.iter().any(|d| d.contains(reason)),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn chained_userland_pointer_formats() {
    for cpu in [0x01000007, 0x0100000c] {
        for format in [2, 6] {
            assert_version(&chained(cpu, format, false), 2);
        }
    }
    for format in [1, 9, 12] {
        for authenticated in [false, true] {
            assert_version(&chained(0x0100000c, format, authenticated), 2);
        }
    }
}

#[test]
fn chained_fat_slice_uses_slice_relative_offsets() {
    let image = chained(
        if cfg!(target_arch = "aarch64") {
            0x0100000c
        } else {
            0x01000007
        },
        6,
        false,
    );
    let mut fat = vec![0; 0x1000 + image.len()];
    fat[..4].copy_from_slice(&0xcafebabeu32.to_be_bytes());
    fat[4..8].copy_from_slice(&1u32.to_be_bytes());
    fat[8..12].copy_from_slice(&u32::from_le_bytes(image[4..8].try_into().unwrap()).to_be_bytes());
    fat[16..20].copy_from_slice(&0x1000u32.to_be_bytes());
    fat[20..24].copy_from_slice(&(image.len() as u32).to_be_bytes());
    fat[0x1000..].copy_from_slice(&image);
    assert_version(&fat, 2);
}

#[test]
fn chained_multiple_starts_and_empty_pages() {
    let mut b = chained(0x01000007, 6, false);
    put32(&mut b, SEGMENT_STARTS, 30);
    put16(&mut b, SEGMENT_STARTS + 22, 0x8002);
    put16(&mut b, SEGMENT_STARTS + 26, 0x100);
    put16(&mut b, SEGMENT_STARTS + 28, 0x8108);
    put64(&mut b, TABLE, 0x800);
    assert_version(&b, 2);
    // The table can also lie on a later page after an empty page.
    put16(&mut b, SEGMENT_STARTS + 22, 0xffff);
    put16(&mut b, SEGMENT_STARTS + 24, 0x100);
    put64(&mut b, 360, BASE + 0x5100);
    put32(&mut b, 376, 0x2100);
    put64(&mut b, 0x2100, 0x800 | (2 << 51));
    put64(&mut b, 0x2108, 0x800);
    assert_version(&b, 2);
}

#[test]
fn unchained_entries_remain_plain_pointers_or_null() {
    let mut b = chained(0x01000007, 6, false);
    put64(&mut b, TABLE, 0x800); // End this chain before the second table entry.
    put64(&mut b, TABLE + 8, 0);
    assert_version(&b, 1);
    put64(&mut b, TABLE + 8, BASE + 0x800);
    assert_version(&b, 2);
    put32(&mut b, STARTS + 12, 0); // No chains in the metadata segment.
    put64(&mut b, TABLE, BASE + 0x800);
    assert_version(&b, 2);
}

#[test]
fn malformed_chains_do_not_produce_a_version() {
    let original = chained(0x01000007, 6, false);
    for (at, value, reason) in [
        (FIXUPS, 1, "fixups version"),
        (FIXUPS + 4, u32::MAX, "starts offset"),
        (SEGMENT_STARTS, 10, "starts in segment"),
        (SEGMENT_STARTS + 8, 0x1000, "segment offset"),
        (SEGMENT_STARTS + 4, 0x0006_0000, "page size"),
        (SEGMENT_STARTS + 4, 0xffff_1000, "pointer format"),
        (492, 12, "fixups header"),
    ] {
        let mut b = original.clone();
        put32(&mut b, at, value);
        assert_rejected(&b, reason);
    }
    for (pointer, reason) in [
        (1 << 63, "external symbol"),
        (0x800 | (0xfff << 51), "crosses a page"),
        (0x800 | (1 << 51), "overlaps a pointer"),
        (0xdead, "unmapped version pointer"),
    ] {
        let mut b = original.clone();
        put64(&mut b, TABLE, pointer);
        assert_rejected(&b, reason);
    }
    let mut b = original.clone();
    put16(&mut b, SEGMENT_STARTS + 22, 0x8002);
    assert_rejected(&b, "chain index");
    let mut b = original.clone();
    put32(&mut b, SEGMENT_STARTS, 30);
    put16(&mut b, SEGMENT_STARTS + 22, 0x8002);
    put16(&mut b, SEGMENT_STARTS + 26, 0x100);
    put16(&mut b, SEGMENT_STARTS + 28, 0x8100);
    assert_rejected(&b, "duplicate chained fixup");
    let mut b = original;
    put16(&mut b, SEGMENT_STARTS + 22, 0x104);
    assert_rejected(&b, "not aligned");

    let mut b = chained(0x01000007, 6, false);
    put32(&mut b, 376, TABLE as u32 + 8);
    assert_rejected(&b, "file and virtual addresses disagree");
    let mut b = chained(0x01000007, 6, false);
    b.copy_within(480..496, 496);
    put32(&mut b, 16, 6);
    put32(&mut b, 20, 480);
    assert_rejected(&b, "duplicate chained fixups command");
}

proptest::proptest! {
    #[test]
    fn damaged_fixups_never_panic(
        edits in proptest::collection::vec((0usize..96, proptest::prelude::any::<u8>()), 0..32),
        pointer in proptest::prelude::any::<u64>(),
    ) {
        let mut b = chained(0x01000007, 6, false);
        for (offset, byte) in edits {
            b[FIXUPS + offset] = byte;
        }
        put64(&mut b, TABLE, pointer);
        let _ = cjr::inspect::inspect_bytes(&b, "damaged-chains".into());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn apple_linker_version_metadata_with_and_without_chains() {
    use std::{fs, process::Command};

    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("version.c");
    fs::write(
        &source,
        r#"
        __attribute__((used, section("__CJ_METADATA,__cjsdkversion")))
        const char *versions[] = { "1.2.0", "1.2.0" };
        int main(void) { return versions[0][0] == '1' ? 0 : 1; }
        "#,
    )
    .unwrap();
    for chains in [false, true] {
        let binary = dir.path().join(if chains { "chained" } else { "classic" });
        let output = Command::new("xcrun")
            .arg("clang")
            .arg("-mmacosx-version-min=13.0")
            .arg(if chains {
                "-Wl,-fixup_chains"
            } else {
                "-Wl,-no_fixup_chains"
            })
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = fs::read(&binary).unwrap();
        let object::File::MachO64(file) = object::File::parse(bytes.as_slice()).unwrap() else {
            panic!("Apple linker did not produce a 64-bit Mach-O");
        };
        let mut commands = file.macho_load_commands().unwrap();
        let mut has_chains = false;
        while let Some(command) = commands.next().unwrap() {
            has_chains |= command.dyld_chained_fixups().unwrap().is_some();
        }
        assert_eq!(has_chains, chains);
        assert_version(&bytes, 2);
    }
}
