//! Transport 3 — a small HTTP server for the local network (std only).
//!
//! * `PUT /dav/NAME` and a minimal WebDAV (OPTIONS, PROPFIND, MKCOL, MOVE, GET, HEAD)
//!   so phone sync apps (FolderSync, Round Sync, Material Files, …) can push photos;
//! * `PUT /api/upload/NAME` used by the upload page in the phone's browser;
//! * the control panel (`/`), `/api/status`, `/qr.svg` and a few POST actions.
//!
//! Every request needs the access token (query `t`, header `X-Porumbel-Token` or the
//! password of HTTP Basic auth). Actions that change something accept only the query
//! or the header, never Basic auth, so another web page cannot trigger them.

use crate::hub::{Hub, Outcome};
use crate::{launch, qr, util};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PANEL: &str = include_str!("panel.html");
const MAX_CONNS: usize = 128;
/// A connection that has not delivered a complete request this long after it was
/// opened (or after the previous answer) is cut: protects against idle/slowloris sockets.
const HEADER_SECS: u64 = 10;

/// Connection bookkeeping. A connection is *half-open* while we wait for a request
/// (nobody has proved anything yet) and *active* while one is being served. When the
/// table is full, a new connection evicts the oldest half-open one, so idle sockets
/// can never lock a real client out.
struct Slots {
    next: u64,
    half: Vec<(u64, Instant, TcpStream)>,
    active: usize,
}
static SLOTS: Mutex<Slots> = Mutex::new(Slots {
    next: 0,
    half: Vec::new(),
    active: 0,
});

fn slots() -> std::sync::MutexGuard<'static, Slots> {
    SLOTS.lock().unwrap_or_else(|e| e.into_inner())
}

fn register(s: &TcpStream) -> Option<u64> {
    let clone = s.try_clone().ok()?;
    let mut g = slots();
    if g.active + g.half.len() >= MAX_CONNS {
        if g.half.is_empty() {
            return None;
        }
        let (_, _, old) = g.half.remove(0);
        let _ = old.shutdown(std::net::Shutdown::Both);
    }
    g.next += 1;
    let id = g.next;
    g.half.push((id, Instant::now(), clone));
    Some(id)
}

/// half-open → active; false when the connection was evicted or timed out meanwhile
fn to_active(id: u64) -> bool {
    let mut g = slots();
    match g.half.iter().position(|(i, _, _)| *i == id) {
        Some(p) => {
            g.half.remove(p);
            g.active += 1;
            true
        }
        None => false,
    }
}

fn to_half(id: u64, s: &TcpStream) {
    let clone = s.try_clone();
    let mut g = slots();
    g.active = g.active.saturating_sub(1);
    if let Ok(c) = clone {
        g.half.push((id, Instant::now(), c));
    }
}

fn release(id: u64, active: bool) {
    let mut g = slots();
    if active {
        g.active = g.active.saturating_sub(1);
    } else {
        g.half.retain(|(i, _, _)| *i != id);
    }
}

fn reaper() {
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let mut g = slots();
        let now = Instant::now();
        g.half.retain(|(_, since, s)| {
            if now.duration_since(*since) >= Duration::from_secs(HEADER_SECS) {
                let _ = s.shutdown(std::net::Shutdown::Both);
                false
            } else {
                true
            }
        });
    }
}

pub struct Req {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    /// false when the client asked for `Connection: close` (or speaks HTTP/1.0)
    pub keep: bool,
}

impl Req {
    fn h(&self, k: &str) -> Option<&str> {
        self.headers.get(k).map(|s| s.as_str())
    }
}

pub fn start(hub: Arc<Hub>) -> io::Result<SocketAddr> {
    let listener = TcpListener::bind((hub.cfg.bind.as_str(), hub.cfg.port))?;
    let addr = listener.local_addr()?;
    hub.port.store(addr.port(), Ordering::Relaxed);
    static REAPER: std::sync::Once = std::sync::Once::new();
    REAPER.call_once(|| {
        std::thread::spawn(reaper);
    });
    std::thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let Some(id) = register(&s) else {
                let mut s = s;
                let _ = s.write_all(b"HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");
                continue;
            };
            let hub = hub.clone();
            std::thread::spawn(move || {
                let mut active = false;
                let _ = handle(&hub, s, id, &mut active);
                release(id, active);
            });
        }
    });
    Ok(addr)
}

