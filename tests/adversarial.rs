//! Hostile-input tests: what a curious guest on the Wi-Fi, a broken sync app or a
//! booby-trapped phone/folder could try.
#![allow(clippy::field_reassign_with_default, clippy::ptr_arg)]

use porumbel::config::Config;
use porumbel::hub::Hub;
use porumbel::{server, util};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const AUTH: &str = "Authorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n"; // p:s3cret-token

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "porumbel-adv-{tag}-{}-{}",
        std::process::id(),
        util::hex(&util::random_bytes(4))
    ));
    fs::create_dir_all(&d).unwrap();
    d
}

fn cfg(inbox: &Path) -> Config {
    let mut c = Config::default();
    c.inbox = inbox.to_path_buf();
    c.token = "s3cret-token".into();
    c.bind = "127.0.0.1".into();
    c.port = 0;
    c.session_gap = 3600;
    c
}

fn start(c: Config) -> (Arc<Hub>, SocketAddr) {
    let hub = Arc::new(Hub::new(c).unwrap());
    let a = server::start(hub.clone()).unwrap();
    (hub, SocketAddr::from(([127, 0, 0, 1], a.port())))
}

fn jpeg(extra: usize, seed: u8) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0];
    v.extend(std::iter::repeat(seed).take(extra));
    v
}

fn http(addr: SocketAddr, raw: &[u8]) -> (u16, String, Vec<u8>) {
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
    let _ = s.write_all(raw);
    let mut buf = Vec::new();
    let _ = s.read_to_end(&mut buf);
    parse(&buf)
}

fn parse(buf: &[u8]) -> (u16, String, Vec<u8>) {
    let Some(split) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
        return (0, String::new(), buf.to_vec());
    };
    let head = String::from_utf8_lossy(&buf[..split]).to_string();
    let code: u16 = head
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    (code, head, buf[split + 4..].to_vec())
}

