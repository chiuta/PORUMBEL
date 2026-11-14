# PORUMBEL

[English](README.md) · **Română** · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Un porumbel-curier pentru scanările tale: duce pozele făcute cu **Open Camera** de pe telefon în
folderul de pe calculator, **automat**, câte un subfolder pentru fiecare sesiune de fotografiere, și
deschide fiecare sesiune încheiată în **[LegatorOCR](https://github.com/chiuta/LegatorOCR)**
(împărțire → îndreptare → tăiere → PDF cu OCR).

Un singur program mic, **fără biblioteci externe**, pentru Windows / macOS / Linux. Fără cont, fără cloud,
nimic nu iese din rețeaua ta.

```
 telefon (Open Camera)                      calculator
 ┌──────────────┐   ① Wi-Fi: WebDAV / pagină de încărcare ─┐
 │ DCIM/Open…   │   ② USB sau Wi-Fi: adb                   ├─► PORUMBEL ─► inbox/2026-11-14_13-55-01/ ─► LegatorOCR
 └──────────────┘   ③ folder sincronizat (Syncthing…)     ─┘            (un folder pe sesiune)
```

## Pornire rapidă

1. Rulați `porumbel` o dată. Creează fișierul de setări (calea se afișează) cu un token de acces aleator și
   pornește serverul local. LegatorOCR este găsit automat dacă `legatorocr-gui` se află lângă PORUMBEL sau în
   `PATH` (altfel setați `launch =`).
2. Alegeți **o** cale de transfer (pot funcționa toate trei deodată):

   | | Cum | Ce trebuie pe telefon | Potrivit pentru |
   |---|---|---|---|
   | ① **Wi-Fi** | `porumbel qr` → scanați codul → alegeți pozele în browser, **sau** lăsați o aplicație de sincronizare să trimită folderul camerei la adresa WebDAV | un browser sau o aplicație de sincronizare cu WebDAV (FolderSync, Round Sync, …) | fără cablu, fără configurare pe calculator |
   | ② **ADB** | `porumbel --adb` — ia pozele noi direct din folderul camerei, la câteva secunde | depanare USB activată, Android 6+ | cel mai rapid, complet automat, fără rețea |
   | ③ **Folder sincronizat** | `porumbel --watch ~/Syncthing/OpenCamera` | orice aplicație care sincronizează un folder cu calculatorul (Syncthing-Fork, …) | complet automat, pe Wi-Fi |

   Pașii pentru fiecare variantă, cu setările din Open Camera: [docs/SETUP.ro.md](docs/SETUP.ro.md).
3. Fotografiați paginile. Când vă opriți 90 de secunde (setarea `session_gap`), sesiunea se încheie și
   LegatorOCR se deschide cu folderul deja încărcat. Ordinea paginilor = ordinea fișierelor = ordinea fotografierii.

Panou de control (stare, cod QR, sesiuni, butoane „Deschide în LegatorOCR”, încărcare din browser):
`http://localhost:8750/?t=TOKENUL_DUMNEAVOASTRĂ` — adresa se afișează la pornire.

## Sesiuni și LegatorOCR

Pozele făcute la mai puțin de `session_gap` secunde una de alta ajung în același folder, numit după ora
primei poze (Open Camera numește fișierele `IMG_20261114_135501.jpg`; pentru alte nume se folosește ora UTC).
La încheierea sesiunii, PORUMBEL rulează `legatorocr-gui <folderul sesiunii>`, care încarcă toate imaginile din el.
LegatorOCR nu are o interfață prin care să adaugi fișiere într-o fereastră deja deschisă, deci fiecare sesiune
se deschide în fereastra ei; cu `open = manual` apăsați voi butonul din panou (sau `porumbel open-last`).
Dacă LegatorOCR nu este găsit, se deschide folderul în managerul de fișiere.

Pozele deja primite sunt ținute minte (nume + dimensiune): o aplicație de sincronizare care retrimite aceeași
poză primește „ok”, fără dubluri. `porumbel forget` șterge această memorie.

La prima rulare, modul ADB și urmărirea de folder lasă în pace pozele care **există deja** (`--adb-all` /
`--watch-all` le importă).

## Setări

Un fișier simplu `cheie = valoare`; parametrii din linia de comandă, cu aceleași nume, au prioritate
(`porumbel --help`). Tabelul cheilor este în [README.md](README.md#settings) (aceleași nume).

Fișierul de setări se caută lângă program (`porumbel.conf`, util pentru o copie portabilă), apoi în
`~/.config/porumbel/` (Linux), `~/Library/Application Support/porumbel/` (macOS) sau `%APPDATA%\porumbel\` (Windows).

## Securitate

* Serverul folosește HTTP simplu în rețeaua locală. Totul cere tokenul (parametrul `t`, antetul
  `X-Porumbel-Token` sau parola de la HTTP Basic). Tokenul circulă necriptat — folosiți o rețea Wi-Fi de încredere,
  sau modul ADB, sau `bind = 127.0.0.1`. **Nu** expuneți portul pe internet.
* Se păstrează doar imagini cu extensii acceptate; numele sunt reduse la un nume simplu de fișier (fără căi,
  fără nume rezervate); încărcarea se face într-un fișier temporar mutat la loc abia când e completă; limită de mărime.
* Butoanele din panou care schimbă ceva (deschide, încheie sesiunea, uită) cer tokenul în adresă sau antet, nu doar
  Basic, ca o pagină deschisă în alt tab să nu le poată declanșa.
* Modul ADB doar citește folderul camerei; `adb_delete` șterge o poză de pe telefon numai după ce copia de pe
  calculator are aceeași dimensiune.

## Compilare și teste

```
cargo build --release          # target/release/porumbel  (Rust 1.75+)
cargo test                     # 16 teste unitare + 6 de integrare
python3 tools/check-qr.py      # codul QR față de un cititor independent (pip install zxing-cpp pillow)
```

## Limite, spuse deschis

* Încă neîncercat cu un telefon real: partea de protocol e testată cu `curl` și socket-uri brute, ADB cu un
  script care ține locul lui `adb`. Aplicațiile de sincronizare diferă în felul în care vorbesc WebDAV; sunt
  acceptate cele care încarcă sub un nume temporar și apoi fac `MOVE`; `DELETE` nu este acceptat (`LOCK` primește un token nesemnificativ).
* Modul ADB cere Android 6 sau mai nou (folosește `find` și `stat` pe telefon).
* Căile de cod pentru Windows și macOS sunt verificate de workflow-ul CI, nu pe calculatorul autorului.
* `porumbel qr` desenează codul cu caractere-bloc; într-un terminal închis la culoare apare inversat și unele aplicații de scanare îl refuză — folosiți atunci imaginea QR din panou.
* Panoul vorbește aceleași opt limbi ca LegatorOCR; ghidul de configurare pas cu pas există doar în română și engleză.

## Licență

MIT — vezi [LICENSE](LICENSE). © 2026 Alexio.
