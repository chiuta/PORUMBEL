//! The hub: receives finished photo files from every transport, files them into session
//! folders, remembers what was already received and decides when a session is complete.

use crate::config::{Config, OpenMode};
use crate::util::{self, json_str};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Entry {
    pub session: String, // "-" = seen but deliberately not imported (baseline)
    pub mtime: u64,
}

struct Cur {
    name: String,
    last: u64,
    count: usize,
}

#[derive(Default)]
struct Inner {
    index: HashMap<(String, u64), Entry>,
    current: Option<Cur>,
    log: VecDeque<String>,
    adb_state: String,
    opened: HashSet<String>,
    ready: VecDeque<String>, // closed sessions waiting to be opened (auto mode)
    counter: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Stored { path: PathBuf, session: String },
    Duplicate,
}

/// First bytes of the file agree with its extension (JPEG, PNG, TIFF, BMP).
fn looks_like_image(path: &Path, name: &str) -> bool {
    use std::io::Read;
    let mut head = [0u8; 8];
    let n = match fs::File::open(path).and_then(|mut f| f.read(&mut head)) {
        Ok(n) => n,
        Err(_) => return false,
    };
    let h = &head[..n];
    match util::ext_of(name).as_str() {
        "jpg" | "jpeg" => h.starts_with(&[0xFF, 0xD8, 0xFF]),
        "png" => h.starts_with(&[0x89, b'P', b'N', b'G']),
        "tif" | "tiff" => {
            h.starts_with(&[b'I', b'I', 0x2A, 0x00])
                || h.starts_with(&[b'M', b'M', 0x00, 0x2A])
                || h.starts_with(&[b'I', b'I', 0x2B, 0x00])
                || h.starts_with(&[b'M', b'M', 0x00, 0x2B])
        }
        "bmp" => h.starts_with(b"BM"),
        _ => true, // an extension the user added by hand: no opinion
    }
}

pub struct SessionInfo {
    pub name: String,
    pub count: usize,
    pub opened: bool,
    pub current: bool,
}

pub struct Hub {
    pub cfg: Config,
    pub lan: Vec<IpAddr>,
    pub port: std::sync::atomic::AtomicU16,
    state_dir: PathBuf,
    inner: Mutex<Inner>,
    /// serialises "is it known? pick a name, move, record" so that concurrent uploads of
    /// the same photo (or the same name) can neither duplicate nor overwrite
    ingest_lock: Mutex<()>,
}

