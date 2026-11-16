# Configurazione, passo per passo

[English](SETUP.md) · [Română](SETUP.ro.md) · [Français](SETUP.fr.md) · **Italiano** · [Español](SETUP.es.md) · [Português](SETUP.pt.md) · [Deutsch](SETUP.de.md) · [Magyar](SETUP.hu.md)

## 0. Sul telefono: Open Camera

Open Camera salva in `DCIM/OpenCamera` (impostazione *Posizione di salvataggio*, il nome varia con la versione). Per scansionare libri: JPEG a piena risoluzione, luce uniforme, senza flash. LegatorOCR legge JPEG, PNG, TIFF e BMP; WebP, HEIC e RAW/DNG sono ignorati di proposito.

Se usate un'altra cartella (es. `DCIM/Libri`) indicatela: `phone_dir = /sdcard/DCIM/Libri` (serve solo alla modalità ADB).

## 1. Sul computer

```
porumbel            # → settings file + panel
porumbel qr         # QR
porumbel doctor     # check
```

Al primo avvio viene creato il file delle impostazioni con un token casuale. Consentite il programma nel firewall per le reti *private* (porta 8750). Un dubbio? `porumbel doctor` controlla cartella, porta, indirizzo di rete, LegatorOCR e (con `--adb`) il telefono.

## 2. Scegliere come spostare le foto

### ①a Wi‑Fi, dal browser del telefono

Telefono e computer sulla stessa Wi‑Fi (non una rete ospiti che isola i dispositivi). Inquadrate il codice QR, toccate *Scegli foto*. Va bene per poche pagine; per un libro intero usate ② o ③.

### ①b Wi‑Fi, un'app di sincronizzazione invia la cartella

Qualsiasi app che carica una cartella su un server WebDAV: indirizzo `http://IP-DEL-COMPUTER:8750/dav/` (lo mostra il pannello), utente a piacere, **password = il token**, cartella di Open Camera, direzione **solo invio**, senza cancellazioni. Consentite `http://` nella rete locale. *Non ancora provato con un'app reale — segnalateci cosa osservate.*

### ② ADB (cavo o debug Wi‑Fi) — il più veloce

1. Installate le *platform-tools* Android (`adb`) nel `PATH` (oppure `adb_path`).
2. Telefono: Impostazioni → Info sul telefono → toccate sette volte *Numero build* → Opzioni sviluppatore → **Debug USB**.
3. Collegate il cavo, accettate la richiesta; `adb devices` deve mostrare `device`.
4. `porumbel --adb`. Le foto già presenti restano da parte (`--adb-all` per importarle); poi ogni nuova foto viene copiata appena Open Camera ha finito di scriverla.
5. Facoltativo: `adb_delete = true` toglie la foto dal telefono dopo la verifica della copia.

### ③ Una cartella sincronizzata (Syncthing e simili)

Telefono: condividete la cartella di Open Camera come *solo invio*. Computer: accettatela in una cartella, poi `porumbel --watch ~/Syncthing/OpenCamera`. I file ancora in sincronizzazione vengono ignorati; un file è preso quando dimensione e ora non cambiano più.

## 3. Fotografare

Continuate; le pagine arrivano in ordine. Dopo `session_gap` secondi di pausa (90 di default) la cartella è completa e si apre LegatorOCR.

## Avvio con il computer

File pronti in `contrib/`: Windows `contrib\windows-autostart.ps1`, Linux `contrib/porumbel.service` (systemd, per utente), macOS `contrib/ro.trom.porumbel.plist` (launchd).

## Problemi

| Sintomo | Da controllare |
|---|---|
| Il telefono non apre l'indirizzo | stessa Wi‑Fi? rete ospiti? firewall? `porumbel qr` mostra il primo indirizzo trovato |
| `401` / chiede la password | il token deve essere quello del file delle impostazioni |
| ADB: *not authorised* | accettate la richiesta sul telefono; ricollegate |
| ADB: *no such file* | `phone_dir` non coincide con la cartella di Open Camera |
| LegatorOCR non si apre | `launch = /percorso/legatorocr-gui` |
| Manca una foto | si prendono solo `jpg, jpeg, png, tif, tiff, bmp`; vedete il registro nel pannello |
| Foto che tornano / non tornano | ricordate per nome+dimensione: `porumbel forget` |