fn read_line<R: BufRead>(r: &mut R, max: usize) -> io::Result<Option<String>> {
    let mut buf = Vec::new();
    let n = r.by_ref().take(max as u64).read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    if !buf.ends_with(b"\n") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "line too long"));
    }
    while buf.last().is_some_and(|c| *c == b'\n' || *c == b'\r') {
        buf.pop();
    }
    Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
}

fn read_request<R: BufRead>(r: &mut R) -> io::Result<Option<Req>> {
    let line = loop {
        match read_line(r, 8192)? {
            None => return Ok(None),
            Some(l) if l.is_empty() => continue, // stray CRLF between requests
            Some(l) => break l,
        }
    };
    let mut p = line.split_whitespace();
    let (Some(method), Some(target), Some(ver)) = (p.next(), p.next(), p.next()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bad request line",
        ));
    };
    if !ver.starts_with("HTTP/1.") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad version"));
    }
    let mut headers = HashMap::new();
    let mut total = 0;
    loop {
        let Some(l) = read_line(r, 8192)? else {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "eof in headers",
            ));
        };
        if l.is_empty() {
            break;
        }
        total += l.len();
        if total > 32 * 1024 || headers.len() > 100 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "headers too large",
            ));
        }
        if let Some((k, v)) = l.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    // absolute-form targets: http://host/path
    let target = match target.find("://") {
        Some(i) => match target[i + 3..].find('/') {
            Some(j) => &target[i + 3 + j..],
            None => "/",
        },
        None => target,
    };
    let (raw_path, raw_q) = target.split_once('?').unwrap_or((target, ""));
    let path = util::percent_decode(raw_path);
    if path.contains('\0') {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad path"));
    }
    let mut query = HashMap::new();
    for kv in raw_q.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        query.insert(util::percent_decode(k), util::percent_decode(v));
    }
    let conn = headers
        .get("connection")
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    let keep = if ver == "HTTP/1.0" {
        conn.contains("keep-alive")
    } else {
        !conn.contains("close")
    };
    Ok(Some(Req {
        method: method.to_ascii_uppercase(),
        path,
        query,
        headers,
        keep,
    }))
}

#[derive(Debug)]
enum Body {
    None,
    Len(u64),
    Chunked,
}

#[derive(Debug)]
#[allow(dead_code)] // the I/O error is kept for debugging output
enum BodyErr {
    TooLarge,
    Bad,
    Io(io::Error),
}

impl From<io::Error> for BodyErr {
    fn from(e: io::Error) -> Self {
        BodyErr::Io(e)
    }
}

fn body_kind(req: &Req) -> Result<Body, BodyErr> {
    if req
        .h("transfer-encoding")
        .is_some_and(|v| v.to_ascii_lowercase().contains("chunked"))
    {
        return Ok(Body::Chunked);
    }
    match req.h("content-length") {
        None => Ok(Body::None),
        Some(v) => v
            .trim()
            .parse::<u64>()
            .map(Body::Len)
            .map_err(|_| BodyErr::Bad),
    }
}

