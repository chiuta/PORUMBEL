//! Hand a session folder to LegatorOCR (`legatorocr-gui FOLDER` loads every image in it).

use crate::config::Config;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const GUI: &str = if cfg!(windows) {
    "legatorocr-gui.exe"
} else {
    "legatorocr-gui"
};

fn on_path(prog: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|d| d.join(prog))
        .find(|p| p.is_file())
}

/// Find the LegatorOCR window program: `launch` setting, `LEGATOROCR` variable, next to
/// PORUMBEL (also the usual portable layouts), then PATH.
pub fn find_legator(cfg: &Config) -> Option<PathBuf> {
    if let Some(l) = &cfg.launch {
        let p = PathBuf::from(l);
        if p.is_file() {
            return Some(p);
        }
        return on_path(l);
    }
    if let Some(v) = std::env::var_os("LEGATOROCR") {
        let p = PathBuf::from(v);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for rel in [
                "",
                "LegatorOCR",
                "../LegatorOCR",
                "../../LegatorOCR/App/LegatorOCR",
                "../LegatorOCRPortable/App/LegatorOCR",
            ] {
                let p = dir.join(rel).join(GUI);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    on_path(GUI)
}

pub fn describe(cfg: &Config) -> String {
    match find_legator(cfg) {
        Some(p) => p.display().to_string(),
        None => String::new(),
    }
}

fn detach(mut c: Command) -> std::io::Result<()> {
    c.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    c.creation_flags(0x0000_0008); // DETACHED_PROCESS
    let mut child = c.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Open a folder in the system file manager (fallback when LegatorOCR is not found).
pub fn open_in_file_manager(dir: &Path) -> std::io::Result<()> {
    let prog = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut c = Command::new(prog);
    c.arg(dir);
    detach(c)
}

/// Open a session folder in LegatorOCR. Returns a short message for the log.
pub fn open_session(cfg: &Config, dir: &Path) -> Result<String, String> {
    if let Some(prog) = find_legator(cfg) {
        let mut c = Command::new(&prog);
        c.args(&cfg.launch_args).arg(dir);
        return detach(c)
            .map(|_| format!("LegatorOCR ← {}", dir.display()))
            .map_err(|e| format!("{}: {e}", prog.display()));
    }
    #[cfg(target_os = "macos")]
    if Path::new("/Applications/LegatorOCR.app").exists() {
        let mut c = Command::new("open");
        c.args(["-a", "LegatorOCR", "--args"])
            .args(&cfg.launch_args)
            .arg(dir);
        return detach(c)
            .map(|_| format!("LegatorOCR ← {}", dir.display()))
            .map_err(|e| e.to_string());
    }
    match open_in_file_manager(dir) {
        Ok(()) => Ok(format!(
            "LegatorOCR not found (set `launch =` in the settings); opened the folder instead: {}",
            dir.display()
        )),
        Err(e) => Err(format!(
            "LegatorOCR not found and the file manager failed: {e}"
        )),
    }
}
