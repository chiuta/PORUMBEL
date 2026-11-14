# PORUMBEL

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · **Español** · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

*Una paloma mensajera para sus escaneos.*

Lleva automáticamente las fotos hechas con **Open Camera** en el teléfono a una carpeta del ordenador, una subcarpeta por sesión de fotografía, y abre cada sesión terminada en **[LegatorOCR](https://github.com/chiuta/LegatorOCR)**. Un solo programa pequeño, **sin bibliotecas externas**, para Windows / macOS / Linux. Sin cuenta, sin nube.

## Inicio rápido

1. Ejecute `porumbel` una vez: crea el archivo de ajustes (se muestra la ruta) con un token aleatorio e inicia el servidor local. LegatorOCR se encuentra solo si `legatorocr-gui` está junto a PORUMBEL o en el `PATH` (si no, `launch =`).
2. Elija una forma de transferencia (las tres pueden funcionar a la vez):
   * **Wi‑Fi**: `porumbel qr`, escanee el código, elija las fotos en el navegador — o una app de sincronización envía la carpeta de la cámara a la dirección WebDAV.
   * **ADB**: `porumbel --adb` toma las fotos nuevas directamente de la carpeta de la cámara (depuración USB activada, Android 6+).
   * **Carpeta sincronizada**: `porumbel --watch ~/Syncthing/OpenCamera` (Syncthing, etc.).
3. Fotografíe las páginas. Tras 90 segundos de pausa (`session_gap`) la sesión se cierra y LegatorOCR se abre con la carpeta ya cargada.

## Seguridad

El servidor usa HTTP simple en la red local; todo exige el token. Use un Wi‑Fi de confianza, el modo ADB o `bind = 127.0.0.1`, y nunca exponga el puerto a internet. Solo se guardan imágenes con extensiones admitidas, con un nombre de archivo simple.

Ajustes completos, límites y guías paso a paso: [README.md](README.md) y [docs/SETUP.md](docs/SETUP.md) (inglés), [docs/SETUP.ro.md](docs/SETUP.ro.md) (rumano).

## Licencia

MIT — [LICENSE](LICENSE). © 2026 Alexio.
