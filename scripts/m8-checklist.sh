#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== tamtri checklist (daemon + app surfaces) =="

echo "[core] cargo test"
cargo test -p tamtri-core -- --test-threads=1

echo "[core] clippy"
cargo clippy -p tamtri-core -p tamtri-daemon --all-targets -- -D warnings

if command -v pnpm >/dev/null 2>&1; then
  echo "[app] typecheck"
  pnpm --filter @tamtri/app run typecheck

  echo "[desktop] typecheck"
  pnpm --filter @tamtri/desktop run typecheck
else
  echo "[pnpm] skipped (pnpm unavailable)"
fi

echo "Checklist passed."
echo "Manual visual pass: docs/visual-qa-checklist.md"
echo "Desktop dev: pnpm run app:web + pnpm run desktop:dev"
