# Contributing

* No third-party crates: keep the program std-only (trade-free, easy to audit, builds offline).
* `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass; the QR encoder has its own check (`tools/check-qr.py`).
* Commits as `Alexio <alexio@trom.tf>` for the maintainer; contributors use their own identity.
* Good first issues: more languages for the panel and docs (the panel strings are in `src/panel.html`),
  `LOCK` support for WebDAV clients that need it, an optional TLS front (reverse proxy recipe), a tray icon.

## Releasing (maintainer)

Binaries and the GitHub release come from `.github/workflows/release.yml`; nothing is built by hand.

* Either push a tag: `./release-alpha.sh` (or `git tag -a v0.1.0 -m … && git push origin v0.1.0`),
* or, without a local git: *Actions → Release → Run workflow*, type the tag (e.g. `v0.1.0-alpha.1`).
  The release step creates the tag on the selected branch.

A tag with a dash (`-alpha.1`, `-rc.1`) is published as a pre-release. Before tagging: CI on `main`
is green, `Cargo.toml` has the version, `cargo test` passes locally.
