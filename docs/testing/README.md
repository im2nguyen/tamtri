# Verification guides

Manual and automated verification for tamtri gateway and harness features.

| Guide | What it covers |
|-------|----------------|
| [elicitation.md](elicitation.md) | Form-mode and URL-mode elicitation |
| [twenty-questions.md](twenty-questions.md) | Hero elicitation demo (20 Questions fixture) |
| [oauth.md](oauth.md) | Remote OAuth 2.1 + PKCE connect and reset |
| [apps.md](apps.md) | MCP Apps (webview, bridge, consent) |
| [tasks.md](tasks.md) | Long-running task cards |
| [roots.md](roots.md) | Per-conversation filesystem roots |
| [mcp-capabilities.md](mcp-capabilities.md) | Capability gates, RC extensions, sampling declined |

## Quick start

1. Build core and fixtures: `cargo build -p tamtri-core`
2. Register servers in `<vault>/config.json` (see each guide for examples).
3. Launch tamtri: `pnpm run dev:web` or `pnpm run dev:desktop`
4. Run automated checks: `cargo test -p tamtri-core`

Fixtures live in [`/fixtures`](../fixtures/README.md) (test binaries only).
