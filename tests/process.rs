use std::{fs, process::Command};
#[test]
fn native_argument_and_exit_transparency() {
    #[cfg(unix)]
    let foreground_before = rustix::termios::tcgetpgrp(std::io::stdin()).ok();
    #[cfg(unix)]
    let signal_mask_before = nix::sys::signal::SigSet::thread_get_mask().unwrap();
    #[cfg(unix)]
    if std::env::var_os("CJR_TEST_REQUIRE_TTY").is_some() {
        assert!(
            foreground_before.is_some(),
            "PTY regression requires a controlling terminal"
        );
    }
    #[cfg(target_os = "linux")]
    let subreaper_before = rustix::process::child_subreaper().unwrap();
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("probe.rs");
    let binary = t
        .path()
        .join(if cfg!(windows) { "probe.exe" } else { "probe" });
    fs::write(&source,r#"fn main(){let a:Vec<_>=std::env::args_os().skip(1).collect();assert_eq!(a,vec![std::ffi::OsString::from("--help"),"--".into(),"a b".into(),"中文".into(),"quote\"backslash\\".into()]);std::process::exit(37);}"#).unwrap();
    let compiled = Command::new("rustc")
        .args(["--crate-name", "argument_probe"])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let plan = cjr::loader::LaunchPlan {
        binary,
        args: ["--help", "--", "a b", "中文", "quote\"backslash\\"]
            .map(Into::into)
            .to_vec(),
        environment: vec![],
        checked: vec![],
    };
    assert_eq!(
        cjr::process::launch(plan, None).unwrap(),
        cjr::process::Outcome::Exited(37)
    );

    // Keep process-wide supervision tests sequential in this integration process.
    let store = cjr::store::Store {
        root: t.path().join("cache"),
    };
    let guard = store.guard().unwrap();
    let id = "c".repeat(64);
    fs::create_dir(store.root.join("runtimes").join(&id)).unwrap();
    let lease = guard.begin_use(&id).unwrap();
    drop(guard);
    let missing = cjr::loader::LaunchPlan {
        binary: t.path().join("does-not-exist"),
        args: vec![],
        environment: vec![],
        checked: vec![],
    };
    assert!(cjr::process::launch(missing, Some(lease)).is_err());
    assert_eq!(store.usage(&id), cjr::store::UsageState::Unused);

    #[cfg(unix)]
    {
        // Returning a child's signal must not kill the library caller or reap a
        // child owned by another component in this process.
        let mut unrelated = Command::new("/bin/sh")
            .args(["-c", "exit 42"])
            .spawn()
            .unwrap();
        let plan = cjr::loader::LaunchPlan {
            binary: "/bin/sh".into(),
            args: ["-c", "kill -TERM $$"].map(Into::into).to_vec(),
            environment: vec![],
            checked: vec![],
        };
        assert_eq!(
            cjr::process::launch(plan, None).unwrap(),
            cjr::process::Outcome::Signaled(15)
        );
        assert_eq!(unrelated.wait().unwrap().code(), Some(42));
    }
    #[cfg(unix)]
    if let Some(previous) = foreground_before {
        assert_eq!(
            rustix::termios::tcgetpgrp(std::io::stdin()).unwrap(),
            previous
        );
    }
    #[cfg(unix)]
    assert_eq!(
        nix::sys::signal::SigSet::thread_get_mask().unwrap(),
        signal_mask_before
    );
    #[cfg(target_os = "linux")]
    assert_eq!(
        rustix::process::child_subreaper().unwrap(),
        subreaper_before
    );
}
