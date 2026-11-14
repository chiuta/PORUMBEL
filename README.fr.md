# PORUMBEL

[English](README.md) · [Română](README.ro.md) · **Français** · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

*Un pigeon voyageur pour vos scans.*

Envoie automatiquement les photos prises avec **Open Camera** sur le téléphone vers un dossier de l'ordinateur, un sous-dossier par session de prise de vue, puis ouvre chaque session terminée dans **[LegatorOCR](https://github.com/chiuta/LegatorOCR)**. Un seul petit programme, **sans bibliothèque externe**, pour Windows / macOS / Linux. Aucun compte, aucun cloud.

## Démarrage rapide

1. Lancez `porumbel` une fois : il crée le fichier de réglages (chemin affiché) avec un jeton aléatoire et démarre le serveur local. LegatorOCR est trouvé automatiquement si `legatorocr-gui` est à côté de PORUMBEL ou dans le `PATH` (sinon `launch =`).
2. Choisissez un moyen de transfert (les trois peuvent fonctionner ensemble) :
   * **Wi‑Fi** : `porumbel qr`, scannez le code, choisissez les photos dans le navigateur — ou une appli de synchronisation envoie le dossier de l'appareil photo à l'adresse WebDAV.
   * **ADB** : `porumbel --adb` récupère les nouvelles photos directement dans le dossier de l'appareil (débogage USB activé, Android 6+).
   * **Dossier synchronisé** : `porumbel --watch ~/Syncthing/OpenCamera` (Syncthing, etc.).
3. Photographiez les pages. Après 90 secondes de pause (`session_gap`), la session est close et LegatorOCR s'ouvre avec le dossier déjà chargé.

## Sécurité

Le serveur parle en HTTP simple sur le réseau local ; tout exige le jeton. Utilisez un Wi‑Fi de confiance, le mode ADB ou `bind = 127.0.0.1`, et n'exposez jamais le port sur Internet. Seules les images aux extensions acceptées sont conservées, sous un nom de fichier simple.

Réglages complets, limites et guides pas à pas : [README.md](README.md) et [docs/SETUP.md](docs/SETUP.md) (anglais), [docs/SETUP.ro.md](docs/SETUP.ro.md) (roumain).

## Licence

MIT — [LICENSE](LICENSE). © 2026 Alexio.