fn read_body<R: BufRead>(
    r: &mut R,
    kind: &Body,
    out: &mut dyn Write,
    limit: u64,
) -> Result<u64, BodyErr> {
    match kind {
        Body::None => Ok(0),
        Body::Len(n) => {
            if *n > limit {
                return Err(BodyErr::TooLarge);
            }
            let got = io::copy(&mut r.by_ref().take(*n), out)?;
            if got != *n {
                return Err(BodyErr::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "short body",
                )));
            }
            Ok(got)
        }
        Body::Chunked => {
            let mut total = 0u64;
            loop {
                let l = read_line(r, 256)?.ok_or(BodyErr::Bad)?;
                let hexs = l.split(';').next().unwrap_or("").trim();
                let size = u64::from_str_radix(hexs, 16).map_err(|_| BodyErr::Bad)?;
                if size == 0 {
                    // trailers until an empty line
                    while let Some(t) = read_line(r, 8192)? {
                        if t.is_empty() {
                            break;
                        }
                    }
                    return Ok(total);
                }
                total = total.checked_add(size).ok_or(BodyErr::TooLarge)?;
                if total > limit {
                    return Err(BodyErr::TooLarge);
                }
                let got = io::copy(&mut r.by_ref().take(size), out)?;
                if got != size {
                    return Err(BodyErr::Io(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "short chunk",
                    )));
                }
                let mut crlf = [0u8; 2];
                r.read_exact(&mut crlf)?;
            }
        }
    }
}

