#!/usr/bin/env bash
# Run from Git Bash inside the PORUMBEL folder: tags the alpha; the Release workflow builds
# Windows/macOS/Linux binaries and publishes the pre-release.
set -euo pipefail
git fetch origin main -q
git checkout main -q && git pull -q --ff-only origin main
git tag -a v0.1.0-alpha.1 -m "PORUMBEL 0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
echo "Watch: https://github.com/chiuta/PORUMBEL/actions"
