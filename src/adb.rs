//! Transport 1 — USB / Wi-Fi debugging bridge. Polls the phone's camera folder with
//! `adb`, pulls every finished photo and files it. Needs `adb` (Android platform-tools)
//! and USB debugging on the phone (Android 6 or newer).

use crate::hub::Hub;
use crate::util;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn adb_cmd(hub: &Hub) -> Command {
    let mut c = Command::new(&hub.cfg.adb_path);
    if let Some(s) = &hub.cfg.adb_serial {
        c.arg("-s").arg(s);
    }
    c.stdin(Stdio::null());
    #[cfg(windows)]
    c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    c
}

/// Run a command, but never wait longer than `secs`: a wedged `adb` server (phone
/// unplugged mid-copy, sleeping USB hub…) must not freeze PORUMBEL.
fn run(mut c: Command, secs: u64) -> Result<String, String> {
    use std::io::Read;
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = c
        .spawn()
        .map_err(|e| format!("cannot run adb ({e}); install platform-tools or set adb_path"))?;
    let mut out = child.stdout.take().ok_or("no stdout")?;
    let mut err = child.stderr.take().ok_or("no stderr")?;
    let t_out = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let t_err = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let deadline = Instant::now() + Duration::from_secs(secs);
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("adb did not answer within {secs} s"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(e) => return Err(e.to_string()),
        }
    };
    let so = String::from_utf8_lossy(&t_out.join().unwrap_or_default()).to_string();
    let se = String::from_utf8_lossy(&t_err.join().unwrap_or_default()).to_string();
    if !status.success() {
        let msg = if se.trim().is_empty() {
            so.trim()
        } else {
            se.trim()
        };
        return Err(msg.lines().next().unwrap_or("adb failed").to_string());
    }
    Ok(so.replace('\r', ""))
}

/// Single-quote for the phone's shell.
fn shq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// First line of `adb version` (used by `porumbel doctor`).
pub fn version(hub: &Hub) -> Result<String, String> {
    let mut c = Command::new(&hub.cfg.adb_path);
    c.arg("version").stdin(Stdio::null());
    #[cfg(windows)]
    c.creation_flags(0x0800_0000);
    let t = run(c, hub.cfg.adb_timeout)?;
    Ok(t.lines().next().unwrap_or("adb").trim().to_string())
}

/// Is one phone connected and authorised? Returns its serial.
pub fn device(hub: &Hub) -> Result<String, String> {
    let mut c = Command::new(&hub.cfg.adb_path);
    c.arg("devices").stdin(Stdio::null());
    #[cfg(windows)]
    c.creation_flags(0x0800_0000);
    let text = run(c, hub.cfg.adb_timeout)?;
    let mut found: Option<String> = None;
    let mut other = String::new();
    for l in text.lines().skip(1) {
        let mut p = l.split_whitespace();
        let (Some(serial), Some(state)) = (p.next(), p.next()) else {
            continue;
        };
        if let Some(want) = &hub.cfg.adb_serial {
            if want != serial {
                continue;
            }
        }
        match state {
            "device" => {
                found = Some(serial.to_string());
                break;
            }
            "unauthorized" => {
                other = "phone not authorised: accept the USB debugging prompt on the phone".into()
            }
            s => other = format!("phone state: {s}"),
        }
    }
    found.ok_or(if other.is_empty() {
        "no phone connected".to_string()
    } else {
        other
    })
}

/// (remote path, size) of the files in the camera folder.
pub fn list_remote(hub: &Hub) -> Result<Vec<(String, u64)>, String> {
    let script = format!(
        "find {} -maxdepth 1 -type f -exec stat -c '%s %n' {{}} +",
        shq(hub.cfg.phone_dir.trim_end_matches('/'))
    );
    let mut c = adb_cmd(hub);
    c.arg("shell").arg(script);
    let text = run(c, hub.cfg.adb_timeout)?;
    let dir = hub.cfg.phone_dir.trim_end_matches('/');
    let mut v = Vec::new();
    for l in text.lines() {
        let Some((s, p)) = l.split_once(' ') else {
            continue;
        };
        let Ok(size) = s.trim().parse::<u64>() else {
            continue;
        };
        // Only direct children of the camera folder. A file name containing a newline can
        // make `stat` print something that looks like another entry; a forged line must
        // not be able to point anywhere else.
        let Some(base) = p.strip_prefix(dir).and_then(|r| r.strip_prefix('/')) else {
            continue;
        };
        if base.is_empty() || base.contains('/') || base == ".." || base.starts_with('.') {
            continue;
        }
        if !util::has_ext_in(base, &hub.cfg.extensions) {
            continue;
        }
        v.push((p.to_string(), size));
    }
    Ok(v)
}

fn pull(hub: &Hub, remote: &str, local: &std::path::Path) -> Result<(), String> {
    let mut c = adb_cmd(hub);
    c.arg("pull").arg(remote).arg(local);
    run(c, hub.cfg.adb_timeout.saturating_mul(10)).map(|_| ())
}

/// One polling pass. `prev` holds the sizes seen on the previous pass: a file is taken
/// only when its size did not change since (Open Camera is done writing it).
pub fn pass(hub: &Hub, prev: &mut HashMap<String, u64>) -> Result<(), String> {
    let serial = device(hub)?;
    hub.set_adb_state(&format!("connected ({serial})"));
    let files = list_remote(hub)?;
    let base = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();

    if !hub.cfg.adb_all && hub.baseline_needed("adb") {
        for (p, s) in &files {
            hub.mark_seen(&base(p), *s);
        }
        hub.baseline_done("adb");
        hub.log(format!(
            "ADB: first run — {} existing photos left alone (use --adb-all to import them)",
            files.len()
        ));
        return Ok(());
    }

    let mut now: HashMap<String, u64> = HashMap::new();
    for (path, size) in files {
        let name = base(&path);
        if size == 0 || hub.known(&name, size) {
            continue;
        }
        now.insert(path.clone(), size);
        if prev.get(&path) != Some(&size) {
            continue; // seen for the first time or still growing
        }
        let tmp = hub.tmp_path("part");
        if let Err(e) = pull(hub, &path, &tmp) {
            hub.log(format!("ADB: cannot pull {name}: {e}"));
            let _ = std::fs::remove_file(&tmp);
            continue;
        }
        let got = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
        if got != size {
            let _ = std::fs::remove_file(&tmp);
            continue; // changed while copying: next pass
        }
        match hub.ingest(&tmp, &name, "ADB") {
            Ok(_) => {
                now.remove(&path);
                if hub.cfg.adb_delete {
                    let mut c = adb_cmd(hub);
                    c.arg("shell").arg(format!("rm -f -- {}", shq(&path)));
                    if let Err(e) = run(c, hub.cfg.adb_timeout) {
                        hub.log(format!("ADB: cannot delete {name} on the phone: {e}"));
                    }
                }
            }
            Err(e) => hub.log(format!("ADB: {name}: {e}")),
        }
    }
    *prev = now;
    Ok(())
}

pub fn run_loop(hub: &Hub) {
    let mut prev = HashMap::new();
    loop {
        if hub.cfg.once {
            // two looks one second apart stand in for the polling history
            if let Ok(l) = list_remote(hub) {
                prev = l.into_iter().collect();
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        if let Err(e) = pass(hub, &mut prev) {
            hub.set_adb_state(&e);
        }
        if hub.cfg.once {
            return;
        }
        std::thread::sleep(Duration::from_secs(hub.cfg.adb_interval));
    }
}