fn authed(hub: &Hub, req: &Req, allow_basic: bool) -> bool {
    let tok = hub.cfg.token.as_bytes();
    if tok.is_empty() {
        return false;
    }
    if let Some(t) = req.query.get("t") {
        if util::ct_eq(t.as_bytes(), tok) {
            return true;
        }
    }
    if let Some(t) = req.h("x-porumbel-token") {
        if util::ct_eq(t.as_bytes(), tok) {
            return true;
        }
    }
    if allow_basic {
        if let Some(a) = req.h("authorization") {
            if let Some(b64) = a
                .strip_prefix("Basic ")
                .or_else(|| a.strip_prefix("basic "))
            {
                if let Some(raw) = util::base64_decode(b64.trim()) {
                    let pass = match raw.iter().position(|c| *c == b':') {
                        Some(i) => &raw[i + 1..],
                        None => &raw[..],
                    };
                    if util::ct_eq(pass, tok) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

struct Resp<'a> {
    code: u16,
    ctype: &'a str,
    extra: Vec<(&'static str, String)>,
    body: Vec<u8>,
}

fn reason(code: u16) -> &'static str {
    match code {
        100 => "Continue",
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        207 => "Multi-Status",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

fn send(stream: &mut TcpStream, r: Resp, head_only: bool, keep: bool, csp: bool) -> io::Result<()> {
    let mut h = format!(
        "HTTP/1.1 {} {}\r\nServer: PORUMBEL\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Length: {}\r\nConnection: {}\r\n",
        r.code,
        reason(r.code),
        r.body.len(),
        if keep { "keep-alive" } else { "close" }
    );
    if !r.ctype.is_empty() {
        h.push_str(&format!("Content-Type: {}\r\n", r.ctype));
    }
    if csp {
        h.push_str("Content-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'\r\nX-Frame-Options: DENY\r\n");
    }
    for (k, v) in &r.extra {
        h.push_str(&format!("{k}: {v}\r\n"));
    }
    h.push_str("\r\n");
    stream.write_all(h.as_bytes())?;
    if !head_only {
        stream.write_all(&r.body)?;
    }
    stream.flush()
}

fn text(code: u16, msg: &str) -> Resp<'static> {
    Resp {
        code,
        ctype: "text/plain; charset=utf-8",
        extra: vec![],
        body: format!("{msg}\n").into_bytes(),
    }
}

fn json(code: u16, body: String) -> Resp<'static> {
    Resp {
        code,
        ctype: "application/json; charset=utf-8",
        extra: vec![],
        body: body.into_bytes(),
    }
}

fn handle(hub: &Hub, stream: TcpStream, id: u64, active: &mut bool) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(60)))?;
    let _ = stream.set_nodelay(true);
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut stream = stream;
    loop {
        let req = match read_request(&mut reader) {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(()),
            Err(_) => {
                let _ = send(&mut stream, text(400, "bad request"), false, false, false);
                return Ok(());
            }
        };
        if !to_active(id) {
            return Ok(()); // evicted or timed out while the request was arriving
        }
        *active = true;
        let keep = route(hub, &req, &mut reader, &mut stream)?;
        if !keep {
            // Lingering close: tell the client we are done, then swallow what it still
            // sends, so an early answer (401, 413…) is not lost to a TCP reset.
            let _ = stream.shutdown(std::net::Shutdown::Write);
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = io::copy(&mut reader.by_ref().take(16 * 1024 * 1024), &mut io::sink());
            return Ok(());
        }
        stream.set_read_timeout(Some(Duration::from_secs(20)))?;
        to_half(id, &stream);
        *active = false;
    }
}

fn last_component(path: &str) -> &str {
    path.trim_end_matches('/').rsplit('/').next().unwrap_or("")
}

fn route(
    hub: &Hub,
    req: &Req,
    reader: &mut BufReader<TcpStream>,
    stream: &mut TcpStream,
) -> io::Result<bool> {
    let m = req.method.as_str();
    let mutating_api = m == "POST";
    if !authed(hub, req, !mutating_api) {
        std::thread::sleep(Duration::from_millis(400)); // slows down guessing
        let mut r = text(401, "token required");
        r.extra.push((
            "WWW-Authenticate",
            "Basic realm=\"PORUMBEL (password = token)\"".into(),
        ));
        send(stream, r, m == "HEAD", false, false)?;
        return Ok(false);
    }
    let path = req.path.as_str();
    let is_dav = path == "/dav" || path.starts_with("/dav/");

    // small bodies we do not use
    let drain = |reader: &mut BufReader<TcpStream>| -> bool {
        match body_kind(req) {
            Ok(k) => read_body(reader, &k, &mut io::sink(), 1 << 20).is_ok(),
            Err(_) => false,
        }
    };

    match m {
        "OPTIONS" => {
            if !drain(reader) {
                return Ok(false);
            }
            let mut r = text(200, "");
            r.body.clear();
            r.extra.push(("DAV", "1, 2".into()));
            r.extra.push(("MS-Author-Via", "DAV".into()));
            r.extra.push((
                "Allow",
                "OPTIONS, GET, HEAD, PUT, PROPFIND, MKCOL, MOVE, LOCK, UNLOCK".into(),
            ));
            send(stream, r, false, req.keep, false)?;
            Ok(req.keep)
        }
        "GET" | "HEAD" if path == "/" => {
            if !drain(reader) {
                return Ok(false);
            }
            let r = Resp {
                code: 200,
                ctype: "text/html; charset=utf-8",
                extra: vec![],
                body: PANEL.as_bytes().to_vec(),
            };
            send(stream, r, m == "HEAD", req.keep, true)?;
            Ok(req.keep)
        }
        "GET" if path == "/api/status" => {
            if !drain(reader) {
                return Ok(false);
            }
            send(
                stream,
                json(200, hub.status_json(&launch::describe(&hub.cfg))),
                false,
                req.keep,
                false,
            )?;
            Ok(req.keep)
        }
        "GET" if path == "/qr.svg" => {
            if !drain(reader) {
                return Ok(false);
            }
            let url = hub.phone_url().unwrap_or_else(|| {
                format!(
                    "http://localhost:{}/?t={}",
                    hub.port.load(Ordering::Relaxed),
                    hub.cfg.token
                )
            });
            let r = match qr::Qr::encode(url.as_bytes()) {
                Ok(q) => Resp {
                    code: 200,
                    ctype: "image/svg+xml",
                    extra: vec![],
                    body: q.to_svg(4).into_bytes(),
                },
                Err(e) => text(500, &e),
            };
            send(stream, r, false, req.keep, false)?;
            Ok(req.keep)
        }
        "POST" if path.starts_with("/api/") => {
            if !drain(reader) {
                return Ok(false);
            }
            let r = api_post(hub, req);
            send(stream, r, false, req.keep, false)?;
            Ok(req.keep)
        }
        "PUT" if !path.starts_with("/api/") || path.starts_with("/api/upload/") => {
            put(hub, req, reader, stream)
        }
        "LOCK" if is_dav => {
            if !drain(reader) {
                return Ok(false);
            }
            // Some WebDAV clients insist on a lock before writing. Uploads here are
            // atomic anyway, so hand out a token that means nothing.
            let tok = format!("opaquelocktoken:{}", util::hex(&util::random_bytes(8)));
            let xml = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><D:prop xmlns:D=\"DAV:\"><D:lockdiscovery><D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope><D:depth>0</D:depth><D:timeout>Second-600</D:timeout><D:locktoken><D:href>{tok}</D:href></D:locktoken></D:activelock></D:lockdiscovery></D:prop>");
            let r = Resp {
                code: 200,
                ctype: "application/xml; charset=utf-8",
                extra: vec![("Lock-Token", format!("<{tok}>"))],
                body: xml.into_bytes(),
            };
            send(stream, r, false, req.keep, false)?;
            Ok(req.keep)
        }
        "UNLOCK" if is_dav => {
            if !drain(reader) {
                return Ok(false);
            }
            let mut r = text(204, "");
            r.body.clear();
            send(stream, r, false, req.keep, false)?;
            Ok(req.keep)
        }
        "MKCOL" => {
            if !drain(reader) {
                return Ok(false);
            }
            let mut r = text(201, "");
            r.body.clear();
            send(stream, r, false, req.keep, false)?;
            Ok(req.keep)
        }
        "PROPFIND" => {
            if !drain(reader) {
                return Ok(false);
            }
            let r = propfind(hub, req);
            send(stream, r, false, req.keep, false)?;
            Ok(req.keep)
        }
        "MOVE" if is_dav => {
            if !drain(reader) {
                return Ok(false);
            }
            let r = move_staged(hub, req);
            send(stream, r, false, req.keep, false)?;
            Ok(req.keep)
        }
        "GET" | "HEAD" if is_dav => {
            if !drain(reader) {
                return Ok(false);
            }
            serve_file(hub, req, stream, m == "HEAD")
        }
        "GET" | "HEAD" => {
            if !drain(reader) {
                return Ok(false);
            }
            send(stream, text(404, "not found"), m == "HEAD", req.keep, false)?;
            Ok(req.keep)
        }
        _ => {
            send(stream, text(405, "method not allowed"), false, false, false)?;
            Ok(false)
        }
    }
}

