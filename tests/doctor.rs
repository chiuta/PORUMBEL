//! `porumbel doctor` exit codes: 0 when nothing blocks, 1 when something does.

use std::process::Command;

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("porumbel-doc-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn doctor(dir: &std::path::Path, extra: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_porumbel"))
        .arg("doctor")
        .arg("--config")
        .arg(dir.join("p.conf"))
        .arg("--inbox")
        .arg(dir.join("inbox"))
        .arg("--port")
        .arg("0")
        .args(extra)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

#[test]
fn doctor_ok_without_adb() {
    let d = tmp("ok");
    let (code, text) = doctor(&d, &[]);
    assert!(text.contains("inbox"), "{text}");
    // no network at all in a sandbox is a FAIL for `phone`; accept either, but the
    // exit code must agree with the report
    assert_eq!(code == 0, !text.contains("[FAIL]"), "{text}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn doctor_fails_when_adb_is_missing() {
    let d = tmp("noadb");
    let (code, text) = doctor(&d, &["--adb", "--adb-path", "/nonexistent/adb-for-sure"]);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("[FAIL] adb"), "{text}");
    let _ = std::fs::remove_dir_all(&d);
}
