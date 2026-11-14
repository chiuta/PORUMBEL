# PORUMBEL

**English** · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

*Porumbel* is Romanian for "pigeon" — a carrier pigeon for your scans.

Takes the photos you shoot with **Open Camera** on your phone, drops them into a folder on your
computer **automatically**, one sub-folder per shooting session, and opens each finished session in
**[LegatorOCR](https://github.com/chiuta/LegatorOCR)** (split → deskew → crop → PDF with OCR).

Single small program, **no third-party crates**, Windows / macOS / Linux. Trade-free: no account, no cloud,
nothing leaves your network.

```
 phone (Open Camera)                       computer
 ┌──────────────┐   ① Wi-Fi: WebDAV / upload page ─┐
 │ DCIM/Open…   │   ② USB or Wi-Fi: adb            ├─► PORUMBEL ─► inbox/2026-11-14_13-55-01/ ─► LegatorOCR
 └──────────────┘   ③ synced folder (Syncthing…)  ─┘            (one folder per session)
```

## Quick start

1. Run `porumbel` once. It creates its settings file (the path is printed) with a random access token
   and starts the local server. LegatorOCR is found automatically when `legatorocr-gui` sits next to
   PORUMBEL or on the `PATH` (otherwise set `launch =`).
2. Choose **one** way to move the photos (all three can run together):

   | | How | Needs on the phone | Good for |
   |---|---|---|---|
   | ① **Wi-Fi** | `porumbel qr` → scan the code → pick photos in the browser, **or** let a sync app push the camera folder to the WebDAV address | a browser, or a WebDAV-capable sync app (FolderSync, Round Sync, …) | no cable, no setup on the computer |
   | ② **ADB** | `porumbel --adb` — pulls new photos straight from the camera folder, every few seconds | USB debugging on, Android 6+ | fastest, fully automatic, no network |
   | ③ **Synced folder** | `porumbel --watch ~/Syncthing/OpenCamera` | any app that syncs a folder to the computer (Syncthing-Fork, …) | fully automatic, Wi-Fi |

   Step by step for each way, with the Open Camera settings: [docs/SETUP.md](docs/SETUP.md).
3. Shoot your pages. When you stop for 90 seconds (setting `session_gap`), the session is closed and
   LegatorOCR opens with that folder already loaded. Page order = file order = shooting order.

Control panel (status, QR code, sessions, "open in LegatorOCR" buttons, upload from a browser):
`http://localhost:8750/?t=YOUR_TOKEN` — the address is printed at start.

## Sessions and LegatorOCR

Photos taken less than `session_gap` seconds apart go into the same folder, named after the first
photo's time stamp (Open Camera names files `IMG_20261114_135501.jpg`; other names get the UTC time).
When a session ends, PORUMBEL runs `legatorocr-gui <session folder>`, which loads every image in it.
LegatorOCR has no "add to running window" interface, so each session opens in its own window; use
`open = manual` if you would rather press the button in the panel (or run `porumbel open-last`).
If LegatorOCR cannot be found the folder is opened in the file manager instead.

Already received photos are remembered (name + size): a sync app that uploads the same picture again is
answered "ok" and nothing is duplicated. `porumbel forget` clears that memory.

On the first run, ADB mode and folder watching leave the photos that are **already there** alone
(`--adb-all` / `--watch-all` import them).

## Settings

A plain `key = value` file; command-line flags of the same names override it (`porumbel --help`).

| key | default | meaning |
|---|---|---|
| `inbox` | `~/PORUMBEL` | destination folder |
| `token` | random | password for everything the server does |
| `serve`, `bind`, `port` | `true`, `0.0.0.0`, `8750` | LAN server (use `bind = 127.0.0.1` for local-only) |
| `max_upload_mb` | `300` | largest accepted photo |
| `adb`, `adb_path`, `adb_serial`, `phone_dir` | off, `adb`, any, `/sdcard/DCIM/OpenCamera` | ADB transport |
| `adb_interval`, `adb_all`, `adb_delete` | `3`, off, off | poll seconds; import old photos; remove from phone after a verified copy |
| `watch` (repeatable), `watch_move`, `watch_all`, `watch_interval` | – | synced-folder transport |
| `session_gap` | `90` | idle seconds that end a session |
| `open`, `launch`, `launch_args` | `auto`, auto-detect | when/how LegatorOCR is started |
| `extensions` | `jpg, jpeg, png, tif, tiff, bmp` | accepted image types (the ones LegatorOCR reads) |

The settings file is looked up next to the program (`porumbel.conf`, handy for a portable copy), else in
`~/.config/porumbel/` (Linux), `~/Library/Application Support/porumbel/` (macOS) or `%APPDATA%\porumbel\` (Windows).

## Security

* The server speaks plain HTTP on your local network. Everything needs the token (query `t`, header
  `X-Porumbel-Token`, or the password of HTTP Basic). The token travels unencrypted — use it on a Wi-Fi you trust,
  or use the ADB mode, or `bind = 127.0.0.1`. Do **not** expose the port to the internet.
* Only image files with accepted extensions are stored; names are reduced to a plain file name (no paths, no
  reserved names); uploads are written to a temporary file and moved into place only when complete; size limit.
* Buttons that change something in the panel (open, close session, forget) need the token in the query or
  header, not just Basic auth, so a page opened in another tab cannot trigger them.
* ADB mode only reads the camera folder; `adb_delete` removes a photo from the phone only after the copy on the
  computer has the same size.

## Build and test

```
cargo build --release          # target/release/porumbel  (Rust 1.75+)
cargo test                     # 16 unit + 6 integration tests
python3 tools/check-qr.py      # QR encoder against an independent reader (pip install zxing-cpp pillow)
```

The integration tests drive the real server over TCP (WebDAV, chunked uploads, token rules), run the ADB
transport against a stand-in `adb` script, the folder watcher against a temporary folder, and the launcher
against a stand-in `legatorocr-gui`.

## Honest limits

* Not yet tried with a real phone: the protocol side is tested with `curl` and raw sockets, ADB with a
  stand-in script. Sync apps differ in how they speak WebDAV; clients that upload through a temporary name and
  then `MOVE` are supported, `DELETE` is not (`LOCK` is answered with a harmless token).
* ADB mode needs Android 6 or newer (it uses `find` and `stat` on the phone).
* The Windows and macOS code paths are checked by the CI workflow, not on this author's machine.
* `porumbel qr` draws the code with block characters; on a dark terminal it appears inverted and some scanners refuse it — use the QR image in the panel then.
* The panel speaks the same eight languages as LegatorOCR; the step-by-step setup guide exists in Romanian and English only.

## License

MIT — see [LICENSE](LICENSE). © 2026 Alexio.
