//! Small helpers: time, names, randomness, base64, percent-decoding, LAN addresses.
//! Everything here is std-only.

use std::net::{IpAddr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// UTC civil date/time from a Unix timestamp (Howard Hinnant's algorithm).
pub fn civil_from_unix(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        ((rem % 3600) / 60) as u32,
        (rem % 60) as u32,
    )
}

/// `2026-11-14_13-55-01` (UTC).
pub fn stamp_utc(secs: u64) -> String {
    let (y, m, d, hh, mm, ss) = civil_from_unix(secs);
    format!("{y:04}-{m:02}-{d:02}_{hh:02}-{mm:02}-{ss:02}")
}

/// RFC 1123 date for HTTP / WebDAV headers.
pub fn http_date(secs: u64) -> String {
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (y, m, d, hh, mm, ss) = civil_from_unix(secs);
    let wd = ((4 + (secs / 86_400)) % 7) as usize; // 1970-01-01 was a Thursday
    format!(
        "{}, {d:02} {} {y:04} {hh:02}:{mm:02}:{ss:02} GMT",
        DAYS[wd],
        MONTHS[(m - 1) as usize]
    )
}

/// Open Camera names files `IMG_20261114_135501.jpg` (phone local time). When the name
/// carries such a stamp, return it as `2026-11-14_13-55-01` so the session folder
/// shows the real time the first page was shot.
pub fn parse_camera_stamp(name: &str) -> Option<String> {
    let b = name.as_bytes();
    if b.len() < 15 {
        return None;
    }
    for i in 0..=(b.len() - 15) {
        let dig = |r: std::ops::Range<usize>| b[r].iter().all(|c| c.is_ascii_digit());
        if dig(i..i + 8) && (b[i + 8] == b'_' || b[i + 8] == b'-') && dig(i + 9..i + 15) {
            let n = |r: std::ops::Range<usize>| -> u32 {
                std::str::from_utf8(&b[r])
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(99_999)
            };
            let (y, mo, d) = (n(i..i + 4), n(i + 4..i + 6), n(i + 6..i + 8));
            let (h, mi, s) = (n(i + 9..i + 11), n(i + 11..i + 13), n(i + 13..i + 15));
            if (1990..=2100).contains(&y)
                && (1..=12).contains(&mo)
                && (1..=31).contains(&d)
                && h < 24
                && mi < 60
                && s < 61
            {
                return Some(format!("{y:04}-{mo:02}-{d:02}_{h:02}-{mi:02}-{s:02}"));
            }
        }
    }
    None
}

