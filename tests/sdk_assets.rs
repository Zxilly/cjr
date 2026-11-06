//! Optional, offline checks against already downloaded official macOS archives.
//! Run with CJR_TEST_MAC_ARCHIVES set, then cargo test --test sdk_assets -- --ignored.
use std::path::PathBuf;
#[test]
#[ignore = "requires official SDK research archives; no network in ordinary tests"]
fn official_macos_archives_preserve_dylib_aliases() {
    let source = PathBuf::from(
        std::env::var_os("CJR_TEST_MAC_ARCHIVES").expect("set CJR_TEST_MAC_ARCHIVES"),
    );
    for (version, digest) in [
        (
            "1.0.5",
            "e07d22584237f065b30a771b7110a87a8931334c40be3a148090944ae916ed1f",
        ),
        (
            "1.2.0",
            "de88d6ba2185c94979e0ab2fe8ac1d81535dfdff3fcb51fbba71e4b99d851f97",
        ),
    ] {
        let name = format!("cangjie-sdk-mac-aarch64-{version}.tar.gz");
        let archive = source.join(&name);
        assert_eq!(cjr::util::hash_file(&archive).unwrap(), digest);
        let t = tempfile::tempdir().unwrap();
        cjr::provision::archive::extract(&archive, &name, "darwin-arm64", t.path()).unwrap();
        let rt = cjr::provision::archive::layout(t.path(), "darwin-arm64").unwrap();
        let report = cjr::inspect::inspect(&rt.join("libcangjie-std-core.dylib")).unwrap();
        assert_eq!(report.sdk_version.as_deref(), Some(version));
        assert_eq!(report.target.arch, "aarch64");
        if version == "1.2.0" {
            assert!(rt.join("libpcre2-8.dylib").is_symlink());
            assert!(rt.join("libpcre2-8.0.dylib").is_symlink());
            assert!(
                rt.join("libpcre2-8.dylib")
                    .canonicalize()
                    .unwrap()
                    .is_file()
            );
        }
        let inventory = cjr::store::inventory(t.path()).unwrap();
        assert!(inventory.len() > 40);
        assert!(!t.path().join("cangjie/bin").exists());
    }
}
