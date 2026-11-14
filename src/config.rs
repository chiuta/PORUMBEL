//! Settings: a plain `key = value` file plus command-line flags (flags win).

use crate::util;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenMode {
    /// Hand a session to LegatorOCR as soon as it is complete (no new photo for `session_gap` s).
    Auto,
    /// Only open when asked (panel button or `porumbel open`).
    Manual,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub inbox: PathBuf,
    pub token: String,
    pub bind: String,
    pub port: u16,
    pub serve: bool,
    pub max_upload_mb: u64,
    pub adb: bool,
    pub adb_path: String,
    pub adb_serial: Option<String>,
    pub phone_dir: String,
    pub adb_interval: u64,
    pub adb_all: bool,
    pub adb_delete: bool,
    pub watch: Vec<PathBuf>,
    pub watch_move: bool,
    pub watch_all: bool,
    pub watch_interval: u64,
    pub session_gap: u64,
    pub open: OpenMode,
    pub launch: Option<String>,
    pub launch_args: Vec<String>,
    pub extensions: Vec<String>,
    pub once: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    Run,
    Init,
    Qr,
    OpenLast,
    Forget,
    PrintConfig,
    Help,
    Version,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            inbox: home_dir().join("PORUMBEL"),
            token: String::new(),
            bind: "0.0.0.0".into(),
            port: 8750,
            serve: true,
            max_upload_mb: 300,
            adb: false,
            adb_path: "adb".into(),
            adb_serial: None,
            phone_dir: "/sdcard/DCIM/OpenCamera".into(),
            adb_interval: 3,
            adb_all: false,
            adb_delete: false,
            watch: Vec::new(),
            watch_move: false,
            watch_all: false,
            watch_interval: 3,
            session_gap: 90,
            open: OpenMode::Auto,
            launch: None,
            launch_args: Vec::new(),
            extensions: ["jpg", "jpeg", "png", "tif", "tiff", "bmp"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            once: false,
        }
    }
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn parse_bool(v: &str) -> Result<bool, String> {
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "da" | "on" => Ok(true),
        "0" | "false" | "no" | "nu" | "off" => Ok(false),
        o => Err(format!("not a yes/no value: {o}")),
    }
}

fn parse_num<T: std::str::FromStr>(k: &str, v: &str) -> Result<T, String> {
    v.trim()
        .parse::<T>()
        .map_err(|_| format!("{k}: not a valid number: {v}"))
}