fn api_post(hub: &Hub, req: &Req) -> Resp<'static> {
    let name = req.query.get("s").map(|s| s.as_str()).unwrap_or("");
    match req.path.as_str() {
        "/api/open" | "/api/open-latest" => {
            let target = if req.path == "/api/open" {
                Some(name.to_string())
            } else {
                hub.latest_session()
            };
            match target.and_then(|n| hub.session_dir(&n).map(|_| n)) {
                Some(n) => match crate::open_named(hub, &n) {
                    Ok(msg) => json(
                        200,
                        format!("{{\"ok\":true,\"msg\":{}}}", util::json_str(&msg)),
                    ),
                    Err(e) => json(
                        500,
                        format!("{{\"ok\":false,\"msg\":{}}}", util::json_str(&e)),
                    ),
                },
                None => json(404, "{\"ok\":false,\"msg\":\"no such session\"}".into()),
            }
        }
        "/api/close" => {
            hub.close_current();
            json(200, "{\"ok\":true}".into())
        }
        "/api/forget" => {
            hub.forget();
            json(200, "{\"ok\":true}".into())
        }
        _ => json(404, "{\"ok\":false}".into()),
    }
}

fn is_temp_name(name: &str) -> bool {
    let e = util::ext_of(name);
    matches!(
        e.as_str(),
        "tmp" | "part" | "partial" | "upload" | "temp" | "crdownload"
    )
}