impl Hub {
    pub fn new(mut cfg: Config) -> std::io::Result<Hub> {
        if cfg.inbox.is_relative() {
            cfg.inbox = std::env::current_dir()?.join(&cfg.inbox);
        }
        let state_dir = cfg.inbox.join(".porumbel");
        fs::create_dir_all(state_dir.join("tmp"))?;
        fs::create_dir_all(state_dir.join("stage"))?;
        // leftovers of an interrupted run
        for d in ["tmp", "stage"] {
            if let Ok(rd) = fs::read_dir(state_dir.join(d)) {
                for e in rd.flatten() {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
        let mut inner = Inner::default();
        if let Ok(text) = fs::read_to_string(state_dir.join("index.tsv")) {
            for l in text.lines() {
                let mut p = l.split('\t');
                if let (Some(n), Some(s), Some(se), Some(m)) =
                    (p.next(), p.next(), p.next(), p.next())
                {
                    if let (Ok(size), Ok(mt)) = (s.parse::<u64>(), m.parse::<u64>()) {
                        inner.index.insert(
                            (n.to_string(), size),
                            Entry {
                                session: se.into(),
                                mtime: mt,
                            },
                        );
                    }
                }
            }
        }
        if let Ok(text) = fs::read_to_string(state_dir.join("opened.txt")) {
            inner.opened = text.lines().map(|s| s.to_string()).collect();
        }
        let port = cfg.port;
        Ok(Hub {
            cfg,
            lan: util::lan_ips(),
            port: std::sync::atomic::AtomicU16::new(port),
            state_dir,
            inner: Mutex::new(inner),
            ingest_lock: Mutex::new(()),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    /// A fresh path for a file being received (same filesystem as the inbox).
    pub fn tmp_path(&self, tag: &str) -> PathBuf {
        let n = {
            let mut g = self.lock();
            g.counter += 1;
            g.counter
        };
        self.state_dir
            .join("tmp")
            .join(format!("{}-{n}.{tag}", util::hex(&util::random_bytes(4))))
    }

    pub fn stage_dir(&self) -> PathBuf {
        self.state_dir.join("stage")
    }

    pub fn log(&self, msg: impl AsRef<str>) {
        let line = format!(
            "{} {}",
            &util::stamp_utc(util::now_unix())[11..19],
            msg.as_ref()
        );
        eprintln!("{line}");
        let mut g = self.lock();
        g.log.push_back(line);
        while g.log.len() > 200 {
            g.log.pop_front();
        }
    }

    pub fn set_adb_state(&self, s: &str) {
        let changed = {
            let mut g = self.lock();
            let changed = g.adb_state != s;
            g.adb_state = s.to_string();
            changed
        };
        if changed {
            self.log(format!("ADB: {s}"));
        }
    }

    /// Already received (same name and size)? Used by ADB and folder watchers to avoid
    /// copying a file twice, and by the server to answer repeated uploads.
    pub fn known(&self, name: &str, size: u64) -> bool {
        match util::sanitize_name(name) {
            Some(n) => self.lock().index.contains_key(&(n, size)),
            None => false,
        }
    }

    /// Mark a file as seen without importing it (first-run baseline).
    pub fn mark_seen(&self, name: &str, size: u64) {
        if let Some(n) = util::sanitize_name(name) {
            self.record(&n, size, "-");
        }
    }

    fn record(&self, name: &str, size: u64, session: &str) {
        let now = util::now_unix();
        self.lock().index.insert(
            (name.to_string(), size),
            Entry {
                session: session.to_string(),
                mtime: now,
            },
        );
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.state_dir.join("index.tsv"))
        {
            let _ = writeln!(f, "{name}\t{size}\t{session}\t{now}");
        }
    }

    pub fn baseline_needed(&self, key: &str) -> bool {
        !self
            .state_dir
            .join(format!("baseline-{}", util::hex(key.as_bytes())))
            .exists()
    }

    pub fn baseline_done(&self, key: &str) {
        let _ = fs::write(
            self.state_dir
                .join(format!("baseline-{}", util::hex(key.as_bytes()))),
            b"",
        );
    }

    /// File a finished photo. `tmp` is moved (or copied, then removed) into the session folder.
    pub fn ingest(&self, tmp: &Path, raw_name: &str, source: &str) -> Result<Outcome, String> {
        let name = util::sanitize_name(raw_name).ok_or_else(|| "invalid file name".to_string())?;
        if !util::has_ext_in(&name, &self.cfg.extensions) {
            let _ = fs::remove_file(tmp);
            return Err(format!("file type not accepted: {name}"));
        }
        let size = fs::metadata(tmp).map_err(|e| e.to_string())?.len();
        if size == 0 {
            let _ = fs::remove_file(tmp);
            return Err(format!("empty file: {name}"));
        }
        if self.cfg.verify_content && !looks_like_image(tmp, &name) {
            let _ = fs::remove_file(tmp);
            return Err(format!("content does not match the file type: {name}"));
        }
        let _one_at_a_time = self.ingest_lock.lock().unwrap_or_else(|e| e.into_inner());
        if self.known(&name, size) {
            let _ = fs::remove_file(tmp);
            return Ok(Outcome::Duplicate);
        }
        let now = util::now_unix();
        let (sdir, sname) = {
            let mut g = self.lock();
            let rotate = match &g.current {
                Some(c) => now.saturating_sub(c.last) > self.cfg.session_gap,
                None => false,
            };
            if rotate {
                Self::close_locked(&mut g);
            }
            if g.current.is_none() {
                let base = util::parse_camera_stamp(&name).unwrap_or_else(|| util::stamp_utc(now));
                let mut sname = base.clone();
                let mut k = 2;
                while self.cfg.inbox.join(&sname).exists() {
                    sname = format!("{base}-{k}");
                    k += 1;
                }
                fs::create_dir_all(self.cfg.inbox.join(&sname)).map_err(|e| e.to_string())?;
                g.current = Some(Cur {
                    name: sname,
                    last: now,
                    count: 0,
                });
            }
            let c = g.current.as_ref().unwrap();
            (self.cfg.inbox.join(&c.name), c.name.clone())
        };
        // free name inside the session
        let mut dest = sdir.join(&name);
        if dest.exists() {
            let (stem, ext) = match name.rfind('.') {
                Some(p) => (&name[..p], &name[p..]),
                None => (name.as_str(), ""),
            };
            let mut k = 2;
            while dest.exists() {
                dest = sdir.join(format!("{stem}-{k}{ext}"));
                k += 1;
            }
        }
        if fs::rename(tmp, &dest).is_err() {
            fs::copy(tmp, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
            let _ = fs::remove_file(tmp);
        }
        self.record(&name, size, &sname);
        {
            let mut g = self.lock();
            if let Some(c) = g.current.as_mut() {
                c.last = util::now_unix();
                c.count += 1;
            }
        }
        self.log(format!(
            "+ {name} ({} KB) → {sname}  [{source}]",
            size.div_ceil(1024)
        ));
        Ok(Outcome::Stored {
            path: dest,
            session: sname,
        })
    }

    fn close_locked(g: &mut Inner) {
        if let Some(c) = g.current.take() {
            if c.count > 0 && !g.opened.contains(&c.name) {
                g.ready.push_back(c.name);
            }
        }
    }

    /// End the running session now (panel button): the next photo starts a new folder.
    pub fn close_current(&self) {
        let mut g = self.lock();
        Self::close_locked(&mut g);
    }

    /// Called every second. Closes an idle session and returns a session folder that
    /// should now be handed to LegatorOCR (auto mode).
    pub fn tick(&self) -> Option<String> {
        let now = util::now_unix();
        let mut g = self.lock();
        let idle =
            matches!(&g.current, Some(c) if now.saturating_sub(c.last) >= self.cfg.session_gap);
        if idle {
            Self::close_locked(&mut g);
        }
        if self.cfg.open == OpenMode::Auto {
            g.ready.pop_front()
        } else {
            g.ready.clear();
            None
        }
    }

    pub fn mark_opened(&self, name: &str) {
        let mut g = self.lock();
        g.opened.insert(name.to_string());
        g.ready.retain(|n| n != name);
        let text: String = g.opened.iter().map(|s| format!("{s}\n")).collect();
        drop(g);
        let _ = fs::write(self.state_dir.join("opened.txt"), text);
    }

    /// Session folders on disk, newest first.
    pub fn sessions(&self) -> Vec<SessionInfo> {
        let g = self.lock();
        let cur = g.current.as_ref().map(|c| c.name.clone());
        let mut v = Vec::new();
        if let Ok(rd) = fs::read_dir(&self.cfg.inbox) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || !e.path().is_dir() {
                    continue;
                }
                let count = fs::read_dir(e.path())
                    .map(|r| {
                        r.flatten()
                            .filter(|f| {
                                util::has_ext_in(
                                    &f.file_name().to_string_lossy(),
                                    &self.cfg.extensions,
                                )
                            })
                            .count()
                    })
                    .unwrap_or(0);
                v.push(SessionInfo {
                    opened: g.opened.contains(&name),
                    current: cur.as_deref() == Some(name.as_str()),
                    name,
                    count,
                });
            }
        }
        v.sort_by(|a, b| b.name.cmp(&a.name));
        v
    }

    /// Resolve a session name from outside (panel, CLI) to a folder, only if it really is
    /// a session folder directly inside the inbox.
    pub fn session_dir(&self, name: &str) -> Option<PathBuf> {
        if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
            return None;
        }
        let p = self.cfg.inbox.join(name);
        if p.is_dir() {
            Some(p)
        } else {
            None
        }
    }

    pub fn latest_session(&self) -> Option<String> {
        self.sessions()
            .into_iter()
            .find(|s| s.count > 0)
            .map(|s| s.name)
    }

    /// Where the file for `name` lives (for GET on the WebDAV side).
    pub fn find_file(&self, name: &str) -> Option<PathBuf> {
        let g = self.lock();
        let mut best: Option<(&(String, u64), &Entry)> = None;
        for (k, v) in g.index.iter() {
            if k.0 == name && v.session != "-" && best.map_or(true, |(_, b)| v.mtime > b.mtime) {
                best = Some((k, v));
            }
        }
        let (k, v) = best?;
        let p = self.cfg.inbox.join(&v.session).join(&k.0);
        p.is_file().then_some(p)
    }

    /// Entries for the WebDAV listing: (name, size, mtime), imported files only.
    pub fn listing(&self) -> Vec<(String, u64, u64)> {
        let g = self.lock();
        let mut v: Vec<_> = g
            .index
            .iter()
            .filter(|(_, e)| e.session != "-")
            .map(|(k, e)| (k.0.clone(), k.1, e.mtime))
            .collect();
        v.sort();
        v.truncate(20_000);
        v
    }

    pub fn entry(&self, name: &str) -> Option<(u64, u64)> {
        let g = self.lock();
        g.index
            .iter()
            .filter(|(k, e)| k.0 == name && e.session != "-")
            .map(|(k, e)| (k.1, e.mtime))
            .max_by_key(|x| x.1)
    }

    pub fn forget(&self) {
        {
            let mut g = self.lock();
            g.index.clear();
            g.opened.clear();
            g.ready.clear();
        }
        if let Ok(rd) = fs::read_dir(&self.state_dir) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n == "index.tsv" || n == "opened.txt" || n.starts_with("baseline-") {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
        self.log("history cleared");
    }

    pub fn phone_url(&self) -> Option<String> {
        let ip = self.lan.first()?;
        Some(format!(
            "http://{}:{}/?t={}",
            ip,
            self.port.load(std::sync::atomic::Ordering::Relaxed),
            self.cfg.token
        ))
    }

    pub fn status_json(&self, launcher: &str) -> String {
        let sessions = self.sessions();
        let g = self.lock();
        let port = self.port.load(std::sync::atomic::Ordering::Relaxed);
        let mut s = String::from("{");
        s.push_str(&format!(
            "\"version\":{},",
            json_str(env!("CARGO_PKG_VERSION"))
        ));
        s.push_str(&format!(
            "\"inbox\":{},",
            json_str(&self.cfg.inbox.display().to_string())
        ));
        s.push_str(&format!(
            "\"open\":{},",
            json_str(if self.cfg.open == OpenMode::Auto {
                "auto"
            } else {
                "manual"
            })
        ));
        s.push_str(&format!("\"session_gap\":{},", self.cfg.session_gap));
        s.push_str(&format!("\"launcher\":{},", json_str(launcher)));
        s.push_str(&format!(
            "\"adb\":{},",
            json_str(if self.cfg.adb { &g.adb_state } else { "" })
        ));
        s.push_str(&format!("\"adb_on\":{},", self.cfg.adb));
        s.push_str(&format!("\"watch\":{},", self.cfg.watch.len()));
        s.push_str(&format!(
            "\"phone_url\":{},",
            json_str(&self.phone_url().unwrap_or_default())
        ));
        s.push_str(&format!(
            "\"webdav\":{},",
            json_str(
                &self
                    .lan
                    .first()
                    .map(|ip| format!("http://{ip}:{port}/dav/"))
                    .unwrap_or_default()
            )
        ));
        s.push_str(&format!("\"max_upload_mb\":{},", self.cfg.max_upload_mb));
        s.push_str("\"sessions\":[");
        for (i, x) in sessions.iter().take(50).enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"name\":{},\"count\":{},\"opened\":{},\"current\":{}}}",
                json_str(&x.name),
                x.count,
                x.opened,
                x.current
            ));
        }
        s.push_str("],\"log\":[");
        for (i, l) in g.log.iter().rev().take(60).enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&json_str(l));
        }
        s.push_str("]}");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub(gap: u64) -> (Hub, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "porumbel-hub-{}-{}",
            std::process::id(),
            util::hex(&util::random_bytes(4))
        ));
        let mut cfg = Config::default();
        cfg.inbox = dir.clone();
        cfg.session_gap = gap;
        cfg.verify_content = false;
        (Hub::new(cfg).unwrap(), dir)
    }

    fn put(h: &Hub, name: &str, data: &[u8]) -> Result<Outcome, String> {
        let t = h.tmp_path("part");
        fs::write(&t, data).unwrap();
        h.ingest(&t, name, "test")
    }

    #[test]
    fn stores_dedupes_and_sessions() {
        let (h, dir) = hub(1000);
        let r = put(&h, "IMG_20261114_135501.jpg", b"aaaa").unwrap();
        let Outcome::Stored { path, session } = r else {
            panic!()
        };
        assert_eq!(session, "2026-11-14_13-55-01");
        assert_eq!(fs::read(path).unwrap(), b"aaaa");
        // same name+size again: duplicate, nothing new
        assert_eq!(
            put(&h, "IMG_20261114_135501.jpg", b"bbbb").unwrap(),
            Outcome::Duplicate
        );
        // same name, different size: stored with a suffix
        let Outcome::Stored { path, .. } = put(&h, "IMG_20261114_135501.jpg", b"cc").unwrap()
        else {
            panic!()
        };
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with("-2.jpg"));
        // refused types and traversal
        assert!(put(&h, "x.exe", b"1").is_err());
        assert!(put(&h, "empty.jpg", b"").is_err());
        let Outcome::Stored { path, .. } = put(&h, "../../evil.jpg", b"zz").unwrap() else {
            panic!()
        };
        assert!(path.starts_with(&dir) && path.ends_with("evil.jpg"));
        // index survives a restart
        let mut cfg = h.cfg.clone();
        cfg.inbox = dir.clone();
        let h2 = Hub::new(cfg).unwrap();
        assert!(h2.known("IMG_20261114_135501.jpg", 4));
        assert_eq!(h2.listing().len(), 3);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn idle_session_becomes_ready_once() {
        let (h, dir) = hub(0);
        put(&h, "a.jpg", b"1").unwrap();
        let ready = h.tick();
        assert!(ready.is_some());
        assert!(h.tick().is_none());
        h.mark_opened(ready.as_deref().unwrap());
        // a new photo after the gap starts a new session
        put(&h, "b.jpg", b"22").unwrap();
        assert_eq!(h.sessions().len(), 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn session_names_are_validated() {
        let (h, dir) = hub(1000);
        put(&h, "a.jpg", b"1").unwrap();
        assert!(h.session_dir("..").is_none());
        assert!(h.session_dir("a/b").is_none());
        assert!(h.session_dir(".porumbel").is_none());
        assert!(h.session_dir(&h.latest_session().unwrap()).is_some());
        let _ = fs::remove_dir_all(dir);
    }
}
