# Installation, pas à pas

[English](SETUP.md) · [Română](SETUP.ro.md) · **Français** · [Italiano](SETUP.it.md) · [Español](SETUP.es.md) · [Português](SETUP.pt.md) · [Deutsch](SETUP.de.md) · [Magyar](SETUP.hu.md)

## 0. Sur le téléphone : Open Camera

Open Camera enregistre dans `DCIM/OpenCamera` (réglage *Emplacement de sauvegarde*, le libellé varie selon la version). Pour numériser un livre : JPEG pleine résolution, lumière régulière, sans flash. LegatorOCR lit JPEG, PNG, TIFF et BMP ; WebP, HEIC et RAW/DNG sont volontairement ignorés.

Si vous utilisez un autre dossier (ex. `DCIM/Livres`), indiquez-le : `phone_dir = /sdcard/DCIM/Livres` (seul le mode ADB en a besoin).

## 1. Sur l'ordinateur

```
porumbel            # → settings file + panel
porumbel qr         # QR
porumbel doctor     # check
```

Au premier lancement, le fichier de réglages est créé avec un jeton aléatoire. Autorisez le programme dans le pare-feu pour les réseaux *privés* (port 8750). Un doute ? `porumbel doctor` vérifie le dossier, le port, l'adresse réseau, LegatorOCR et (avec `--adb`) le téléphone.

## 2. Choisir le moyen de transfert

### ①a Wi‑Fi, depuis le navigateur du téléphone

Téléphone et ordinateur sur le même Wi‑Fi (pas un réseau invité qui isole les appareils). Scannez le QR code, touchez *Choisir des photos*. Convient à quelques pages ; pour un livre entier, utilisez ② ou ③.

### ①b Wi‑Fi, une appli de synchronisation envoie le dossier

Toute appli capable d'envoyer un dossier vers un serveur WebDAV : adresse `http://IP-DE-L-ORDINATEUR:8750/dav/` (le panneau l'affiche), utilisateur au choix, **mot de passe = le jeton**, dossier d'Open Camera, sens **envoi seulement**, sans suppression. Autorisez `http://` sur le réseau local. *Pas encore testé avec une vraie appli — dites-nous ce que vous constatez.*

### ② ADB (câble ou débogage sans fil) — le plus rapide

1. Installez les *platform-tools* Android (`adb`) dans le `PATH` (ou `adb_path`).
2. Téléphone : Paramètres → À propos → touchez sept fois *Numéro de build* → Options pour les développeurs → **Débogage USB**.
3. Branchez le câble, acceptez la demande, `adb devices` doit afficher `device`.
4. `porumbel --adb`. Les photos déjà présentes sont laissées de côté (`--adb-all` pour les importer) ; ensuite chaque nouvelle photo est copiée dès qu'Open Camera a fini de l'écrire.
5. Facultatif : `adb_delete = true` supprime la photo du téléphone après vérification de la copie.

### ③ Un dossier synchronisé (Syncthing, etc.)

Téléphone : partagez le dossier d'Open Camera en *envoi seulement*. Ordinateur : acceptez-le dans un dossier, puis `porumbel --watch ~/Syncthing/OpenCamera`. Les fichiers en cours de synchronisation sont ignorés ; un fichier est pris quand sa taille et son heure ne bougent plus.

## 3. Photographier

Continuez ; les pages arrivent dans l'ordre. Après `session_gap` secondes de pause (90 par défaut), le dossier est complet et LegatorOCR s'ouvre avec lui.

## Démarrage avec l'ordinateur

Fichiers prêts à l'emploi dans `contrib/` : Windows `contrib\windows-autostart.ps1`, Linux `contrib/porumbel.service` (systemd, par utilisateur), macOS `contrib/ro.trom.porumbel.plist` (launchd).

## En cas de problème

| Symptôme | À vérifier |
|---|---|
| Le téléphone n'ouvre pas l'adresse | même Wi‑Fi ? réseau invité ? pare-feu ? `porumbel qr` donne la première adresse trouvée |
| `401` / mot de passe demandé | le jeton doit être celui du fichier de réglages |
| ADB : *not authorised* | acceptez la demande sur le téléphone ; rebranchez |
| ADB : *no such file* | `phone_dir` ne correspond pas au dossier d'Open Camera |
| LegatorOCR ne s'ouvre pas | `launch = /chemin/legatorocr-gui` |
| Photo manquante | seuls `jpg, jpeg, png, tif, tiff, bmp` sont pris ; voir le journal du panneau |
| Des photos reviennent / ne reviennent pas | mémorisées par nom+taille : `porumbel forget` |
