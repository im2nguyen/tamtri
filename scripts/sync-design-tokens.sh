#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOKENS="$ROOT/macos/Sources/Tamtri/Design/design-tokens.json"
OUT="$ROOT/renderer/src/tokens.css"

python3 - "$TOKENS" "$OUT" <<'PY'
import json
import sys

tokens_path, out_path = sys.argv[1], sys.argv[2]
with open(tokens_path, encoding="utf-8") as f:
    t = json.load(f)

s = t["spacing"]
r = t["radius"]
layout = t["layout"]
light = t["colors"]["light"]
dark = t["colors"]["dark"]

css = f""":root {{
  --tamtri-space-xs: {s["xs"]}px;
  --tamtri-space-sm: {s["sm"]}px;
  --tamtri-space-md: {s["md"]}px;
  --tamtri-space-lg: {s["lg"]}px;
  --tamtri-space-xl: {s["xl"]}px;

  --tamtri-radius-card: {r["card"]}px;
  --tamtri-radius-bar: {r["bar"]}px;

  --tamtri-text-primary: {light["textPrimary"]};
  --tamtri-text-secondary: {light["textSecondary"]};
  --tamtri-text-muted: {light["textMuted"]};
  --tamtri-surface-card: {light["surfaceCard"]};
  --tamtri-surface-user: {light["surfaceUser"]};
  --tamtri-surface-thinking: {light["surfaceThinking"]};
  --tamtri-surface-tool: {light["surfaceTool"]};
  --tamtri-surface-composer: {light["surfaceComposer"]};
  --tamtri-border: {light["border"]};
  --tamtri-focus: {light["focus"]};
  --tamtri-max-line: {layout["contentMaxCh"]}ch;
}}

@media (prefers-color-scheme: dark) {{
  :root {{
    --tamtri-text-primary: {dark["textPrimary"]};
    --tamtri-text-secondary: {dark["textSecondary"]};
    --tamtri-text-muted: {dark["textMuted"]};
    --tamtri-surface-card: {dark["surfaceCard"]};
    --tamtri-surface-user: {dark["surfaceUser"]};
    --tamtri-surface-thinking: {dark["surfaceThinking"]};
    --tamtri-surface-tool: {dark["surfaceTool"]};
    --tamtri-surface-composer: {dark["surfaceComposer"]};
    --tamtri-border: {dark["border"]};
  }}
}}
"""

with open(out_path, "w", encoding="utf-8") as f:
    f.write(css)

print(f"Wrote {out_path}")
PY
