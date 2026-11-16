# PORUMBEL

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · **Português** · [Deutsch](README.de.md) · [Magyar](README.hu.md)

*Um pombo-correio para as suas digitalizações.*

Leva automaticamente as fotos tiradas com o **Open Camera** no telemóvel para uma pasta do computador, uma subpasta por sessão de fotografia, e abre cada sessão concluída no **[LegatorOCR](https://github.com/chiuta/LegatorOCR)**. Um único programa pequeno, **sem bibliotecas externas**, para Windows / macOS / Linux. Sem conta, sem nuvem.

## Início rápido

1. Execute `porumbel` uma vez: cria o ficheiro de definições (o caminho é mostrado) com um token aleatório e inicia o servidor local. O LegatorOCR é encontrado automaticamente se `legatorocr-gui` estiver junto do PORUMBEL ou no `PATH` (senão, `launch =`).
2. Escolha uma forma de transferência (as três podem funcionar em conjunto):
   * **Wi‑Fi**: `porumbel qr`, leia o código, escolha as fotos no navegador — ou uma app de sincronização envia a pasta da câmara para o endereço WebDAV.
   * **ADB**: `porumbel --adb` recolhe as fotos novas diretamente da pasta da câmara (depuração USB ativa, Android 6+).
   * **Pasta sincronizada**: `porumbel --watch ~/Syncthing/OpenCamera` (Syncthing, etc.).
3. Fotografe as páginas. Após 90 segundos de pausa (`session_gap`) a sessão fecha e o LegatorOCR abre com a pasta já carregada.

## Segurança

O servidor usa HTTP simples na rede local; tudo exige o token. Use um Wi‑Fi de confiança, o modo ADB ou `bind = 127.0.0.1`, e nunca exponha a porta à internet. Só se guardam imagens com extensões aceites, com um nome de ficheiro simples.

Definições completas, limites e guias passo a passo: [README.md](README.md) e [docs/SETUP.pt.md](docs/SETUP.pt.md) · [EN](docs/SETUP.md) · [RO](docs/SETUP.ro.md).

## Licença

MIT — [LICENSE](LICENSE). © 2026 Alexio.
