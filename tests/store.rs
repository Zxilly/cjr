use cjr::{config::Config, model::Artifact, store::Store};
use std::{fs, time::Duration};
fn config(root: &std::path::Path) -> Config {
    Config {
        home: root.join("cjr"),
        cjv_home: None,
        warnings: vec![],
    }
}
#[test]
fn publication_integrity_and_active_cleanup() {
    let t = tempfile::tempdir().unwrap();
    let store = Store::new(&config(t.path()));
    let guard = store.guard().unwrap();
    let payload = store.root.join("staging/source");
    fs::create_dir_all(payload.join("cangjie/runtime/lib/linux_x86_64_cjnative")).unwrap();
    let file = "cangjie/runtime/lib/linux_x86_64_cjnative/libtest.so";
    fs::write(payload.join(file), b"payload").unwrap();
    let a = Artifact {
        channel: "lts".into(),
        version: "1.0.5".into(),
        platform: "linux-x64".into(),
        name: "sdk.tar.gz".into(),
        url: "https://example.com/sdk.tar.gz".into(),
        sha256: "ab".repeat(32),
    };
    let ready = guard
        .publish(
            &payload,
            &a,
            std::path::Path::new("test"),
            "archive-verified",
        )
        .unwrap();
    assert!(guard.find(&a).unwrap().is_some());
    let lease = guard.begin_use(&ready.receipt.bundle_id).unwrap();
    drop(guard);
    let entries = store.clean(true, false).unwrap();
    assert!(
        entries
            .iter()
            .any(|x| matches!(x.status, cjr::store::CleanStatus::Skipped(_)))
    );
    assert!(ready.root.exists());
    lease.finish(true).unwrap();
    fs::write(ready.root.join("payload").join(file), b"modified").unwrap();
    assert!(store.guard().unwrap().find(&a).is_err());
    store.clean(true, false).unwrap();
    assert!(!ready.root.exists());
}
#[test]
fn lock_protocol_excludes_second_handle() {
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("lock");
    let first = cjr::lock::Lock::acquire(&path, Duration::ZERO, true).unwrap();
    assert!(cjr::lock::Lock::acquire(&path, Duration::ZERO, false).is_err());
    drop(first);
    assert!(cjr::lock::Lock::acquire(&path, Duration::ZERO, false).is_ok());
}

#[test]
fn unowned_directory_is_never_claimed_or_cleaned() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::new(&config(t.path()));
    fs::create_dir_all(s.root.join("downloads")).unwrap();
    fs::write(s.root.join("downloads/personal"), b"keep").unwrap();
    assert!(s.guard().is_err());
    assert!(s.clean(true, false).is_err());
    assert!(s.root.join("downloads/personal").is_file());
}

#[test]
fn unknown_usage_is_retained() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::new(&config(t.path()));
    drop(s.guard().unwrap());
    let id = "f".repeat(64);
    fs::create_dir(s.root.join("runtimes").join(&id)).unwrap();
    fs::create_dir(s.root.join("usage").join(&id)).unwrap();
    fs::write(
        s.root.join("usage").join(&id).join("invalid.json"),
        b"broken",
    )
    .unwrap();
    let items = s.clean(true, false).unwrap();
    assert!(
        items
            .iter()
            .any(|e| e.status.to_string() == "skipped: unknown")
    );
    assert!(s.root.join("runtimes").join(id).exists());
}

#[test]
fn cancelled_launch_releases_lease_but_uncertain_exit_retains_it() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::new(&config(t.path()));
    let guard = s.guard().unwrap();
    let id = "a".repeat(64);
    fs::create_dir(s.root.join("runtimes").join(&id)).unwrap();
    let lease = guard.begin_use(&id).unwrap();
    assert_ne!(s.usage(&id), cjr::store::UsageState::Unused);
    drop(lease);
    assert_eq!(s.usage(&id), cjr::store::UsageState::Unused);
    guard.begin_use(&id).unwrap().finish(false).unwrap();
    assert_ne!(s.usage(&id), cjr::store::UsageState::Unused);
    drop(guard);
    assert!(
        s.clean(true, false)
            .unwrap()
            .iter()
            .any(|e| matches!(e.status, cjr::store::CleanStatus::Skipped(_)))
    );
}

#[test]
fn publication_cannot_consume_external_sdk() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::new(&config(t.path()));
    let guard = s.guard().unwrap();
    let external = t.path().join("user-sdk");
    fs::create_dir(&external).unwrap();
    fs::write(external.join("keep"), b"owned by user").unwrap();
    let artifact = Artifact {
        channel: "lts".into(),
        version: "1.0.5".into(),
        platform: "linux-x64".into(),
        name: "sdk.tar.gz".into(),
        url: "https://example.com/sdk.tar.gz".into(),
        sha256: "ab".repeat(32),
    };
    let result = guard.publish(&external, &artifact, &external, "archive-verified");
    assert!(matches!(
        result,
        Err(cjr::Error::Diagnostic {
            code: "UnsafeCache",
            ..
        })
    ));
    assert_eq!(fs::read(external.join("keep")).unwrap(), b"owned by user");
}

#[cfg(unix)]
#[test]
fn linked_usage_directory_never_receives_session_records() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::new(&config(t.path()));
    let guard = s.guard().unwrap();
    let id = "b".repeat(64);
    fs::create_dir(s.root.join("runtimes").join(&id)).unwrap();
    let outside = t.path().join("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, s.root.join("usage").join(&id)).unwrap();
    assert!(guard.begin_use(&id).is_err());
    assert_eq!(s.usage(&id), cjr::store::UsageState::Unknown);
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}
#[cfg(unix)]
#[test]
fn cleaning_never_follows_links() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::new(&config(t.path()));
    drop(s.guard().unwrap());
    let outside = t.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"x").unwrap();
    std::os::unix::fs::symlink(&outside, s.root.join("downloads/link")).unwrap();
    s.clean(false, false).unwrap();
    assert!(outside.join("keep").exists());
    assert!(!s.root.join("downloads/link").exists());
}
