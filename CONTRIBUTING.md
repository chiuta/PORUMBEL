# Contributing

* No third-party crates: keep the program std-only (trade-free, easy to audit, builds offline).
* `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass; the QR encoder has its own check (`tools/check-qr.py`).
* Commits as `Alexio <alexio@trom.tf>` for the maintainer; contributors use their own identity.
* Good first issues: more languages for the panel and docs (the panel strings are in `src/panel.html`),
  `LOCK` support for WebDAV clients that need it, an optional TLS front (reverse proxy recipe), a tray icon.
