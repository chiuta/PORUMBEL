//! PORUMBEL — carries photos from Open Camera on the phone into a folder on the
//! computer and hands them to LegatorOCR.

#![allow(clippy::field_reassign_with_default)]

pub mod adb;
pub mod config;
pub mod doctor;
pub mod hub;
pub mod launch;
pub mod qr;
pub mod server;
pub mod util;
pub mod watch;

use hub::Hub;

/// Open a session folder in LegatorOCR and remember that it was opened.
pub fn open_named(hub: &Hub, name: &str) -> Result<String, String> {
    let dir = hub
        .session_dir(name)
        .ok_or_else(|| format!("no such session: {name}"))?;
    match launch::open_session(&hub.cfg, &dir) {
        Ok(msg) => {
            hub.mark_opened(name);
            hub.log(&msg);
            Ok(msg)
        }
        Err(e) => {
            hub.log(format!("cannot open {name}: {e}"));
            Err(e)
        }
    }
}
