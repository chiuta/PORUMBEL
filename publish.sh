#!/usr/bin/env bash
# Publish PORUMBEL to github.com/chiuta/PORUMBEL (run from Git Bash, in this folder).
# Works whether or not you already created the empty repository on GitHub.
set -euo pipefail
cd "$(dirname "$0")"
git config user.name  "Alexio"
git config user.email "alexio@trom.tf"
if ! git remote get-url origin >/dev/null 2>&1; then
  if gh repo view chiuta/PORUMBEL >/dev/null 2>&1; then
    git remote add origin https://github.com/chiuta/PORUMBEL.git
    git push -u origin main
  else
    gh repo create chiuta/PORUMBEL --public --source . --remote origin --push \
      --description "Open Camera → folder → LegatorOCR: automatic photo transfer (Rust, no dependencies)"
  fi
else
  git push -u origin main
fi
# optional: first draft pre-release (the Release workflow builds Windows/macOS/Linux binaries)
# git tag v0.1.0-alpha.1 && git push origin v0.1.0-alpha.1
echo "Done. Enable GitHub Pages (docs/ folder) for porumbel.chiuta.ro."
