#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/scripts/sync-design-tokens.sh"
cd "$ROOT/renderer"

if ! command -v npm >/dev/null 2>&1; then
  echo "npm is required to build the renderer island." >&2
  exit 1
fi

npm install
npm run build
python3 - <<'PY'
from pathlib import Path
html = Path("dist/index.html")
text = html.read_text()
text = text.replace(' type="module" crossorigin', '')
text = text.replace(' type="module"', '')
html.write_text(text)
PY
echo "Renderer built to renderer/dist"
