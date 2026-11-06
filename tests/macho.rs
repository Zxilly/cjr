use std::path::PathBuf;
fn put32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn put64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
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
