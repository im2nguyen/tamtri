#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== tamtri checklist (daemon + app surfaces) =="

echo "[core] cargo test"
cargo test -p tamtri-core -- --test-threads=1

echo "[core] clippy"
cargo clippy -p tamtri-core -p tamtri-daemon --all-targets -- -D warnings

if command -v npm >/dev/null 2>&1; then
  echo "[app] typecheck"
  npm run typecheck --workspace @tamtri/app

  echo "[desktop] typecheck"
  npm run typecheck --workspace @tamtri/desktop
else
  echo "[npm] skipped (npm unavailable)"
fi

echo "Checklist passed."
echo "Manual visual pass: docs/visual-qa-checklist.md"
echo "Desktop dev: npm run app:web + npm run desktop:dev"