fn put(addr: SocketAddr, path: &str, body: &[u8]) -> u16 {
    let mut raw = format!(
        "PUT {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Length: {}\r\n{AUTH}\r\n",
        body.len()
    )
    .into_bytes();
    raw.extend_from_slice(body);
    http(addr, &raw).0
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if p.is_dir() {
                out.extend(files_under(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

fn tmp_leftovers(inbox: &Path) -> usize {
    fs::read_dir(inbox.join(".porumbel/tmp"))
        .map(|r| r.count())
        .unwrap_or(0)
}

#[test]
fn concurrent_identical_uploads_store_one_copy() {
    let dir = tmpdir("same");
    let (hub, a) = start(cfg(&dir));
    let body = jpeg(3000, 1);
    let hs: Vec<_> = (0..24)
        .map(|_| {
            let b = body.clone();
            std::thread::spawn(move || put(a, "/dav/IMG_20261116_100000.jpg", &b))
        })
        .collect();
    for h in hs {
        let c = h.join().unwrap();
        assert!(c == 201 || c == 204, "status {c}");
    }
    assert_eq!(files_under(&dir).len(), 1, "{:?}", files_under(&dir));
    assert_eq!(hub.sessions().len(), 1);
    assert_eq!(tmp_leftovers(&dir), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn concurrent_same_name_different_content_never_overwrites() {
    let dir = tmpdir("clash");
    let (_hub, a) = start(cfg(&dir));
    let hs: Vec<_> = (0..24usize)
        .map(|i| {
            let b = jpeg(100 + i, i as u8); // different sizes
            std::thread::spawn(move || put(a, "/dav/IMG_20261116_100100.jpg", &b))
        })
        .collect();
    for h in hs {
        assert_eq!(h.join().unwrap(), 201);
    }
    let files = files_under(&dir);
    assert_eq!(
        files.len(),
        24,
        "some upload was overwritten: {}",
        files.len()
    );
    let mut sizes: Vec<u64> = files
        .iter()
        .map(|f| fs::metadata(f).unwrap().len())
        .collect();
    sizes.sort();
    sizes.dedup();
    assert_eq!(sizes.len(), 24);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn path_tricks_never_leave_the_inbox() {
    let root = tmpdir("trav");
    let inbox = root.join("inbox");
    let (_hub, a) = start(cfg(&inbox));
    let names = [
        "..%2F..%2Fevil1.jpg",
        "%2e%2e/%2e%2e/evil2.jpg",
        "..%5C..%5Cevil3.jpg",
        "a/../../evil4.jpg",
        "%00evil5.jpg",
        "evil6.jpg%00.exe",
        "....//evil7.jpg",
        "C%3A%5CWindows%5Cevil8.jpg",
        "%E2%80%AEgpj.exe.jpg", // right-to-left override
        "CON.jpg",
        "con%20.jpg",
        "NUL",
        "..",
        "%2e",
    ];
    for n in names {
        let _ = put(a, &format!("/dav/{n}"), &jpeg(50, 2));
    }
    // everything that was stored sits inside the inbox, nothing outside it
    for f in files_under(&root) {
        assert!(f.starts_with(&inbox), "escaped: {}", f.display());
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        assert!(!name.contains(['/', '\\', ':', '\0', '\u{202e}']), "{name}");
        let stem = name.split('.').next().unwrap().trim().to_ascii_uppercase();
        assert!(
            !matches!(stem.as_str(), "CON" | "NUL" | "PRN" | "AUX"),
            "reserved: {name}"
        );
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn fake_images_are_refused() {
    let dir = tmpdir("magic");
    let (hub, a) = start(cfg(&dir));
    assert_eq!(put(a, "/dav/script.jpg", b"#!/bin/sh\nrm -rf ~\n"), 415);
    assert_eq!(put(a, "/dav/exe.png", b"MZ\x90\x00 not a png"), 415);
    assert_eq!(put(a, "/dav/tiny.jpg", b"\xFF"), 415);
    assert_eq!(put(a, "/dav/real.jpg", &jpeg(10, 1)), 201);
    assert_eq!(put(a, "/dav/real.png", b"\x89PNG\r\n\x1a\n....."), 201);
    assert_eq!(put(a, "/dav/real.tif", b"II*\x00....."), 201);
    assert_eq!(put(a, "/dav/real.bmp", b"BM......"), 201);
    assert_eq!(files_under(&dir).len(), 4);
    assert_eq!(tmp_leftovers(&dir), 0);
    drop(hub);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn hostile_names_are_inert_in_every_output() {
    let dir = tmpdir("xss");
    let (_hub, a) = start(cfg(&dir));
    assert_eq!(
        put(
            a,
            "/dav/%3Cimg%20src%3Dx%20onerror%3Dalert(1)%3E.jpg",
            &jpeg(10, 3)
        ),
        201
    );
    assert_eq!(
        put(
            a,
            "/dav/%22%3E%3Cscript%3Ex%3C%2Fscript%3E%26amp.jpg",
            &jpeg(11, 3)
        ),
        201
    );
    for (path, extra) in [("/api/status", ""), ("/dav/", "Depth: 1\r\n")] {
        let m = if path == "/dav/" { "PROPFIND" } else { "GET" };
        let (code, _, body) = http(
            a,
            format!("{m} {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n{extra}{AUTH}\r\n")
                .as_bytes(),
        );
        assert!(code == 200 || code == 207);
        let t = String::from_utf8_lossy(&body);
        assert!(
            !t.contains("<img") && !t.contains("<script") && !t.contains("onerror=alert(1)>"),
            "{t}"
        );
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn garbage_does_not_hurt_the_server() {
    let dir = tmpdir("fuzz");
    let (_hub, a) = start(cfg(&dir));
    let mut seed = 0x1234_5678u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let starts: [&[u8]; 8] = [
        b"",
        b"GET ",
        b"PUT /dav/a.jpg HTTP/1.1\r\n",
        b"POST /api/open HTTP/1.1\r\n",
        b"PROPFIND / HTTP/1.1\r\n",
        b"GET / HTTP/9.9\r\n",
        b"\r\n\r\n",
        b"PUT /dav/a.jpg HTTP/1.1\r\nTransfer-Encoding: chunked\r\n",
    ];
    for i in 0..200 {
        let mut raw = starts[i % starts.len()].to_vec();
        for _ in 0..(rnd() % 300) {
            raw.push((rnd() & 0xFF) as u8);
        }
        let mut s = TcpStream::connect(a).unwrap();
        s.set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
        let _ = s.write_all(&raw);
        let _ = s.shutdown(std::net::Shutdown::Write);
        let mut sink = Vec::new();
        let _ = s.read_to_end(&mut sink);
    }
    // header and length abuses
    let big_header = format!("GET / HTTP/1.1\r\nX: {}\r\n\r\n", "a".repeat(100_000));
    assert_ne!(http(a, big_header.as_bytes()).0, 200);
    let many: String = (0..500).map(|i| format!("H{i}: v\r\n")).collect();
    assert_ne!(
        http(a, format!("GET / HTTP/1.1\r\n{many}\r\n").as_bytes()).0,
        200
    );
    for cl in [
        "18446744073709551615",
        "-1",
        "99999999999999999999999",
        "1e3",
        "0x10",
        "",
    ] {
        let (c, _, _) = http(a, format!("PUT /dav/a.jpg HTTP/1.1\r\nConnection: close\r\nContent-Length: {cl}\r\n{AUTH}\r\n").as_bytes());
        assert!(
            matches!(c, 400 | 413 | 201 | 204 | 415 | 0),
            "Content-Length {cl:?} -> {c}"
        );
    }
    // still alive and correct
    assert_eq!(put(a, "/dav/ok.jpg", &jpeg(5, 1)), 201);
    assert_eq!(files_under(&dir).len(), 1);
    assert_eq!(tmp_leftovers(&dir), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn aborted_upload_leaves_nothing_behind() {
    let dir = tmpdir("abort");
    let (hub, a) = start(cfg(&dir));
    {
        let mut s = TcpStream::connect(a).unwrap();
        s.write_all(
            format!(
                "PUT /dav/IMG_20261116_110000.jpg HTTP/1.1\r\nContent-Length: 100000\r\n{AUTH}\r\n"
            )
            .as_bytes(),
        )
        .unwrap();
        s.write_all(&jpeg(500, 1)).unwrap();
        // connection dropped here
    }
    std::thread::sleep(Duration::from_millis(500));
    assert!(files_under(&dir).is_empty());
    assert!(hub.sessions().is_empty());
    assert_eq!(tmp_leftovers(&dir), 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn auth_edge_cases() {
    let dir = tmpdir("auth");
    let (_hub, a) = start(cfg(&dir));
    let get = |extra: &str, q: &str| {
        http(
            a,
            format!("GET /api/status{q} HTTP/1.1\r\nConnection: close\r\n{extra}\r\n").as_bytes(),
        )
        .0
    };
    assert_eq!(get("", "?t=s3cret-token"), 200);
    assert_eq!(get("", "?t="), 401);
    assert_eq!(get("", "?t=s3cret-toke"), 401);
    assert_eq!(get("", "?t=s3cret-token2"), 401);
    assert_eq!(get("Authorization: Basic !!!notbase64!!!\r\n", ""), 401);
    assert_eq!(get("Authorization: Basic \r\n", ""), 401);
    assert_eq!(get("Authorization: Bearer s3cret-token\r\n", ""), 401);
    assert_eq!(get("Authorization: Basic dG9rZW4=\r\n", ""), 401); // "token"
    assert_eq!(get("X-Porumbel-Token: s3cret-token\r\n", ""), 200);
    drop(dir);
    // an empty configured token must lock everything, never open it
    let d2 = tmpdir("auth2");
    let mut c = cfg(&d2);
    c.token = String::new();
    let (_h, a2) = start(c);
    for q in ["?t=", "", "?t=x"] {
        let (code, _, _) = http(a2, format!("GET /api/status{q} HTTP/1.1\r\nConnection: close\r\nAuthorization: Basic Og==\r\nX-Porumbel-Token: \r\n\r\n").as_bytes());
        assert_eq!(code, 401, "empty token accepted for {q:?}");
    }
    let _ = fs::remove_dir_all(d2);
}

#[test]
fn pipelined_requests_and_head() {
    let dir = tmpdir("pipe");
    let (_hub, a) = start(cfg(&dir));
    assert_eq!(put(a, "/dav/IMG_1.jpg", &jpeg(20, 1)), 201);
    let two = format!("GET /api/status HTTP/1.1\r\n{AUTH}\r\nHEAD /dav/IMG_1.jpg HTTP/1.1\r\nConnection: close\r\n{AUTH}\r\n");
    let mut s = TcpStream::connect(a).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    s.write_all(two.as_bytes()).unwrap();
    let mut buf = Vec::new();
    let _ = s.read_to_end(&mut buf);
    let t = String::from_utf8_lossy(&buf).to_string();
    assert_eq!(t.matches("HTTP/1.1 200 OK").count(), 2, "{t}");
    assert!(t.ends_with("\r\n\r\n"), "HEAD must not carry a body: {t:?}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn idle_connections_cannot_lock_everyone_out() {
    let dir = tmpdir("idle");
    let (_hub, a) = start(cfg(&dir));
    // a guest opens many connections and says nothing
    let idle: Vec<TcpStream> = (0..40).filter_map(|_| TcpStream::connect(a).ok()).collect();
    std::thread::sleep(Duration::from_millis(200));
    // the legitimate phone must still get through right away
    assert_eq!(
        put(a, "/dav/legit.jpg", &jpeg(10, 1)),
        201,
        "server saturated by idle sockets"
    );
    drop(idle);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn silent_connection_is_dropped_quickly() {
    let dir = tmpdir("slow");
    let (_hub, a) = start(cfg(&dir));
    let mut s = TcpStream::connect(a).unwrap();
    s.write_all(b"GET / HT").unwrap(); // slowloris: never finishes
    s.set_read_timeout(Some(Duration::from_secs(14))).unwrap();
    let t0 = std::time::Instant::now();
    let mut b = [0u8; 64];
    let _ = s.read(&mut b); // returns when the server hangs up (or its 400)
    assert!(
        t0.elapsed() < Duration::from_secs(13),
        "server waited {:?}",
        t0.elapsed()
    );
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
mod unix_only {
    use super::*;
    use porumbel::{adb, watch};
    use std::collections::HashMap;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn symlinks_in_a_synced_folder_are_not_followed() {
        let root = tmpdir("sym");
        let sync = root.join("sync");
        fs::create_dir_all(&sync).unwrap();
        let secret = root.join("secret.png");
        fs::write(&secret, b"\x89PNG\r\n\x1a\nTOP SECRET PRIVATE DATA").unwrap();
        symlink(&secret, sync.join("looks-innocent.png")).unwrap();
        symlink(&root, sync.join("loop")).unwrap(); // directory loop
        let mut c = cfg(&root.join("inbox"));
        c.watch_all = true;
        let hub = Hub::new(c).unwrap();
        let mut prev = HashMap::new();
        for _ in 0..3 {
            watch::pass(&hub, &sync, &mut prev);
        }
        assert!(
            files_under(&root.join("inbox")).is_empty(),
            "{:?}",
            files_under(&root.join("inbox"))
        );
        let _ = fs::remove_dir_all(root);
    }

    /// The phone's toybox `stat -c '%s %n'` does not exist on macOS (BSD stat): provide a
    /// portable stand-in first on the PATH of the pretend phone.
    fn stat_shim(dir: &Path) -> String {
        let d = dir.join("shim");
        fs::create_dir_all(&d).unwrap();
        let p = d.join("stat");
        fs::write(&p, "#!/bin/sh\nshift 2\nfor f in \"$@\"; do printf '%s %s\\n' \"$(wc -c < \"$f\" | tr -d ' ')\" \"$f\"; done\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        d.display().to_string()
    }

    fn fake_adb(dir: &Path, body: &str) -> String {
        let shim = stat_shim(dir);
        let p = dir.join("adb");
        fs::write(&p, format!("#!/bin/sh\nPATH='{shim}':\"$PATH\"\n{body}\n")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        p.display().to_string()
    }

    #[test]
    fn a_booby_trapped_file_name_on_the_phone_cannot_steer_the_pull() {
        let root = tmpdir("adbevil");
        let phone = root.join("cam");
        fs::create_dir_all(&phone).unwrap();
        let outside = root.join("private.jpg");
        fs::write(&outside, jpeg(40, 9)).unwrap();
        // a name with a newline makes `stat` print what looks like a second listing line
        // ("999 ../private.jpg" cannot be built with a slash in the name, but a bare
        // relative name or a different size for a real file can)
        fs::write(phone.join("x.jpg\n999 ../private.jpg"), jpeg(5, 1)).ok(); // may be refused by the OS
        fs::write(phone.join("x.jpg\n41 y.jpg"), jpeg(5, 1)).unwrap();
        fs::write(phone.join("y.jpg"), jpeg(7, 1)).unwrap();
        fs::write(phone.join("good.jpg"), jpeg(9, 1)).unwrap();
        let log = root.join("pulls.log");
        let mut c = cfg(&root.join("inbox"));
        c.adb_path = fake_adb(&root, &format!("while [ \"$1\" = \"-s\" ]; do shift 2; done\ncase \"$1\" in\n devices) printf 'List of devices attached\\nF\\tdevice\\n';;\n shell) shift; sh -c \"$1\";;\n pull) echo \"$2\" >> '{}'; cp \"$2\" \"$3\";;\nesac", log.display()));
        c.phone_dir = phone.display().to_string();
        c.adb_all = true;
        c.adb_delete = true;
        c.verify_content = true;
        let hub = Hub::new(c).unwrap();
        let mut prev = HashMap::new();
        for _ in 0..3 {
            let _ = adb::pass(&hub, &mut prev);
        }
        let pulled = fs::read_to_string(&log).unwrap_or_default();
        for l in pulled.lines() {
            assert!(
                l.starts_with(&format!("{}/", phone.display()))
                    && !l[phone.display().to_string().len() + 1..].contains('/'),
                "pulled outside the camera folder: {l:?}"
            );
        }
        assert!(
            outside.exists(),
            "a file outside the camera folder was deleted"
        );
        let got: Vec<String> = files_under(&root.join("inbox"))
            .iter()
            .map(|f| f.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(
            got.iter().any(|n| n == "good.jpg") && got.iter().any(|n| n == "y.jpg"),
            "{got:?}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_hung_adb_does_not_hang_porumbel() {
        let root = tmpdir("adbhang");
        let mut c = cfg(&root.join("inbox"));
        c.adb_path = fake_adb(&root, "sleep 600");
        c.adb_timeout = 1;
        let hub = Hub::new(c).unwrap();
        let t0 = std::time::Instant::now();
        let r = adb::pass(&hub, &mut HashMap::new());
        assert!(r.is_err());
        assert!(t0.elapsed() < Duration::from_secs(10), "{:?}", t0.elapsed());
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn mutated_valid_requests_never_break_the_server() {
    let dir = tmpdir("mutate");
    let (_hub, a) = start(cfg(&dir));
    let mut seed = 0x9E37_79B9u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let body = jpeg(20, 7);
    let valid: Vec<Vec<u8>> = vec![
        [
            format!(
                "PUT /dav/m.jpg HTTP/1.1\r\nConnection: close\r\nContent-Length: {}\r\n{AUTH}\r\n",
                body.len()
            )
            .into_bytes(),
            body.clone(),
        ]
        .concat(),
        format!("PROPFIND /dav/ HTTP/1.1\r\nConnection: close\r\nDepth: 1\r\n{AUTH}\r\n")
            .into_bytes(),
        format!("MKCOL /dav/x/ HTTP/1.1\r\nConnection: close\r\n{AUTH}\r\n").into_bytes(),
        format!("MOVE /dav/a.part HTTP/1.1\r\nConnection: close\r\nDestination: http://h/dav/a.jpg\r\n{AUTH}\r\n").into_bytes(),
        format!("GET /api/status HTTP/1.1\r\nConnection: close\r\n{AUTH}\r\n").into_bytes(),
    ];
    for i in 0..100 {
        let mut raw = valid[i % valid.len()].clone();
        for _ in 0..=(rnd() % 4) {
            let at = (rnd() as usize) % raw.len();
            match rnd() % 3 {
                0 => raw[at] ^= 1 << (rnd() % 8),
                1 => {
                    raw.remove(at);
                }
                _ => raw.insert(at, (rnd() & 0xFF) as u8),
            }
        }
        let mut s = TcpStream::connect(a).unwrap();
        s.set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        let _ = s.write_all(&raw);
        let _ = s.shutdown(std::net::Shutdown::Write);
        let mut sink = Vec::new();
        let _ = s.read_to_end(&mut sink);
    }
    // whatever was stored is a real image under the inbox; nothing half-written remains
    assert_eq!(tmp_leftovers(&dir), 0);
    for f in files_under(&dir) {
        assert!(f.starts_with(&dir), "{f:?}");
    }
    assert_eq!(put(a, "/dav/after.jpg", &jpeg(9, 3)), 201);
    let _ = fs::remove_dir_all(dir);
}
