# Configurare, pas cu pas

[English](SETUP.md) · **Română** · [Français](SETUP.fr.md) · [Italiano](SETUP.it.md) · [Español](SETUP.es.md) · [Português](SETUP.pt.md) · [Deutsch](SETUP.de.md) · [Magyar](SETUP.hu.md)

## 0. Pe telefon: Open Camera

Open Camera își ține pozele în `DCIM/OpenCamera`, dacă nu schimbați (Setări → căutați *Save location*;
formularea exactă diferă între versiuni). Două obiceiuri ajută la scanarea cărților:

* Fotografiați JPEG la rezoluția maximă, cu lumină uniformă și fără bliț — LegatorOCR citește JPEG, PNG,
  TIFF și BMP. Alte formate (WebP, HEIC, RAW/DNG) nu sunt preluate, intenționat.
* Dacă doriți, țineți un folder separat pentru lucrare (de ex. `DCIM/Carti`) și spuneți-i lui PORUMBEL unde
  este: `phone_dir = /sdcard/DCIM/Carti` (doar modul ADB are nevoie de asta; celelalte două urmează ce trimite
  aplicația de sincronizare).

## 1. Pe calculator

```
porumbel            # prima rulare creează fișierul de setări și îi afișează calea
porumbel qr         # cod QR + adresa pentru telefon
```

Permiteți programul în firewall pentru rețele *private* când sistemul întreabă (portul 8750).
Deschideți panoul la adresa afișată la pornire (`http://localhost:8750/?t=…`).

Nu sunteți siguri că totul e la locul lui? `porumbel doctor` verifică folderul, portul, adresa de rețea,
LegatorOCR și — cu `--adb` — telefonul, și spune ce trebuie reparat.

## 2. Alegeți calea de transfer

### ①a Wi-Fi, din browserul telefonului (nu instalați nimic)

Telefonul și calculatorul în aceeași rețea Wi-Fi (nu o rețea pentru oaspeți care izolează dispozitivele).
Scanați codul QR, apăsați *Alegeți pozele*, selectați paginile. Numele se păstrează, deci folderul sesiunii
primește ora fotografierii. Potrivit pentru câteva pagini; pentru o carte întreagă folosiți ② sau ③.

### ①b Wi-Fi, o aplicație de sincronizare trimite folderul camerei (automat)

Orice aplicație care poate încărca un folder pe un server WebDAV:

* adresa: `http://IP-CALCULATOR:8750/dav/` (panoul arată adresa exactă)
* utilizator: oricare; **parola: tokenul** din fișierul de setări
* folderul trimis: cel al Open Camera; sensul: **doar încărcare**, fără ștergeri oglindite;
  la fișiere noi (sau la câteva minute).
* aplicația trebuie să aibă voie să folosească `http://` simplu în rețeaua locală.

Merg și aplicațiile care încarcă mai întâi sub un nume temporar (`.tmp`, `.part`) și apoi redenumesc (`MOVE`).
*Încă neverificat cu o aplicație reală — vă rog să raportați ce observați.*

### ② ADB (cablu sau depanare Wi-Fi) — cel mai rapid

1. Instalați Android *platform-tools* (`adb`) și puneți-l în `PATH` (sau setați `adb_path`).
2. Pe telefon: Setări → Despre telefon → apăsați de șapte ori *Număr build* → Opțiuni dezvoltator → **Depanare USB**.
3. Conectați cablul, acceptați „Permiteți depanarea USB?”, apoi `adb devices` trebuie să arate `device`.
   (Android 11+: *Depanare wireless* cu `adb pair` / `adb connect` funcționează la fel.)
4. `porumbel --adb` — sau `adb = true` în fișierul de setări. Prima rulare lasă în pace pozele care există
   deja (adăugați `--adb-all` ca să le importați); apoi fiecare poză nouă e copiată imediat ce Open Camera
   a terminat de scris.
5. Opțional: `adb_delete = true` șterge fiecare poză de pe telefon după ce copia a fost verificată după dimensiune.

### ③ Un folder ținut în sincron (Syncthing și altele)

1. Telefon: partajați folderul Open Camera ca *doar trimitere* (de ex. Syncthing-Fork).
2. Calculator: acceptați-l într-un folder, de ex. `~/Syncthing/OpenCamera`.
3. `porumbel --watch ~/Syncthing/OpenCamera` (linia `watch = …` se poate repeta pentru mai multe foldere).
   Fișierele încă în sincronizare (`.syncthing.*.tmp`, fișiere ascunse) sunt ignorate; un fișier se copiază
   când dimensiunea și ora lui nu se mai schimbă. Lăsați `watch_move` oprit pentru foldere doar-primire.

## 3. Fotografiați

Continuați; paginile sosesc în ordine. Când faceți pauză `session_gap` secunde (implicit 90), folderul e
complet și LegatorOCR se deschide cu el. Sesiunile vechi rămân în `inbox/` și în lista din panou.

## Pornire odată cu calculatorul

Fișiere gata făcute în `contrib/`:

* **Windows:** `powershell -ExecutionPolicy Bypass -File contrib\windows-autostart.ps1 "C:\Tools\porumbel.exe"`
  (sarcină la autentificare, fără drepturi de administrator) — sau `Win+R` → `shell:startup` și o scurtătură.
* **Linux (systemd, per utilizator):** copiați `contrib/porumbel.service` în `~/.config/systemd/user/`, apoi
  `systemctl --user enable --now porumbel` (binarul se așteaptă în `~/bin`; altfel editați `ExecStart`).
* **macOS:** copiați `contrib/ro.trom.porumbel.plist` în `~/Library/LaunchAgents/` (editați calea), apoi
  `launchctl load` — sau adăugați `porumbel` în *Setări sistem → General → Elemente de autentificare*.

## Probleme

| Simptom | De verificat |
|---|---|
| Telefonul nu deschide adresa | aceeași rețea Wi-Fi? rețea pentru oaspeți? firewall? `porumbel qr` arată prima adresă găsită — dacă calculatorul are mai multe, folosiți-o pe cea din rețeaua telefonului |
| `401` / se cere parola | tokenul trebuie să fie cel din fișierul de setări |
| ADB: *phone not authorised* | acceptați mesajul de pe telefon; scoateți și reconectați cablul |
| ADB: *no such file or directory* | `phone_dir` nu coincide cu folderul Open Camera |
| LegatorOCR nu se deschide, se deschide folderul | setați `launch = /calea/către/legatorocr-gui` (sau puneți-l lângă `porumbel`) |
| Lipsește o poză | se preiau doar `jpg, jpeg, png, tif, tiff, bmp` (`extensions`); vedeți jurnalul din panou |
| Pozele șterse revin / nu revin | pozele primite sunt ținute minte după nume+dimensiune: `porumbel forget` |
