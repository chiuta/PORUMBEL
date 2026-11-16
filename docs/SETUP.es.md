# Configuración, paso a paso

[English](SETUP.md) · [Română](SETUP.ro.md) · [Français](SETUP.fr.md) · [Italiano](SETUP.it.md) · **Español** · [Português](SETUP.pt.md) · [Deutsch](SETUP.de.md) · [Magyar](SETUP.hu.md)

## 0. En el teléfono: Open Camera

Open Camera guarda en `DCIM/OpenCamera` (ajuste *Ubicación de guardado*; el nombre cambia según la versión). Para escanear libros: JPEG a resolución completa, luz uniforme, sin flash. LegatorOCR lee JPEG, PNG, TIFF y BMP; WebP, HEIC y RAW/DNG se ignoran a propósito.

Si usa otra carpeta (p. ej. `DCIM/Libros`), indíquela: `phone_dir = /sdcard/DCIM/Libros` (solo el modo ADB lo necesita).

## 1. En el ordenador

```
porumbel            # → settings file + panel
porumbel qr         # QR
porumbel doctor     # check
```

En el primer arranque se crea el archivo de ajustes con un token aleatorio. Permita el programa en el cortafuegos para redes *privadas* (puerto 8750). ¿Dudas? `porumbel doctor` comprueba carpeta, puerto, dirección de red, LegatorOCR y (con `--adb`) el teléfono.

## 2. Elegir cómo mover las fotos

### ①a Wi‑Fi, desde el navegador del teléfono

Teléfono y ordenador en la misma Wi‑Fi (no una red de invitados que aísle los dispositivos). Escanee el código QR y toque *Elegir fotos*. Sirve para pocas páginas; para un libro entero use ② o ③.

### ①b Wi‑Fi, una app de sincronización envía la carpeta

Cualquier app que suba una carpeta a un servidor WebDAV: dirección `http://IP-DEL-ORDENADOR:8750/dav/` (el panel la muestra), usuario cualquiera, **contraseña = el token**, carpeta de Open Camera, sentido **solo subida**, sin borrados. Permita `http://` en la red local. *Aún no probado con una app real — cuéntenos qué observa.*

### ② ADB (cable o depuración Wi‑Fi) — lo más rápido

1. Instale las *platform-tools* de Android (`adb`) en el `PATH` (o `adb_path`).
2. Teléfono: Ajustes → Acerca del teléfono → toque siete veces *Número de compilación* → Opciones de desarrollador → **Depuración USB**.
3. Conecte el cable, acepte el aviso; `adb devices` debe mostrar `device`.
4. `porumbel --adb`. Las fotos que ya existen se dejan (`--adb-all` para importarlas); después cada foto nueva se copia en cuanto Open Camera termina de escribirla.
5. Opcional: `adb_delete = true` borra la foto del teléfono tras verificar la copia.

### ③ Una carpeta sincronizada (Syncthing y similares)

Teléfono: comparta la carpeta de Open Camera como *solo envío*. Ordenador: acéptela en una carpeta y luego `porumbel --watch ~/Syncthing/OpenCamera`. Los archivos aún en sincronización se ignoran; un archivo se toma cuando su tamaño y hora ya no cambian.

## 3. Fotografiar

Siga; las páginas llegan en orden. Tras `session_gap` segundos de pausa (90 por defecto) la carpeta está completa y se abre LegatorOCR.

## Arrancar con el ordenador

Archivos listos en `contrib/`: Windows `contrib\windows-autostart.ps1`, Linux `contrib/porumbel.service` (systemd, por usuario), macOS `contrib/ro.trom.porumbel.plist` (launchd).

## Problemas

| Síntoma | Qué comprobar |
|---|---|
| El teléfono no abre la dirección | ¿misma Wi‑Fi? ¿red de invitados? ¿cortafuegos? `porumbel qr` muestra la primera dirección hallada |
| `401` / pide contraseña | el token debe ser el del archivo de ajustes |
| ADB: *not authorised* | acepte el aviso en el teléfono; reconecte |
| ADB: *no such file* | `phone_dir` no coincide con la carpeta de Open Camera |
| LegatorOCR no se abre | `launch = /ruta/legatorocr-gui` |
| Falta una foto | solo se toman `jpg, jpeg, png, tif, tiff, bmp`; vea el registro del panel |
| Vuelven / no vuelven fotos | se recuerdan por nombre+tamaño: `porumbel forget` |
