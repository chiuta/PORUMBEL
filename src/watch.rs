//! Transport 2 — a folder that something else keeps in sync (Syncthing, a cloud client,
//! Samba share, …). Every finished image appearing there is copied (or moved) in.

use crate::hub::Hub;
use crate::util;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn scan(dir: &Path, exts: &[String], depth: u32, out: &mut Vec<(PathBuf, u64, u64)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue; // .stfolder, .syncthing.*.tmp, hidden temp files
        }
        // never follow symbolic links: a synced folder is not trusted to point inside itself
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let Ok(md) = e.metadata() else { continue };
        let p = e.path();
        if md.is_dir() {
            if depth < 3 {
                scan(&p, exts, depth + 1, out);
            }
        } else if md.is_file() && util::has_ext_in(&name, exts) {
            let mt = md
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            out.push((p, md.len(), mt));
        }
    }
}

type Seen = HashMap<PathBuf, (u64, u64)>;

pub fn pass(hub: &Hub, dir: &Path, prev: &mut Seen) {
    let mut files = Vec::new();
    scan(dir, &hub.cfg.extensions, 0, &mut files);
    let key = format!("watch:{}", dir.display());
    if !hub.cfg.watch_all && hub.baseline_needed(&key) {
        for (p, s, _) in &files {
            hub.mark_seen(&p.file_name().unwrap_or_default().to_string_lossy(), *s);
        }
        hub.baseline_done(&key);
        hub.log(format!(
            "watch {}: first run — {} existing files left alone (use --watch-all to import them)",
            dir.display(),
            files.len()
        ));
        return;
    }
    let mut now: Seen = HashMap::new();
    for (p, size, mt) in files {
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if size == 0 || hub.known(&name, size) {
            continue;
        }
        now.insert(p.clone(), (size, mt));
        if prev.get(&p) != Some(&(size, mt)) {
            continue; // new or still being written/synced
        }
        let tmp = hub.tmp_path("part");
        if let Err(e) = fs::copy(&p, &tmp) {
            hub.log(format!("watch: cannot read {}: {e}", p.display()));
            let _ = fs::remove_file(&tmp);
            continue;
        }
        if fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0) != size {
            let _ = fs::remove_file(&tmp);
            continue;
        }
        match hub.ingest(&tmp, &name, "folder") {
            Ok(_) => {
                now.remove(&p);
                if hub.cfg.watch_move {
                    if let Err(e) = fs::remove_file(&p) {
                        hub.log(format!("watch: cannot remove {}: {e}", p.display()));
                    }
                }
            }
            Err(e) => hub.log(format!("watch: {name}: {e}")),
        }
    }
    *prev = now;
}

pub fn run_loop(hub: &Hub, dir: PathBuf) {
    if !dir.is_dir() {
        hub.log(format!("watch: {} is not a folder", dir.display()));
        if hub.cfg.once {
            return;
        }
    }
    let mut prev: Seen = HashMap::new();
    loop {
        if hub.cfg.once {
            let mut files = Vec::new();
            scan(&dir, &hub.cfg.extensions, 0, &mut files);
            prev = files.into_iter().map(|(p, s, m)| (p, (s, m))).collect();
            std::thread::sleep(Duration::from_secs(1));
        }
        pass(hub, &dir, &mut prev);
        if hub.cfg.once {
            return;
        }
        std::thread::sleep(Duration::from_secs(hub.cfg.watch_interval));
    }
}
