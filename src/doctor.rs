//! `porumbel doctor` — one screen that says what works and what does not, so that a first
//! test on a real phone does not start with guessing.

use crate::hub::Hub;
use crate::{adb, launch};
use std::net::TcpListener;
use std::path::Path;

fn line(tag: &str, what: &str, detail: &str) {
    println!("[{tag:^4}] {what:<12} {detail}");
}

/// Print the report. Returns the number of FAIL lines.
pub fn run(hub: &Hub, conf: &Path) -> usize {
    let cfg = &hub.cfg;
    let mut bad = 0;
    let mut fail = |what: &str, d: &str| {
        bad += 1;
        line("FAIL", what, d)
    };

    println!("PORUMBEL {} — diagnostic\n", env!("CARGO_PKG_VERSION"));
    line("ok", "settings", &conf.display().to_string());

    // inbox must be writable
    let probe = cfg.inbox.join(".porumbel-doctor");
    match std::fs::write(&probe, b"x").and_then(|_| std::fs::remove_file(&probe)) {
        Ok(()) => line("ok", "inbox", &cfg.inbox.display().to_string()),
        Err(e) => fail(
            "inbox",
            &format!("{} is not writable: {e}", cfg.inbox.display()),
        ),
    }

    if cfg.token.len() < 16 {
        line(
            "warn",
            "token",
            &format!("only {} characters; use 16 or more", cfg.token.len()),
        );
    } else {
        line("ok", "token", "long enough");
    }

    if cfg.serve {
        match TcpListener::bind((cfg.bind.as_str(), cfg.port)) {
            Ok(_) => line("ok", "port", &format!("{}:{} is free", cfg.bind, cfg.port)),
            Err(e) => line(
                "warn",
                "port",
                &format!(
                    "{}:{} not free ({e}) — is PORUMBEL already running?",
                    cfg.bind, cfg.port
                ),
            ),
        }
        match hub.phone_url() {
            Some(u) => {
                let shown = u.split("?t=").next().unwrap_or(&u).to_string();
                line("ok", "phone", &format!("{shown}  (porumbel qr)"));
            }
            None => fail(
                "phone",
                "no local network address found; connect to Wi-Fi/LAN",
            ),
        }
    } else {
        line("--", "server", "disabled (serve = false)");
    }

    let lg = launch::describe(cfg);
    if lg.is_empty() {
        line(
            "warn",
            "LegatorOCR",
            "not found; the session folder will be opened instead (set `launch =`)",
        );
    } else {
        line("ok", "LegatorOCR", &lg);
    }

    if cfg.adb {
        match adb::version(hub) {
            Ok(v) => {
                line("ok", "adb", &v);
                match adb::device(hub) {
                    Ok(s) => {
                        line("ok", "phone (adb)", &format!("connected: {s}"));
                        match adb::list_remote(hub) {
                            Ok(f) => line(
                                "ok",
                                "phone_dir",
                                &format!("{}: {} photo(s) visible", cfg.phone_dir, f.len()),
                            ),
                            Err(e) => fail("phone_dir", &format!("{}: {e}", cfg.phone_dir)),
                        }
                    }
                    Err(e) => fail("phone (adb)", &e),
                }
            }
            Err(e) => fail("adb", &e),
        }
    } else {
        line("--", "adb", "off (enable with --adb)");
    }

    for d in &cfg.watch {
        if d.is_dir() {
            line("ok", "watch", &d.display().to_string());
        } else {
            fail("watch", &format!("{} is not a folder", d.display()));
        }
    }

    println!();
    if bad == 0 {
        println!("Nothing blocks PORUMBEL.");
    } else {
        println!("{bad} problem(s) found.");
    }
    bad
}