impl Config {
    /// Apply one `key = value` (file) or `--key value` (flag) setting.
    pub fn set(&mut self, key: &str, val: &str) -> Result<(), String> {
        let key = key.trim().replace('-', "_");
        let val = val.trim();
        match key.as_str() {
            "inbox" => self.inbox = expand_home(val),
            "token" => self.token = val.to_string(),
            "bind" => self.bind = val.to_string(),
            "port" => self.port = parse_num(&key, val)?,
            "serve" => self.serve = parse_bool(val)?,
            "max_upload_mb" => self.max_upload_mb = parse_num::<u64>(&key, val)?.clamp(1, 4096),
            "adb" => self.adb = parse_bool(val)?,
            "adb_path" => self.adb_path = val.to_string(),
            "adb_serial" => {
                self.adb_serial = if val.is_empty() {
                    None
                } else {
                    Some(val.into())
                }
            }
            "phone_dir" => self.phone_dir = val.to_string(),
            "adb_interval" => self.adb_interval = parse_num::<u64>(&key, val)?.max(1),
            "adb_all" => self.adb_all = parse_bool(val)?,
            "adb_delete" => self.adb_delete = parse_bool(val)?,
            "watch" => {
                if !val.is_empty() {
                    self.watch.push(expand_home(val));
                }
            }
            "watch_move" => self.watch_move = parse_bool(val)?,
            "watch_all" => self.watch_all = parse_bool(val)?,
            "watch_interval" => self.watch_interval = parse_num::<u64>(&key, val)?.max(1),
            "session_gap" => self.session_gap = parse_num(&key, val)?,
            "open" => {
                self.open = match val.to_ascii_lowercase().as_str() {
                    "auto" => OpenMode::Auto,
                    "manual" | "never" => OpenMode::Manual,
                    o => return Err(format!("open: expected auto or manual, got {o}")),
                }
            }
            "launch" => {
                self.launch = if val.is_empty() {
                    None
                } else {
                    Some(val.into())
                }
            }
            "launch_args" => {
                self.launch_args = val.split_whitespace().map(|s| s.to_string()).collect()
            }
            "extensions" => {
                self.extensions = val
                    .split([',', ' ', ';'])
                    .map(|s| s.trim().trim_start_matches('.').to_ascii_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if self.extensions.is_empty() {
                    return Err("extensions: the list is empty".into());
                }
            }
            "once" => self.once = parse_bool(val)?,
            other => return Err(format!("unknown setting: {other}")),
        }
        Ok(())
    }

    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') || l.starts_with(';') {
                continue;
            }
            let Some((k, v)) = l.split_once('=') else {
                return Err(format!(
                    "{}:{}: expected key = value",
                    path.display(),
                    n + 1
                ));
            };
            self.set(k, v)
                .map_err(|e| format!("{}:{}: {e}", path.display(), n + 1))?;
        }
        Ok(())
    }

    /// Text of a fresh settings file (bilingual comments).
    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str("# PORUMBEL — setări / settings\n# Linii `cheie = valoare`; `#` = comentariu. Flags on the command line override this file.\n\n");
        s.push_str(&format!("# Folderul în care ajung pozele (câte un subfolder per sesiune)\n# Folder that receives the photos (one subfolder per session)\ninbox = {}\n\n", self.inbox.display()));
        s.push_str(&format!("# Parola de acces la server (nu o publicați) / access token for the server\ntoken = {}\n\n", self.token));
        s.push_str(&format!("# Server în rețeaua locală: WebDAV + pagină de încărcare + panou\n# LAN server: WebDAV + upload page + control panel\nserve = {}\nbind = {}\nport = {}\nmax_upload_mb = {}\n\n", self.serve, self.bind, self.port, self.max_upload_mb));
        s.push_str(&format!("# Transfer prin cablu/Wi-Fi ADB (depanare USB activată pe telefon)\n# Transfer over ADB (USB debugging on)\nadb = {}\nadb_path = {}\nadb_serial = {}\nphone_dir = {}\nadb_interval = {}\nadb_all = {}\nadb_delete = {}\n\n", self.adb, self.adb_path, self.adb_serial.clone().unwrap_or_default(), self.phone_dir, self.adb_interval, self.adb_all, self.adb_delete));
        s.push_str("# Folder sincronizat (Syncthing etc.); linia `watch` se poate repeta\n# Synced folder (Syncthing etc.); the `watch` line may be repeated\n# watch = /calea/către/folderul/sincronizat\n");
        s.push_str(&format!(
            "watch_move = {}\nwatch_all = {}\nwatch_interval = {}\n\n",
            self.watch_move, self.watch_all, self.watch_interval
        ));
        s.push_str(&format!("# Sesiune = poze făcute la intervale mai mici de `session_gap` secunde\n# Session = photos shot less than `session_gap` seconds apart\nsession_gap = {}\n# open = auto (deschide LegatorOCR când sesiunea s-a încheiat) | manual\nopen = {}\n", self.session_gap, if self.open == OpenMode::Auto { "auto" } else { "manual" }));
        s.push_str("# Programul LegatorOCR (gol = căutare automată: lângă PORUMBEL, în PATH)\nlaunch =\nlaunch_args =\n");
        s.push_str(&format!("extensions = {}\n", self.extensions.join(", ")));
        s
    }

    pub fn max_upload_bytes(&self) -> u64 {
        self.max_upload_mb * 1024 * 1024
    }
}

fn expand_home(v: &str) -> PathBuf {
    if let Some(rest) = v.strip_prefix("~/").or_else(|| v.strip_prefix("~\\")) {
        home_dir().join(rest)
    } else if v == "~" {
        home_dir()
    } else {
        PathBuf::from(v)
    }
}

/// Where the settings file lives: `--config`, else next to the program (portable use),
/// else the per-user configuration folder.
pub fn config_path(explicit: Option<&str>) -> PathBuf {
    if let Some(p) = explicit {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("porumbel.conf");
            if p.exists() {
                return p;
            }
        }
    }
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join("AppData"))
    } else if cfg!(target_os = "macos") {
        home_dir().join("Library").join("Application Support")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".config"))
    };
    base.join("porumbel").join("porumbel.conf")
}

