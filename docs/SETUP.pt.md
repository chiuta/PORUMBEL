# Configuração, passo a passo

[English](SETUP.md) · [Română](SETUP.ro.md) · [Français](SETUP.fr.md) · [Italiano](SETUP.it.md) · [Español](SETUP.es.md) · **Português** · [Deutsch](SETUP.de.md) · [Magyar](SETUP.hu.md)

## 0. No telefone: Open Camera

O Open Camera guarda em `DCIM/OpenCamera` (definição *Local de gravação*; o nome varia com a versão). Para digitalizar livros: JPEG em resolução máxima, luz uniforme, sem flash. O LegatorOCR lê JPEG, PNG, TIFF e BMP; WebP, HEIC e RAW/DNG são ignorados de propósito.

Se usar outra pasta (ex. `DCIM/Livros`), indique-a: `phone_dir = /sdcard/DCIM/Livros` (só o modo ADB precisa).

## 1. No computador

```
porumbel            # → settings file + panel
porumbel qr         # QR
porumbel doctor     # check
```

No primeiro arranque é criado o ficheiro de definições com um token aleatório. Permita o programa na firewall para redes *privadas* (porta 8750). Dúvidas? `porumbel doctor` verifica pasta, porta, endereço de rede, LegatorOCR e (com `--adb`) o telefone.

## 2. Escolher como mover as fotos

### ①a Wi‑Fi, a partir do navegador do telefone

Telefone e computador na mesma Wi‑Fi (não numa rede de convidados que isole dispositivos). Leia o código QR e toque em *Escolher fotos*. Serve para poucas páginas; para um livro inteiro use ② ou ③.

### ①b Wi‑Fi, uma app de sincronização envia a pasta

Qualquer app que envie uma pasta para um servidor WebDAV: endereço `http://IP-DO-COMPUTADOR:8750/dav/` (o painel mostra-o), utilizador qualquer, **palavra-passe = o token**, pasta do Open Camera, sentido **só envio**, sem apagar. Permita `http://` na rede local. *Ainda não testado com uma app real — diga-nos o que observa.*

### ② ADB (cabo ou depuração Wi‑Fi) — o mais rápido

1. Instale as *platform-tools* do Android (`adb`) no `PATH` (ou `adb_path`).
2. Telefone: Definições → Acerca do telefone → toque sete vezes em *Número da compilação* → Opções de programador → **Depuração USB**.
3. Ligue o cabo, aceite o aviso; `adb devices` deve mostrar `device`.
4. `porumbel --adb`. As fotos que já existem ficam de fora (`--adb-all` para as importar); depois cada foto nova é copiada logo que o Open Camera acabe de a escrever.
5. Opcional: `adb_delete = true` apaga a foto do telefone depois de verificada a cópia.

### ③ Uma pasta sincronizada (Syncthing e semelhantes)

Telefone: partilhe a pasta do Open Camera como *só envio*. Computador: aceite-a numa pasta e depois `porumbel --watch ~/Syncthing/OpenCamera`. Os ficheiros ainda em sincronização são ignorados; um ficheiro é tomado quando tamanho e hora deixam de mudar.

## 3. Fotografar

Continue; as páginas chegam por ordem. Após `session_gap` segundos de pausa (90 por omissão) a pasta está completa e o LegatorOCR abre.

## Arrancar com o computador

Ficheiros prontos em `contrib/`: Windows `contrib\windows-autostart.ps1`, Linux `contrib/porumbel.service` (systemd, por utilizador), macOS `contrib/ro.trom.porumbel.plist` (launchd).

## Problemas

| Sintoma | O que verificar |
|---|---|
| O telefone não abre o endereço | mesma Wi‑Fi? rede de convidados? firewall? `porumbel qr` mostra o primeiro endereço encontrado |
| `401` / pede palavra-passe | o token tem de ser o do ficheiro de definições |
| ADB: *not authorised* | aceite o aviso no telefone; volte a ligar |
| ADB: *no such file* | `phone_dir` não coincide com a pasta do Open Camera |
| O LegatorOCR não abre | `launch = /caminho/legatorocr-gui` |
| Falta uma foto | só se tomam `jpg, jpeg, png, tif, tiff, bmp`; veja o registo no painel |
| Fotos voltam / não voltam | lembradas por nome+tamanho: `porumbel forget` |
