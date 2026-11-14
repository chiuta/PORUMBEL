# PORUMBEL

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · **Magyar**

*Postagalamb a szkenneléseihez.*

Automatikusan átviszi a telefonon az **Open Camera** alkalmazással készült fotókat a számítógép egy mappájába, fényképezési munkamenetenként egy almappába, és minden lezárt munkamenetet megnyit a **[LegatorOCR](https://github.com/chiuta/LegatorOCR)**-ben. Egyetlen kis program, **külső könyvtárak nélkül**, Windowsra / macOS-re / Linuxra. Nincs fiók, nincs felhő.

## Gyors kezdés

1. Futtassa egyszer a `porumbel` programot: létrehozza a beállításfájlt (az útvonalat kiírja) egy véletlenszerű tokennel, és elindítja a helyi szervert. A LegatorOCR-t automatikusan megtalálja, ha a `legatorocr-gui` a PORUMBEL mellett vagy a `PATH`-ban van (különben `launch =`).
2. Válasszon átviteli módot (mindhárom futhat egyszerre):
   * **Wi‑Fi**: `porumbel qr`, olvassa be a kódot, válassza ki a fotókat a böngészőben — vagy egy szinkronizáló alkalmazás elküldi a fényképezőgép mappáját a WebDAV-címre.
   * **ADB**: a `porumbel --adb` közvetlenül a kameramappából veszi az új fotókat (USB-hibakeresés bekapcsolva, Android 6+).
   * **Szinkronizált mappa**: `porumbel --watch ~/Syncthing/OpenCamera` (Syncthing stb.).
3. Fényképezze le az oldalakat. 90 másodperc szünet után (`session_gap`) a munkamenet lezárul, és a LegatorOCR megnyílik a már betöltött mappával.

## Biztonság

A szerver egyszerű HTTP-n beszél a helyi hálózaton; minden a tokent kéri. Használjon megbízható Wi‑Fit, ADB módot vagy `bind = 127.0.0.1` beállítást, és soha ne tegye ki a portot az internetnek. Csak az engedélyezett kiterjesztésű képek maradnak meg, egyszerű fájlnévvel.

Teljes beállítások, korlátok és lépésenkénti útmutatók: [README.md](README.md) és [docs/SETUP.md](docs/SETUP.md) (angol), [docs/SETUP.ro.md](docs/SETUP.ro.md) (román).

## Licenc

MIT — [LICENSE](LICENSE). © 2026 Alexio.