/// Create the settings file (with a fresh random token) if it does not exist yet.
pub fn ensure_file(path: &Path) -> Result<bool, String> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let mut c = Config::default();
    c.token = util::hex(&util::random_bytes(16));
    std::fs::write(path, c.render()).map_err(|e| format!("{}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(true)
}

pub const HELP: &str = "\
PORUMBEL — carries photos from Open Camera (phone) into a folder on this computer and
hands them to LegatorOCR.  /  Duce pozele din Open Camera în folderul de pe calculator
și le deschide în LegatorOCR.

USAGE
  porumbel [options]            run (server, ADB and folder watchers as configured)
  porumbel init                 create the settings file and print where it is
  porumbel qr                   print the QR code for the phone and exit
  porumbel open-last            open the newest session in LegatorOCR and exit
  porumbel forget               forget which photos were already received
  porumbel print-config         show the effective settings

OPTIONS (same names as in the settings file)
  --config FILE        settings file            --inbox DIR      destination folder
  --port N  --bind IP  server address           --token TEXT     access password
  --no-serve           no LAN server            --max-upload-mb N
  --adb                pull over ADB            --adb-path PROG  --adb-serial ID
  --phone-dir PATH     default /sdcard/DCIM/OpenCamera
  --adb-interval S     --adb-all (also old photos)   --adb-delete (remove from phone after copy)
  --watch DIR          watch a synced folder (repeatable)
  --watch-move         move instead of copy     --watch-all (also old files)  --watch-interval S
  --session-gap S      idle seconds that end a session (default 90)
  --open auto|manual   open LegatorOCR automatically or only on request
  --launch PROG        LegatorOCR program       --launch-args \"ARGS\"
  --extensions LIST    default jpg,jpeg,png,tif,tiff,bmp
  --once               one pass of ADB/watch, then exit
  -h, --help   -V, --version
";

/// Parse command line over the settings. Returns the command to run.
pub fn apply_args(cfg: &mut Config, args: &[String]) -> Result<Cmd, String> {
    let mut cmd = Cmd::Run;
    let mut i = 0;
    const BOOLS: [&str; 6] = [
        "adb",
        "adb-all",
        "adb-delete",
        "watch-move",
        "watch-all",
        "once",
    ];
    while i < args.len() {
        let a = &args[i];
        if let Some(flag) = a.strip_prefix("--") {
            if flag == "help" {
                return Ok(Cmd::Help);
            }
            if flag == "version" {
                return Ok(Cmd::Version);
            }
            if flag == "config" {
                i += 2; // handled before loading
                continue;
            }
            if flag.starts_with("config=") {
                i += 1;
                continue;
            }
            if flag == "no-serve" {
                cfg.serve = false;
            } else if let Some((k, v)) = flag.split_once('=') {
                cfg.set(k, v)?;
            } else if BOOLS.contains(&flag) {
                cfg.set(flag, "true")?;
            } else {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| format!("--{flag} needs a value"))?;
                cfg.set(flag, v)?;
                i += 1;
            }
        } else {
            match a.as_str() {
                "-h" => return Ok(Cmd::Help),
                "-V" => return Ok(Cmd::Version),
                "init" => cmd = Cmd::Init,
                "qr" => cmd = Cmd::Qr,
                "open-last" => cmd = Cmd::OpenLast,
                "forget" => cmd = Cmd::Forget,
                "print-config" => cmd = Cmd::PrintConfig,
                other => return Err(format!("unknown argument: {other}  (try --help)")),
            }
        }
        i += 1;
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_and_flags() {
        let mut c = Config::default();
        c.set("port", "9000").unwrap();
        c.set("watch", "/a").unwrap();
        c.set("watch", "/b").unwrap();
        assert_eq!(c.port, 9000);
        assert_eq!(c.watch.len(), 2);
        assert!(c.set("port", "x").is_err());
        assert!(c.set("nope", "1").is_err());
        let cmd = apply_args(
            &mut c,
            &[
                "--adb".into(),
                "--session-gap".into(),
                "5".into(),
                "--no-serve".into(),
                "qr".into(),
            ],
        )
        .unwrap();
        assert_eq!(cmd, Cmd::Qr);
        assert!(c.adb && !c.serve && c.session_gap == 5);
    }

    #[test]
    fn rendered_file_reloads() {
        let mut c = Config::default();
        c.token = "abc".into();
        c.adb = true;
        let dir = std::env::temp_dir().join(format!("porumbel-cfg-{}", util::now_unix()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("p.conf");
        std::fs::write(&p, c.render()).unwrap();
        let mut d = Config::default();
        d.load_file(&p).unwrap();
        assert_eq!(d.token, "abc");
        assert!(d.adb);
        assert_eq!(d.phone_dir, c.phone_dir);
        let _ = std::fs::remove_dir_all(dir);
    }
}
