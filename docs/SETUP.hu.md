# Beállítás lépésről lépésre

[English](SETUP.md) · [Română](SETUP.ro.md) · [Français](SETUP.fr.md) · [Italiano](SETUP.it.md) · [Español](SETUP.es.md) · [Português](SETUP.pt.md) · [Deutsch](SETUP.de.md) · **Magyar**

## 0. A telefonon: Open Camera

Az Open Camera a `DCIM/OpenCamera` mappába ment (*Mentési hely* beállítás; a neve verziótól függ). Könyvszkenneléshez: JPEG teljes felbontásban, egyenletes fény, vaku nélkül. A LegatorOCR JPEG, PNG, TIFF és BMP fájlokat olvas; a WebP, HEIC és RAW/DNG szándékosan kimarad.

Másik mappa esetén (pl. `DCIM/Konyvek`) adja meg: `phone_dir = /sdcard/DCIM/Konyvek` (csak az ADB módhoz kell).

## 1. A számítógépen

```
porumbel            # → settings file + panel
porumbel qr         # QR
porumbel doctor     # check
```

Az első indításkor létrejön a beállításfájl egy véletlen tokennel. Engedélyezze a programot a tűzfalban *privát* hálózatokon (8750-es port). Kétség esetén a `porumbel doctor` ellenőrzi a mappát, a portot, a hálózati címet, a LegatorOCR-t és (`--adb` mellett) a telefont.

## 2. Az átviteli mód kiválasztása

### ①a Wi‑Fi, a telefon böngészőjéből

A telefon és a számítógép ugyanazon a Wi‑Fi-n legyen (ne vendéghálózaton, amely elszigeteli az eszközöket). Olvassa be a QR-kódot, érintse meg a *Fotók kiválasztása* gombot. Pár oldalhoz jó; egész könyvhöz használja a ② vagy ③ módot.

### ①b Wi‑Fi, szinkronizáló alkalmazás küldi a mappát

Bármely alkalmazás, amely mappát tölt fel WebDAV-kiszolgálóra: cím `http://A-GEP-IP-JE:8750/dav/` (a panel mutatja), a felhasználónév tetszőleges, **jelszó = a token**, az Open Camera mappája, irány **csak feltöltés**, törlés nélkül. Engedélyezze a `http://` használatát a helyi hálózaton. *Valódi alkalmazással még nem próbáltuk — írja meg, mit tapasztal.*

### ② ADB (kábel vagy Wi‑Fi hibakeresés) — a leggyorsabb

1. Telepítse az Android *platform-tools* csomagot (`adb`) a `PATH`-ba (vagy `adb_path`).
2. Telefon: Beállítások → A telefonról → hétszer érintse meg a *Build-számot* → Fejlesztői beállítások → **USB-hibakeresés**.
3. Csatlakoztassa a kábelt, fogadja el a kérdést; az `adb devices` `device`-t kell mutasson.
4. `porumbel --adb`. A már meglévő fotókat békén hagyja (`--adb-all` behozza őket); utána minden új fotót átmásol, amint az Open Camera végzett az írással.
5. Opcionális: az `adb_delete = true` az ellenőrzött másolat után törli a fotót a telefonról.

### ③ Szinkronizált mappa (Syncthing és társai)

Telefon: osszza meg az Open Camera mappáját *csak küldésre*. Számítógép: fogadja el egy mappába, majd `porumbel --watch ~/Syncthing/OpenCamera`. A még szinkronizálás alatt álló fájlokat figyelmen kívül hagyja; egy fájlt akkor vesz át, ha mérete és ideje már nem változik.

## 3. Fényképezés

Folytassa; az oldalak sorrendben érkeznek. `session_gap` másodperc szünet után (alapértelmezés 90) a mappa kész, és megnyílik vele a LegatorOCR.

## Indítás a számítógéppel

Kész fájlok a `contrib/` mappában: Windows `contrib\windows-autostart.ps1`, Linux `contrib/porumbel.service` (systemd, felhasználónként), macOS `contrib/ro.trom.porumbel.plist` (launchd).

## Hibaelhárítás

| Tünet | Ellenőrizendő |
|---|---|
| A telefon nem nyitja meg a címet | ugyanaz a Wi‑Fi? vendéghálózat? tűzfal? a `porumbel qr` az első talált címet mutatja |
| `401` / jelszót kér | a tokennek egyeznie kell a beállításfájlban lévővel |
| ADB: *not authorised* | fogadja el a kérdést a telefonon; csatlakoztassa újra |
| ADB: *no such file* | a `phone_dir` nem egyezik az Open Camera mappájával |
| A LegatorOCR nem nyílik meg | `launch = /utvonal/legatorocr-gui` |
| Hiányzik egy fotó | csak `jpg, jpeg, png, tif, tiff, bmp` kerül át; nézze a panel naplóját |
| Fotók visszatérnek / nem térnek vissza | név+méret alapján megjegyzi: `porumbel forget` |
