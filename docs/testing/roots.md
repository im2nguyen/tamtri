# Testing Roots

Verify per-conversation filesystem roots attach from the shell, persist legibly in `meta.json`, and reach downstream servers through the gateway.

## Prerequisites

- Built `m7-roots-mcp` fixture (`cargo build -p tamtri-core`).
- Vault gateway config with the roots fixture registered (`id`: `m7roots`).
- macOS shell (security-scoped bookmarks live in Application Support, not the vault).

## Build

```bash
cargo build -p tamtri-core
```

Binary: `target/debug/m7-roots-mcp`

## Config example

Add to `<vault>/config.json`:

```json
{
  "id": "m7roots",
  "display_name": "Roots fixture",
  "enabled": true,
  "scope": "user",
  "transport": {
    "type": "stdio",
    "command": "/absolute/path/to/tamtri/target/debug/m7-roots-mcp",
    "args": [],
    "env": []
  }
}
```

Gateway tool: `m7roots__probe_roots`

## Manual verification

- [ ] Register the server; probe capabilities; **Roots** badge is green.
- [ ] Open conversation header → **Roots**.
- [ ] **Add Folder** — pick a directory; bookmark saved under Application Support (not in vault).
- [ ] Confirm `meta.json` lists the root ref (`id`, `name`, `uri`, `kind`, `scope`).
- [ ] Call `m7roots__probe_roots` — structured content lists attached roots.
- [ ] Remove bookmark file manually and reopen Roots — missing bookmark warning with **Re-pick Folder**.

Bookmark path: `~/Library/Application Support/tamtri/root-bookmarks/<conversation_id>/<root_id>.bookmark`

## Automated tests

```bash
cargo test -p tamtri-core m7_roots
```

Coverage lives in `core/tests/m7_roots.rs`.

## Known limitations

- Bookmark bytes never enter the vault; only portable refs live in `meta.json`.
- External working directories use a separate consent/bookmark path (see CLAUDE.md WorkingDir).
- Roots are per-conversation; there is no global roots registry in V1.