fn put(
    hub: &Hub,
    req: &Req,
    reader: &mut BufReader<TcpStream>,
    stream: &mut TcpStream,
) -> io::Result<bool> {
    let raw = last_component(&req.path);
    let kind = match body_kind(req) {
        Ok(k) => k,
        Err(_) => {
            send(stream, text(400, "bad length"), false, false, false)?;
            return Ok(false);
        }
    };
    let Some(name) = util::sanitize_name(raw) else {
        send(stream, text(400, "bad file name"), false, false, false)?;
        return Ok(false);
    };
    let accepted = util::has_ext_in(&name, &hub.cfg.extensions);
    let staged = !accepted && is_temp_name(&name);
    if !accepted && !staged {
        send(
            stream,
            text(415, "file type not accepted"),
            false,
            false,
            false,
        )?;
        return Ok(false);
    }
    let limit = hub.cfg.max_upload_bytes();
    if let Body::Len(n) = kind {
        if n > limit {
            send(stream, text(413, "file too large"), false, false, false)?;
            return Ok(false);
        }
    }
    if req
        .h("expect")
        .is_some_and(|v| v.eq_ignore_ascii_case("100-continue"))
    {
        stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
    }
    let tmp = hub.tmp_path("part");
    let res = {
        let mut f = fs::File::create(&tmp)?;
        let r = read_body(reader, &kind, &mut f, limit);
        let _ = f.flush();
        r
    };
    match res {
        Ok(_) => {}
        Err(BodyErr::TooLarge) => {
            let _ = fs::remove_file(&tmp);
            send(stream, text(413, "file too large"), false, false, false)?;
            return Ok(false);
        }
        Err(BodyErr::Bad) => {
            let _ = fs::remove_file(&tmp);
            send(stream, text(400, "bad body"), false, false, false)?;
            return Ok(false);
        }
        Err(BodyErr::Io(_)) => {
            let _ = fs::remove_file(&tmp);
            return Ok(false);
        }
    }
    if staged {
        let dest = hub.stage_dir().join(&name);
        let ok = fs::rename(&tmp, &dest).is_ok();
        let mut r = text(if ok { 201 } else { 500 }, "");
        r.body.clear();
        send(stream, r, false, req.keep, false)?;
        return Ok(req.keep);
    }
    let r = match hub.ingest(&tmp, &name, "web") {
        Ok(Outcome::Stored { .. }) => text(201, "stored"),
        Ok(Outcome::Duplicate) => text(204, ""),
        Err(e) => text(415, &e),
    };
    send(stream, r, false, req.keep, false)?;
    Ok(req.keep)
}

fn move_staged(hub: &Hub, req: &Req) -> Resp<'static> {
    let src = util::sanitize_name(last_component(&req.path));
    let dst_raw = req.h("destination").unwrap_or("");
    let dst_path = match dst_raw.find("://") {
        Some(i) => dst_raw[i + 3..]
            .find('/')
            .map(|j| &dst_raw[i + 3 + j..])
            .unwrap_or("/"),
        None => dst_raw,
    };
    let dst_path = dst_path.split('?').next().unwrap_or("");
    let dst = util::percent_decode(last_component(dst_path));
    let (Some(src), true) = (src, !dst.is_empty()) else {
        return text(400, "bad request");
    };
    let staged = hub.stage_dir().join(&src);
    if !staged.is_file() {
        return text(404, "nothing staged under that name");
    }
    let tmp = hub.tmp_path("part");
    if fs::rename(&staged, &tmp).is_err() {
        return text(500, "cannot move");
    }
    match hub.ingest(&tmp, &dst, "web") {
        Ok(Outcome::Stored { .. }) => text(201, ""),
        Ok(Outcome::Duplicate) => text(204, ""),
        Err(e) => text(415, &e),
    }
}

fn ctype_for(name: &str) -> &'static str {
    match util::ext_of(name).as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "tif" | "tiff" => "image/tiff",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
}

