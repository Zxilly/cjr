use std::{fs, process::Command};
fn cmd(home: &std::path::Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_cjr"));
    c.env("CJR_HOME", home.join("cjr"))
        .env("CJV_HOME", home.join("cjv"));
    c
}
#[test]
fn read_only_commands_do_not_create_cache() {
    let t = tempfile::tempdir().unwrap();
    let output = cmd(t.path()).args(["list", "--json"]).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["entries"].as_array().unwrap().len(), 0);
    assert!(!t.path().join("cjr").exists());
    assert!(!t.path().join("cjv").exists());
    let output = cmd(t.path())
        .arg("inspect")
        .arg(env!("CARGO_BIN_EXE_cjr"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!t.path().join("cjr").exists());
    assert!(
        cmd(t.path())
            .args(["cache", "clean", "--all", "--dry-run"])
            .status()
            .unwrap()
            .success()
    );
    assert!(!t.path().join("cjr").exists());
}
#[test]
fn malformed_and_unknown_inputs_fail_without_installing() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("bad");
    fs::write(&p, b"not a binary").unwrap();
    assert!(!cmd(t.path()).arg(&p).status().unwrap().success());
    assert!(
        !cmd(t.path())
            .arg(env!("CARGO_BIN_EXE_cjr"))
            .status()
            .unwrap()
            .success()
    );
    assert!(!t.path().join("cjr").exists());
}
#[test]
fn command_scope() {
    let t = tempfile::tempdir().unwrap();
    assert!(!cmd(t.path()).output().unwrap().status.success());
    for arg in ["--help", "--version"] {
        assert!(cmd(t.path()).arg(arg).output().unwrap().status.success());
    }
    assert!(
        !cmd(t.path())
            .args(["cache", "import"])
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn cjv_settings_precedence_and_invalid_environment() {
    let t = tempfile::tempdir().unwrap();
    let managed = t.path().join("managed");
    fs::create_dir_all(managed.join("cache")).unwrap();
    let key = "a".repeat(64);
    fs::write(
        managed.join("cache").join(format!("{key}.data")),
        b"fixture",
    )
    .unwrap();
    fs::write(managed.join("cache").join(format!("{key}.json")),serde_json::to_vec(&serde_json::json!({"key":key,"kind":"artifact","url":"https://example.com/sdk","sha256":"b".repeat(64),"size":7})).unwrap()).unwrap();
    let settings = t.path().join("settings.toml");
    fs::write(
        &settings,
        format!("version = 1\nhome = '{}'\n", managed.display()),
    )
    .unwrap();
    let o = cmd(t.path())
        .env_remove("CJV_HOME")
        .env("CJV_CONFIG_FILE", &settings)
        .args(["list", "--json"])
        .output()
        .unwrap();
    assert!(o.status.success());
    let j: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(j["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        j["entries"][0]["sources"][0]["status"],
        "official identity unconfirmed"
    );
    let o = cmd(t.path())
        .env("CJV_HOME", "relative")
        .args(["list", "--json"])
        .output()
        .unwrap();
    assert!(o.status.success());
    let j: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(!j["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn explicit_cache_does_not_require_user_home() {
    let t = tempfile::tempdir().unwrap();
    let output = cmd(t.path())
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .args(["list", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!t.path().join("cjr").exists());
}
