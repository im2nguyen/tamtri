# Verification guides

Manual and automated verification for tamtri gateway features. One guide per feature, not per milestone.

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

1. Build core fixtures: `cargo build -p tamtri-core`
2. Register servers in `<vault>/config.json` (see each guide for examples).
3. Launch tamtri: `cd macos && swift run Tamtri`
4. Run automated checks: `cargo test -p tamtri-core`