fn propfind(hub: &Hub, req: &Req) -> Resp<'static> {
    let path = req.path.as_str();
    let depth0 = req.h("depth").is_some_and(|d| d.trim() == "0");
    let last = last_component(path);
    let looks_like_file = !path.ends_with('/') && util::has_ext_in(last, &hub.cfg.extensions);
    let mut x = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<D:multistatus xmlns:D=\"DAV:\">\n",
    );
    if looks_like_file {
        match hub.entry(last) {
            Some((size, mt)) => x.push_str(&file_response(path, last, size, mt)),
            None => return text(404, "not found"),
        }
    } else {
        let mut base = path.to_string();
        if !base.ends_with('/') {
            base.push('/');
        }
        let href: String = base
            .split('/')
            .map(util::percent_encode)
            .collect::<Vec<_>>()
            .join("/");
        x.push_str(&format!(
            "<D:response><D:href>{}</D:href><D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype><D:displayname>{}</D:displayname></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>\n",
            util::xml_escape(&href),
            util::xml_escape(last)
        ));
        if !depth0 {
            for (n, size, mt) in hub.listing() {
                x.push_str(&file_response(&format!("{base}{n}"), &n, size, mt));
            }
        }
    }
    x.push_str("</D:multistatus>\n");
    Resp {
        code: 207,
        ctype: "application/xml; charset=utf-8",
        extra: vec![],
        body: x.into_bytes(),
    }
}

fn file_response(path: &str, name: &str, size: u64, mtime: u64) -> String {
    let href: String = path
        .split('/')
        .map(util::percent_encode)
        .collect::<Vec<_>>()
        .join("/");
    format!(
        "<D:response><D:href>{}</D:href><D:propstat><D:prop><D:resourcetype/><D:displayname>{}</D:displayname><D:getcontentlength>{}</D:getcontentlength><D:getlastmodified>{}</D:getlastmodified><D:getcontenttype>{}</D:getcontenttype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>\n",
        util::xml_escape(&href),
        util::xml_escape(name),
        size,
        util::http_date(mtime),
        ctype_for(name)
    )
}

fn serve_file(hub: &Hub, req: &Req, stream: &mut TcpStream, head: bool) -> io::Result<bool> {
    let name = last_component(&req.path);
    let Some(p) = hub.find_file(name) else {
        send(stream, text(404, "not found"), head, req.keep, false)?;
        return Ok(req.keep);
    };
    let mut f = fs::File::open(&p)?;
    let len = f.metadata()?.len();
    let h = format!(
        "HTTP/1.1 200 OK\r\nServer: PORUMBEL\r\nContent-Type: {}\r\nContent-Length: {len}\r\nX-Content-Type-Options: nosniff\r\nConnection: {}\r\n\r\n",
        ctype_for(name),
        if req.keep { "keep-alive" } else { "close" }
    );
    stream.write_all(h.as_bytes())?;
    if !head {
        io::copy(&mut f, stream)?;
    }
    stream.flush()?;
    Ok(req.keep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_roundtrip() {
        let data = b"5\r\nhello\r\n6;ext=1\r\n world\r\n0\r\nTrailer: x\r\n\r\nNEXT";
        let mut r = io::Cursor::new(&data[..]);
        let mut out = Vec::new();
        let n = read_body(&mut r, &Body::Chunked, &mut out, 100).unwrap();
        assert_eq!(n, 11);
        assert_eq!(out, b"hello world");
        let mut rest = String::new();
        r.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "NEXT");
    }

    #[test]
    fn chunked_limit_and_garbage() {
        let mut r = io::Cursor::new(&b"ff\r\n"[..]);
        assert!(matches!(
            read_body(&mut r, &Body::Chunked, &mut io::sink(), 10),
            Err(BodyErr::TooLarge)
        ));
        let mut r = io::Cursor::new(&b"zz\r\n"[..]);
        assert!(matches!(
            read_body(&mut r, &Body::Chunked, &mut io::sink(), 10),
            Err(BodyErr::Bad)
        ));
    }

    #[test]
    fn parses_request() {
        let raw = b"PUT /dav/a%20b.jpg?t=xyz HTTP/1.1\r\nHost: h\r\nContent-Length: 3\r\n\r\n";
        let mut r = io::Cursor::new(&raw[..]);
        let q = read_request(&mut r).unwrap().unwrap();
        assert_eq!(q.method, "PUT");
        assert_eq!(q.path, "/dav/a b.jpg");
        assert_eq!(q.query.get("t").unwrap(), "xyz");
        assert_eq!(q.h("content-length"), Some("3"));
    }
}