/// Reduce an untrusted file name to a safe single path component.
/// Returns `None` when nothing usable is left.
pub fn sanitize_name(raw: &str) -> Option<String> {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let mut out = String::new();
    for c in base.chars() {
        if c.is_control() {
            continue;
        }
        if c.is_alphanumeric() || " -_.()+,@#".contains(c) {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    let out = out.trim_matches(|c| c == ' ' || c == '.').to_string();
    if out.is_empty() {
        return None;
    }
    // keep the extension when shortening
    let out = if out.len() > 150 {
        let (stem, ext) = match out.rfind('.') {
            Some(p) if out.len() - p <= 9 => (&out[..p], &out[p..]),
            _ => (out.as_str(), ""),
        };
        let mut cut = 150usize.saturating_sub(ext.len());
        while !stem.is_char_boundary(cut) {
            cut -= 1;
        }
        format!("{}{}", &stem[..cut], ext)
    } else {
        out
    };
    // Windows reserved device names
    let stem_up = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem_up.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem_up.len() == 4
            && (stem_up.starts_with("COM") || stem_up.starts_with("LPT"))
            && stem_up.as_bytes()[3].is_ascii_digit());
    Some(if reserved { format!("_{out}") } else { out })
}

pub fn ext_of(name: &str) -> String {
    match name.rfind('.') {
        Some(p) => name[p + 1..].to_ascii_lowercase(),
        None => String::new(),
    }
}

pub fn has_ext_in(name: &str, list: &[String]) -> bool {
    let e = ext_of(name);
    !e.is_empty() && list.contains(&e)
}

pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let h = (b[i + 1] as char).to_digit(16);
            let l = (b[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (h, l) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn percent_encode(s: &str) -> String {
    let mut o = String::new();
    for &c in s.as_bytes() {
        if c.is_ascii_alphanumeric() || b"-_.~".contains(&c) {
            o.push(c as char);
        } else {
            o.push_str(&format!("%{c:02X}"));
        }
    }
    o
}

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Constant-time comparison (length is not secret here).
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\r' | b'\n' | b' ' => continue,
            _ => return None,
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Cryptographically secure random bytes from the operating system.
pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    if fill_os(&mut buf) {
        return buf;
    }
    // Last resort: std's randomly keyed SipHash mixed with time and pid.
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    for (i, chunk) in buf.chunks_mut(8).enumerate() {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(now_nanos() ^ (i as u64));
        h.write_u32(std::process::id());
        let v = h.finish().to_le_bytes();
        for (d, s) in chunk.iter_mut().zip(v) {
            *d = s;
        }
    }
    buf
}

#[cfg(unix)]
fn fill_os(buf: &mut [u8]) -> bool {
    use std::io::Read;
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(buf))
        .is_ok()
}

#[cfg(windows)]
fn fill_os(buf: &mut [u8]) -> bool {
    #[link(name = "bcrypt")]
    extern "system" {
        fn BCryptGenRandom(h: *mut core::ffi::c_void, b: *mut u8, n: u32, flags: u32) -> i32;
    }
    // BCRYPT_USE_SYSTEM_PREFERRED_RNG = 2
    unsafe { BCryptGenRandom(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32, 2) == 0 }
}

#[cfg(not(any(unix, windows)))]
fn fill_os(_buf: &mut [u8]) -> bool {
    false
}

/// Addresses of this computer on the local network. Uses the "connect a UDP socket"
/// trick: nothing is sent, the OS just picks the outgoing interface.
pub fn lan_ips() -> Vec<IpAddr> {
    let mut v: Vec<IpAddr> = Vec::new();
    for target in [
        "192.168.0.1:9",
        "192.168.1.1:9",
        "10.0.0.1:9",
        "172.16.0.1:9",
        "192.0.2.1:9",
    ] {
        if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
            if s.connect(target).is_ok() {
                if let Ok(a) = s.local_addr() {
                    let ip = a.ip();
                    if !ip.is_loopback() && !ip.is_unspecified() && !v.contains(&ip) {
                        v.push(ip);
                    }
                }
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(stamp_utc(0), "1970-01-01_00-00-00");
        assert_eq!(stamp_utc(1_794_664_501), "2026-11-14_13-55-01");
        assert_eq!(http_date(0), "Thu, 01 Jan 1970 00:00:00 GMT");
        assert_eq!(http_date(1_794_664_501), "Sat, 14 Nov 2026 13:55:01 GMT");
    }

    #[test]
    fn camera_stamp() {
        assert_eq!(
            parse_camera_stamp("IMG_20261114_135501.jpg").as_deref(),
            Some("2026-11-14_13-55-01")
        );
        assert_eq!(
            parse_camera_stamp("IMG_20261114_135501_1.jpg").as_deref(),
            Some("2026-11-14_13-55-01")
        );
        assert!(parse_camera_stamp("IMG_20269914_135501.jpg").is_none());
        assert!(parse_camera_stamp("photo.jpg").is_none());
    }

    #[test]
    fn names() {
        assert_eq!(sanitize_name("../../etc/passwd").as_deref(), Some("passwd"));
        assert_eq!(
            sanitize_name("C:\\x\\IMG_1.jpg").as_deref(),
            Some("IMG_1.jpg")
        );
        assert_eq!(sanitize_name("..").as_deref(), None);
        assert_eq!(sanitize_name("").as_deref(), None);
        assert_eq!(sanitize_name("a:b*c?.jpg").as_deref(), Some("a_b_c_.jpg"));
        assert_eq!(sanitize_name("CON.jpg").as_deref(), Some("_CON.jpg"));
        assert_eq!(sanitize_name("a\u{0}b.jpg").as_deref(), Some("ab.jpg"));
        let long = format!("{}.jpg", "x".repeat(400));
        let s = sanitize_name(&long).unwrap();
        assert!(s.len() <= 150 && s.ends_with(".jpg"));
    }

    #[test]
    fn pct() {
        assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_encode("a b/é"), "a%20b%2F%C3%A9");
    }

    #[test]
    fn b64() {
        assert_eq!(base64_decode("dXNlcjpwYXNz").unwrap(), b"user:pass");
        assert_eq!(base64_decode("dXNlcjpwYXNzMQ==").unwrap(), b"user:pass1");
    }

    #[test]
    fn rnd() {
        assert_ne!(random_bytes(16), random_bytes(16));
    }
}
