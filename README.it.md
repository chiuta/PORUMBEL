# PORUMBEL

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · **Italiano** · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

*Un piccione viaggiatore per le tue scansioni.*

Porta automaticamente le foto scattate con **Open Camera** sul telefono in una cartella del computer, una sottocartella per sessione di scatto, e apre ogni sessione conclusa in **[LegatorOCR](https://github.com/chiuta/LegatorOCR)**. Un solo piccolo programma, **senza librerie esterne**, per Windows / macOS / Linux. Nessun account, nessun cloud.

## Avvio rapido

1. Eseguite `porumbel` una volta: crea il file delle impostazioni (il percorso viene mostrato) con un token casuale e avvia il server locale. LegatorOCR viene trovato automaticamente se `legatorocr-gui` è accanto a PORUMBEL o nel `PATH` (altrimenti `launch =`).
2. Scegliete un modo di trasferimento (tutti e tre possono funzionare insieme):
   * **Wi‑Fi**: `porumbel qr`, inquadrate il codice, scegliete le foto nel browser — oppure un'app di sincronizzazione invia la cartella della fotocamera all'indirizzo WebDAV.
   * **ADB**: `porumbel --adb` preleva le foto nuove direttamente dalla cartella della fotocamera (debug USB attivo, Android 6+).
   * **Cartella sincronizzata**: `porumbel --watch ~/Syncthing/OpenCamera` (Syncthing, ecc.).
3. Fotografate le pagine. Dopo 90 secondi di pausa (`session_gap`) la sessione si chiude e LegatorOCR si apre con la cartella già caricata.

## Sicurezza

Il server usa HTTP semplice sulla rete locale; tutto richiede il token. Usate un Wi‑Fi fidato, la modalità ADB o `bind = 127.0.0.1`, e non esponete mai la porta su Internet. Si conservano solo immagini con estensioni accettate, con un nome di file semplice.

Impostazioni complete, limiti e guide passo passo: [README.md](README.md) e [docs/SETUP.it.md](docs/SETUP.it.md) · [EN](docs/SETUP.md) · [RO](docs/SETUP.ro.md).

## Licenza

MIT — [LICENSE](LICENSE). © 2026 Alexio.
