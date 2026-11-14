# PORUMBEL

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · **Deutsch** · [Magyar](README.hu.md)

*Eine Brieftaube für Ihre Scans.*

Bringt Fotos, die mit **Open Camera** auf dem Telefon entstehen, automatisch in einen Ordner auf dem Computer – ein Unterordner pro Aufnahmesitzung – und öffnet jede abgeschlossene Sitzung in **[LegatorOCR](https://github.com/chiuta/LegatorOCR)**. Ein einziges kleines Programm, **ohne externe Bibliotheken**, für Windows / macOS / Linux. Kein Konto, keine Cloud.

## Schnellstart

1. Starten Sie `porumbel` einmal: Es legt die Einstellungsdatei an (der Pfad wird angezeigt), mit einem zufälligen Token, und startet den lokalen Server. LegatorOCR wird automatisch gefunden, wenn `legatorocr-gui` neben PORUMBEL oder im `PATH` liegt (sonst `launch =`).
2. Wählen Sie einen Übertragungsweg (alle drei können zusammen laufen):
   * **WLAN**: `porumbel qr`, Code scannen, Fotos im Browser auswählen – oder eine Sync-App schickt den Kameraordner an die WebDAV-Adresse.
   * **ADB**: `porumbel --adb` holt neue Fotos direkt aus dem Kameraordner (USB-Debugging an, Android 6+).
   * **Synchronisierter Ordner**: `porumbel --watch ~/Syncthing/OpenCamera` (Syncthing usw.).
3. Fotografieren Sie die Seiten. Nach 90 Sekunden Pause (`session_gap`) ist die Sitzung abgeschlossen und LegatorOCR öffnet sich mit dem geladenen Ordner.

## Sicherheit

Der Server spricht einfaches HTTP im lokalen Netz; alles verlangt das Token. Nutzen Sie ein vertrauenswürdiges WLAN, den ADB-Modus oder `bind = 127.0.0.1`, und setzen Sie den Port nie dem Internet aus. Gespeichert werden nur Bilder mit zugelassenen Endungen unter einem schlichten Dateinamen.

Alle Einstellungen, Grenzen und Schritt-für-Schritt-Anleitungen: [README.md](README.md) und [docs/SETUP.md](docs/SETUP.md) (Englisch), [docs/SETUP.ro.md](docs/SETUP.ro.md) (Rumänisch).

## Lizenz

MIT — [LICENSE](LICENSE). © 2026 Alexio.
