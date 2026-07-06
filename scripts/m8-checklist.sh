#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Milestone 8 checklist =="

echo "[core] cargo test (single-threaded)"
cargo test -p tamtri-core -- --test-threads=1

echo "[core] clippy"
cargo clippy -p tamtri-core --all-targets -- -D warnings

echo "[macos] swift build"
cd macos && swift build && cd ..

echo "[macos] swift test"
cd macos && swift test && cd ..

if command -v npm >/dev/null 2>&1; then
  echo "[renderer] build"
  bash scripts/build-renderer.sh
else
  echo "[renderer] skipped (npm unavailable)"
fi

echo "M8 checklist passed."
