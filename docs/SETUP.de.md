# Einrichtung, Schritt für Schritt

[English](SETUP.md) · [Română](SETUP.ro.md) · [Français](SETUP.fr.md) · [Italiano](SETUP.it.md) · [Español](SETUP.es.md) · [Português](SETUP.pt.md) · **Deutsch** · [Magyar](SETUP.hu.md)

## 0. Am Handy: Open Camera

Open Camera speichert in `DCIM/OpenCamera` (Einstellung *Speicherort*, die Bezeichnung hängt von der Version ab). Für Buchscans: JPEG in voller Auflösung, gleichmäßiges Licht, ohne Blitz. LegatorOCR liest JPEG, PNG, TIFF und BMP; WebP, HEIC und RAW/DNG werden absichtlich übergangen.

Bei einem anderen Ordner (z. B. `DCIM/Buecher`) tragen Sie ihn ein: `phone_dir = /sdcard/DCIM/Buecher` (nur der ADB-Modus braucht das).

## 1. Am Computer

```
porumbel            # → settings file + panel
porumbel qr         # QR
porumbel doctor     # check
```

Beim ersten Start wird die Einstellungsdatei mit einem zufälligen Token angelegt. Erlauben Sie das Programm in der Firewall für *private* Netze (Port 8750). Unsicher? `porumbel doctor` prüft Ordner, Port, Netzwerkadresse, LegatorOCR und (mit `--adb`) das Handy.

## 2. Den Übertragungsweg wählen

### ①a WLAN, über den Browser des Handys

Handy und Computer im selben WLAN (kein Gastnetz, das Geräte trennt). QR-Code scannen, *Fotos auswählen* tippen. Gut für wenige Seiten; für ein ganzes Buch ② oder ③ nehmen.

### ①b WLAN, eine Sync-App schickt den Ordner

Jede App, die einen Ordner auf einen WebDAV-Server lädt: Adresse `http://COMPUTER-IP:8750/dav/` (das Panel zeigt sie), beliebiger Benutzername, **Passwort = der Token**, Ordner von Open Camera, Richtung **nur hochladen**, nichts löschen. `http://` im lokalen Netz zulassen. *Mit einer echten App noch nicht getestet — berichten Sie, was Sie beobachten.*

### ② ADB (Kabel oder WLAN-Debugging) — am schnellsten

1. Android-*platform-tools* (`adb`) im `PATH` installieren (oder `adb_path`).
2. Handy: Einstellungen → Über das Telefon → siebenmal auf *Build-Nummer* tippen → Entwickleroptionen → **USB-Debugging**.
3. Kabel anstecken, Abfrage bestätigen; `adb devices` muss `device` zeigen.
4. `porumbel --adb`. Vorhandene Fotos bleiben unberührt (`--adb-all` importiert sie); danach wird jedes neue Foto kopiert, sobald Open Camera es fertig geschrieben hat.
5. Optional: `adb_delete = true` löscht das Foto nach geprüfter Kopie vom Handy.

### ③ Ein synchronisierter Ordner (Syncthing & Co.)

Handy: Open-Camera-Ordner als *nur senden* freigeben. Computer: in einen Ordner annehmen, dann `porumbel --watch ~/Syncthing/OpenCamera`. Dateien, die noch synchronisiert werden, werden ignoriert; eine Datei wird übernommen, wenn sich Größe und Zeit nicht mehr ändern.

## 3. Fotografieren

Einfach weitermachen; die Seiten kommen der Reihe nach an. Nach `session_gap` Sekunden Pause (Standard 90) ist der Ordner komplett und LegatorOCR öffnet sich damit.

## Mit dem Computer starten

Fertige Dateien in `contrib/`: Windows `contrib\windows-autostart.ps1`, Linux `contrib/porumbel.service` (systemd, pro Benutzer), macOS `contrib/ro.trom.porumbel.plist` (launchd).

## Probleme

| Symptom | Prüfen |
|---|---|
| Das Handy öffnet die Adresse nicht | gleiches WLAN? Gastnetz? Firewall? `porumbel qr` zeigt die erste gefundene Adresse |
| `401` / Passwortabfrage | der Token muss dem in der Einstellungsdatei entsprechen |
| ADB: *not authorised* | Abfrage am Handy bestätigen; neu anstecken |
| ADB: *no such file* | `phone_dir` passt nicht zum Open-Camera-Ordner |
| LegatorOCR öffnet sich nicht | `launch = /pfad/legatorocr-gui` |
| Ein Foto fehlt | nur `jpg, jpeg, png, tif, tiff, bmp` werden übernommen; Protokoll im Panel ansehen |
| Fotos kommen wieder / nicht wieder | gemerkt nach Name+Größe: `porumbel forget` |
