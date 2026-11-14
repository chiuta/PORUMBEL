#![allow(clippy::field_reassign_with_default, clippy::ptr_arg)]
use porumbel::config::{Config, OpenMode};
use porumbel::hub::Hub;
use porumbel::{adb, server, util, watch};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "porumbel-it-{tag}-{}-{}",
        std::process::id(),
        util::hex(&util::random_bytes(4))
    ));
    fs::create_dir_all(&d).unwrap();
    d
}

fn cfg(inbox: &PathBuf) -> Config {
    let mut c = Config::default();
    c.inbox = inbox.clone();
    c.token = "s3cret-token".into();
    c.bind = "127.0.0.1".into();
    c.port = 0;
    c.session_gap = 3600;
    c
}

fn b64(s: &str) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let b = s.as_bytes();
    let mut o = String::new();
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                o.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}

fn http(addr: SocketAddr, raw: &[u8]) -> (u16, String, Vec<u8>) {
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    let _ = s.write_all(raw);
    let mut buf = Vec::new();
    let _ = s.read_to_end(&mut buf);
    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no header end");
    let head = String::from_utf8_lossy(&buf[..split]).to_string();
    let code: u16 = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    (code, head, buf[split + 4..].to_vec())
}

fn put(addr: SocketAddr, path: &str, auth: Option<&str>, body: &[u8]) -> (u16, String, Vec<u8>) {
    let mut r = format!(
        "PUT {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(a) = auth {
        r.push_str(&format!("Authorization: Basic {}\r\n", b64(a)));
    }
    r.push_str("\r\n");
    let mut raw = r.into_bytes();
    raw.extend_from_slice(body);
    http(addr, &raw)
}

fn start_server(c: Config) -> (Arc<Hub>, SocketAddr) {
    let hub = Arc::new(Hub::new(c).unwrap());
    let addr = server::start(hub.clone()).unwrap();
    (hub, SocketAddr::from(([127, 0, 0, 1], addr.port())))
}

