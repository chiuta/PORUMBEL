# Setup, step by step

**English** · [Română](SETUP.ro.md) · [Français](SETUP.fr.md) · [Italiano](SETUP.it.md) · [Español](SETUP.es.md) · [Português](SETUP.pt.md) · [Deutsch](SETUP.de.md) · [Magyar](SETUP.hu.md)

## 0. On the phone: Open Camera

Open Camera keeps its photos in `DCIM/OpenCamera` unless you change it (Settings → look for *Save location*;
the exact wording differs between versions). Two habits help for book scanning:

* Shoot as JPEG at the camera's full resolution, with even light and no flash — LegatorOCR reads JPEG, PNG,
  TIFF and BMP. Photos in other formats (WebP, HEIC, RAW/DNG) are not picked up on purpose.
* Keep one folder for the scan job if you like (e.g. `DCIM/Carti`). Then tell PORUMBEL where it is:
  `phone_dir = /sdcard/DCIM/Carti` (only ADB mode needs this; the other two follow whatever the sync app sends).

## 1. On the computer

```
porumbel            # first run creates the settings file and prints its path
porumbel qr         # QR code + address for the phone
```

Allow the program through the firewall for *private* networks when the system asks (port 8750).
Open the panel at the address printed at start (`http://localhost:8750/?t=…`).

Not sure everything is in place? `porumbel doctor` checks the folder, the port, the network address,
LegatorOCR and — with `--adb` — the phone, and says what to fix.

## 2. Pick a way to move the photos

### ①a Wi-Fi, from the phone's browser (nothing to install)

Phone and computer on the same Wi-Fi (not a guest network that isolates devices). Scan the QR code, tap
*Choose photos*, select the pages. Names are kept, so the session folder gets the shooting time.
Good for a handful of pages; for a whole book use ② or ③.

### ①b Wi-Fi, a sync app pushes the camera folder (automatic)

Any app that can upload a folder to a WebDAV server:

* address: `http://COMPUTER-IP:8750/dav/` (the panel shows the exact address)
* user name: anything; **password: the token** from the settings file
* folder to send: Open Camera's folder; direction: **upload only**, do not delete or mirror deletions;
  run on new files (or every few minutes).
* the app must be allowed to use plain `http://` on the local network.

Apps that first upload under a temporary name (`.tmp`, `.part`) and then rename (`MOVE`) work too.
*Not verified with a real app yet — please report what you find.*

### ② ADB (cable or Wi-Fi debugging) — fastest

1. Install Android *platform-tools* (`adb`) and put it on the `PATH` (or set `adb_path`).
2. On the phone: Settings → About phone → tap *Build number* seven times → Developer options → **USB debugging**.
3. Connect the cable, accept the "Allow USB debugging?" prompt, then `adb devices` must show `device`.
   (Android 11+: *Wireless debugging* with `adb pair` / `adb connect` works the same way.)
4. `porumbel --adb` — or put `adb = true` in the settings file. The first run leaves the photos that
   already exist alone (add `--adb-all` to import them); after that every new photo is copied as soon as
   Open Camera has finished writing it.
5. Optional: `adb_delete = true` removes each photo from the phone after the copy was verified by size.

### ③ A folder kept in sync (Syncthing and friends)

1. Phone: share Open Camera's folder as *send only* (Syncthing-Fork, for example).
2. Computer: accept it into a folder, e.g. `~/Syncthing/OpenCamera`.
3. `porumbel --watch ~/Syncthing/OpenCamera` (the line `watch = …` may be repeated for several folders).
   Files still being synced (`.syncthing.*.tmp`, hidden files) are ignored; a file is copied once its size
   and time stop changing. Leave `watch_move` off for receive-only folders.

## 3. Shoot

Keep going; pages arrive in order. When you pause for `session_gap` seconds (default 90) the folder is
complete and LegatorOCR opens with it. Earlier sessions stay in `inbox/` and in the panel's list.

## Start with the computer

Ready-made files are in `contrib/`:

* **Windows:** `powershell -ExecutionPolicy Bypass -File contrib\windows-autostart.ps1 "C:\Tools\porumbel.exe"`
  (a logon task, no admin rights) — or `Win+R` → `shell:startup` and drop a shortcut there.
* **Linux (systemd, per user):** copy `contrib/porumbel.service` to `~/.config/systemd/user/`, then
  `systemctl --user enable --now porumbel` (binary expected in `~/bin`; edit `ExecStart` otherwise).
* **macOS:** copy `contrib/ro.trom.porumbel.plist` to `~/Library/LaunchAgents/` (edit the path), then
  `launchctl load` it — or add `porumbel` under *System Settings → General → Login Items*.

## Trouble

| Symptom | Check |
|---|---|
| The phone cannot open the address | same Wi-Fi? guest network? firewall prompt answered? `porumbel qr` shows the first address found — if the computer has several, use the one on the phone's network |
| `401` / password asked | the token must match the one in the settings file |
| ADB: *phone not authorised* | accept the prompt on the phone; unplug/replug |
| ADB: *no such file or directory* | `phone_dir` does not match Open Camera's folder |
| LegatorOCR does not open, the folder does | set `launch = /path/to/legatorocr-gui` (or put it next to `porumbel`) |
| A photo is missing | only `jpg, jpeg, png, tif, tiff, bmp` are taken (`extensions`); see the log in the panel |
| Photos you deleted come back / do not come back | received photos are remembered by name+size: `porumbel forget` |
