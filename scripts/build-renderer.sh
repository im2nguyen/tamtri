#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/renderer"

if ! command -v npm >/dev/null 2>&1; then
  echo "npm is required to build the renderer island." >&2
  exit 1
fi

npm install
npm run build
echo "Renderer built to renderer/dist"