#[test]
fn webdav_upload_flow() {
    let dir = tmpdir("dav");
    let mut c = cfg(&dir);
    c.max_upload_mb = 1;
    let (hub, a) = start_server(c);

    // no credentials / wrong credentials
    assert_eq!(put(a, "/dav/IMG_20261114_135501.jpg", None, b"x").0, 401);
    assert_eq!(
        put(a, "/dav/IMG_20261114_135501.jpg", Some("u:wrong"), b"x").0,
        401
    );

    // good upload with Basic auth (any user name, password = token)
    let photo = vec![7u8; 5000];
    let (code, _, _) = put(
        a,
        "/dav/IMG_20261114_135501.jpg",
        Some("phone:s3cret-token"),
        &photo,
    );
    assert_eq!(code, 201);
    let sessions = hub.sessions();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "2026-11-14_13-55-01");
    assert_eq!(
        fs::read(dir.join("2026-11-14_13-55-01/IMG_20261114_135501.jpg")).unwrap(),
        photo
    );

    // same photo again -> accepted, nothing new
    assert_eq!(
        put(
            a,
            "/dav/IMG_20261114_135501.jpg",
            Some("p:s3cret-token"),
            &photo
        )
        .0,
        204
    );
    assert_eq!(hub.sessions()[0].count, 1);

    // token in the query works too (browser upload page)
    assert_eq!(
        put(
            a,
            "/api/upload/IMG_20261114_135502.jpg?t=s3cret-token",
            None,
            b"abc"
        )
        .0,
        201
    );
    assert_eq!(hub.sessions()[0].count, 2);

    // refused: wrong type, traversal ends up as a plain name, too large
    assert_eq!(put(a, "/dav/run.exe", Some("p:s3cret-token"), b"MZ").0, 415);
    let (code, _, _) = put(
        a,
        "/dav/..%2F..%2Fescape.jpg",
        Some("p:s3cret-token"),
        b"zz",
    );
    assert_eq!(code, 201);
    assert!(!dir.parent().unwrap().join("escape.jpg").exists());
    let big = vec![0u8; 2 * 1024 * 1024];
    assert_eq!(put(a, "/dav/big.jpg", Some("p:s3cret-token"), &big).0, 413);

    // chunked upload
    let chunked = b"PUT /dav/chunky.jpg HTTP/1.1\r\nHost: x\r\nConnection: close\r\nTransfer-Encoding: chunked\r\nAuthorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";
    assert_eq!(http(a, chunked).0, 201);
    assert_eq!(
        fs::read(hub.find_file("chunky.jpg").unwrap()).unwrap(),
        b"hello world"
    );

    // temp name + MOVE (clients that upload to a temporary name first)
    assert_eq!(
        put(a, "/dav/IMG_9.jpg.tmp", Some("p:s3cret-token"), b"tmpdata").0,
        201
    );
    let mv = b"MOVE /dav/IMG_9.jpg.tmp HTTP/1.1\r\nHost: x\r\nConnection: close\r\nDestination: http://x/dav/IMG_9.jpg\r\nAuthorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n\r\n";
    assert_eq!(http(a, mv).0, 201);
    assert_eq!(
        fs::read(hub.find_file("IMG_9.jpg").unwrap()).unwrap(),
        b"tmpdata"
    );

    // PROPFIND lists what was received; GET returns the bytes
    let pf = b"PROPFIND /dav/ HTTP/1.1\r\nHost: x\r\nConnection: close\r\nDepth: 1\r\nContent-Length: 0\r\nAuthorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n\r\n";
    let (code, _, body) = http(a, pf);
    let xml = String::from_utf8_lossy(&body).to_string();
    assert_eq!(code, 207);
    assert!(
        xml.contains("IMG_20261114_135501.jpg") && xml.contains("<D:getcontentlength>5000<"),
        "{xml}"
    );
    let get = b"GET /dav/IMG_20261114_135501.jpg HTTP/1.1\r\nHost: x\r\nConnection: close\r\nAuthorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n\r\n";
    let (code, head, body) = http(a, get);
    assert_eq!(code, 200);
    assert!(head.contains("image/jpeg"));
    assert_eq!(body, photo);
    let pf0 = b"PROPFIND /dav/IMG_nope.jpg HTTP/1.1\r\nHost: x\r\nConnection: close\r\nDepth: 0\r\nAuthorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n\r\n";
    assert_eq!(http(a, pf0).0, 404);

    // LOCK / UNLOCK are accepted (clients that lock before writing)
    let lock = b"LOCK /dav/IMG_9.jpg HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Length: 0\r\nAuthorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n\r\n";
    let (code, head, _) = http(a, lock);
    assert_eq!(code, 200);
    assert!(head.to_ascii_lowercase().contains("lock-token"));
    let unlock = b"UNLOCK /dav/IMG_9.jpg HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Length: 0\r\nAuthorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n\r\n";
    assert_eq!(http(a, unlock).0, 204);

    // OPTIONS advertises DAV
    let (code, head, _) = http(a, b"OPTIONS /dav/ HTTP/1.1\r\nHost: x\r\nConnection: close\r\nAuthorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n\r\n");
    assert_eq!(code, 200);
    assert!(head.to_ascii_lowercase().contains("dav: 1"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn panel_api_and_csrf_rules() {
    let dir = tmpdir("api");
    let (hub, a) = start_server(cfg(&dir));
    let ok = "Authorization: Basic cDpzM2NyZXQtdG9rZW4=\r\n";

    let (code, head, body) = http(
        a,
        "GET /?t=s3cret-token HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"
            .to_string()
            .as_bytes(),
    );
    assert_eq!(code, 200);
    assert!(head.contains("Content-Security-Policy") && head.contains("no-referrer"));
    assert!(String::from_utf8_lossy(&body).contains("PORUMBEL"));
    assert_eq!(
        http(a, b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").0,
        401
    );

    put(a, "/dav/a.jpg", Some("p:s3cret-token"), b"1");
    let (code, _, body) = http(
        a,
        format!("GET /api/status HTTP/1.1\r\nHost: x\r\nConnection: close\r\n{ok}\r\n").as_bytes(),
    );
    let j = String::from_utf8_lossy(&body).to_string();
    assert_eq!(code, 200);
    assert!(
        j.contains("\"sessions\":[{") && j.contains("\"count\":1"),
        "{j}"
    );

    // state-changing POST: Basic auth alone is refused (a foreign web page could send it)
    let post = |extra: &str, path: &str| {
        http(a, format!("POST {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Length: 0\r\n{extra}\r\n").as_bytes()).0
    };
    assert_eq!(post(ok, "/api/close"), 401);
    assert_eq!(
        post("X-Porumbel-Token: s3cret-token\r\n", "/api/close"),
        200
    );
    assert_eq!(post("", "/api/close?t=s3cret-token"), 200);
    // opening a name outside the inbox is refused
    assert_eq!(
        post("X-Porumbel-Token: s3cret-token\r\n", "/api/open?s=..%2F.."),
        404
    );
    // QR image
    let (code, head, body) = http(
        a,
        format!("GET /qr.svg HTTP/1.1\r\nHost: x\r\nConnection: close\r\n{ok}\r\n").as_bytes(),
    );
    assert_eq!(code, 200);
    assert!(head.contains("image/svg+xml") && String::from_utf8_lossy(&body).contains("<svg"));
    drop(hub);
    let _ = fs::remove_dir_all(dir);
}

#[cfg(unix)]
mod unix_only {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A stand-in for `adb` that runs the "phone" commands on this machine, so the
    /// real command lines of the ADB transport are exercised end to end.
    fn fake_adb(dir: &PathBuf) -> String {
        let p = dir.join("adb");
        fs::write(
            &p,
            "#!/bin/sh\nwhile [ \"$1\" = \"-s\" ]; do shift 2; done\ncase \"$1\" in\n devices) printf 'List of devices attached\\nFAKE123\\tdevice\\n\\n';;\n shell) shift; sh -c \"$1\";;\n pull) cp \"$2\" \"$3\";;\n *) exit 1;;\nesac\n",
        )
        .unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        p.display().to_string()
    }

    #[test]
    fn adb_baseline_stability_and_delete() {
        let root = tmpdir("adb");
        let phone = root.join("phone dir"); // a space on purpose
        fs::create_dir_all(&phone).unwrap();
        fs::write(phone.join("IMG_20260101_100000.jpg"), b"old photo").unwrap();
        let mut c = cfg(&root.join("inbox"));
        c.adb = true;
        c.adb_path = fake_adb(&root);
        c.phone_dir = phone.display().to_string();
        c.adb_delete = true;
        let hub = Hub::new(c).unwrap();
        let mut prev = HashMap::new();

        // first run: the existing photo is left alone
        adb::pass(&hub, &mut prev).unwrap();
        assert!(hub.sessions().is_empty());

        // a new photo is only taken once its size stayed the same between two polls
        fs::write(phone.join("IMG_20261114_135501.jpg"), b"new photo bytes").unwrap();
        fs::write(phone.join(".pending-123-IMG_x.jpg"), b"hidden temp").unwrap();
        fs::write(phone.join("notes.txt"), b"not an image").unwrap();
        adb::pass(&hub, &mut prev).unwrap();
        assert!(hub.sessions().is_empty(), "taken too early");
        adb::pass(&hub, &mut prev).unwrap();
        let s = hub.sessions();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].count, 1);
        assert_eq!(
            fs::read(root.join("inbox/2026-11-14_13-55-01/IMG_20261114_135501.jpg")).unwrap(),
            b"new photo bytes"
        );
        // deleted on the phone after the copy, hidden temp and old photo untouched
        assert!(!phone.join("IMG_20261114_135501.jpg").exists());
        assert!(
            phone.join(".pending-123-IMG_x.jpg").exists()
                && phone.join("IMG_20260101_100000.jpg").exists()
        );
        // nothing more on the next pass
        adb::pass(&hub, &mut prev).unwrap();
        assert_eq!(hub.sessions()[0].count, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn adb_reports_no_phone() {
        let root = tmpdir("adb2");
        let p = root.join("adb");
        fs::write(&p, "#!/bin/sh\nprintf 'List of devices attached\\n\\n'\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        let mut c = cfg(&root.join("inbox"));
        c.adb_path = p.display().to_string();
        let hub = Hub::new(c).unwrap();
        assert_eq!(
            adb::pass(&hub, &mut HashMap::new()).unwrap_err(),
            "no phone connected"
        );
        let mut c = cfg(&root.join("inbox"));
        c.adb_path = "/nonexistent/adb".into();
        let hub = Hub::new(c).unwrap();
        assert!(adb::pass(&hub, &mut HashMap::new())
            .unwrap_err()
            .contains("cannot run adb"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn open_in_legatorocr_gets_the_session_folder() {
        let root = tmpdir("launch");
        let out = root.join("args.txt");
        let l = root.join("legatorocr-gui");
        fs::write(
            &l,
            format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n", out.display()),
        )
        .unwrap();
        fs::set_permissions(&l, fs::Permissions::from_mode(0o755)).unwrap();
        let mut c = cfg(&root.join("inbox"));
        c.launch = Some(l.display().to_string());
        c.launch_args = vec!["--demo".into()];
        c.session_gap = 0;
        c.open = OpenMode::Auto;
        let hub = Hub::new(c).unwrap();
        let t = hub.tmp_path("part");
        fs::write(&t, b"jpegdata").unwrap();
        hub.ingest(&t, "IMG_20261114_135501.jpg", "test").unwrap();
        // the idle session is reported once, then handed over
        let name = hub.tick().expect("session ready");
        porumbel::open_named(&hub, &name).unwrap();
        for _ in 0..50 {
            if out.exists() && fs::metadata(&out).map(|m| m.len() > 0).unwrap_or(false) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let got = fs::read_to_string(&out).unwrap();
        let lines: Vec<&str> = got.lines().collect();
        assert_eq!(lines[0], "--demo");
        assert_eq!(PathBuf::from(lines[1]), root.join("inbox").join(&name));
        assert!(hub.tick().is_none(), "must not be opened twice");
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn watched_folder_copy_and_move() {
    let root = tmpdir("watch");
    let sync = root.join("sync");
    fs::create_dir_all(sync.join(".stfolder")).unwrap();
    fs::create_dir_all(sync.join("sub")).unwrap();
    fs::write(sync.join("IMG_20250101_000000.jpg"), b"before").unwrap();
    let mut c = cfg(&root.join("inbox"));
    c.watch_move = true;
    let hub = Hub::new(c).unwrap();
    let mut prev = HashMap::new();
    watch::pass(&hub, &sync, &mut prev); // baseline
    assert!(hub.sessions().is_empty());

    fs::write(sync.join("sub/IMG_20261114_140000.jpg"), b"page one").unwrap();
    fs::write(
        sync.join(".syncthing.IMG_20261114_140100.jpg.tmp"),
        b"partial",
    )
    .unwrap();
    watch::pass(&hub, &sync, &mut prev);
    assert!(hub.sessions().is_empty());
    watch::pass(&hub, &sync, &mut prev);
    assert_eq!(hub.sessions().len(), 1);
    assert!(
        !sync.join("sub/IMG_20261114_140000.jpg").exists(),
        "moved out of the synced folder"
    );
    assert!(sync.join("IMG_20250101_000000.jpg").exists());
    let _ = fs::remove_dir_all(root);
}
