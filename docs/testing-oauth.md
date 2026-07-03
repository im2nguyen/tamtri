# Testing OAuth (Milestone 6)

Manual E2E notes for remote streamable HTTP gateway servers with OAuth 2.1 + PKCE.

## Why settings show "connected"

`oauth_connection_status` (`core/src/mcp/oauth.rs`) is purely local:

1. Server has an `oauth` block in `config.json`.
2. A non-empty access token exists in the in-memory credential store for `oauth.token_ref`.
3. `reauth_required` is false and `expires_at` (if set) is still in the future.

On launch, the macOS shell loads any keychain entry for each `token_ref` into core (`AppStore.reloadGatewayServers`). Completing Connect runs PKCE exchange, writes the bundle to keychain (`OAuthTokenStore`), and core memory. Tokens survive app restarts.

Vault `config.json` stores references only (`token_ref`, endpoints, `client_id`). Token values never land in the vault.

## Reset connected state

There is no Disconnect button yet. To force `missing` again:

1. **Delete the keychain entry** for the server's `token_ref` (service `tamtri.gateway`, account = the ref, e.g. `keychain://my-remote`):
   ```bash
   security delete-generic-password -s "tamtri.gateway" -a "keychain://my-remote"
   ```
   Or use Keychain Access and search for `tamtri.gateway`.

2. **Quit and relaunch tamtri.** Core keeps the old token in memory until restart; deleting keychain alone while the app is running does not clear status.

3. Optionally remove or disable the server in gateway settings (does not delete the keychain entry by itself).

## Real servers to try

tamtri does **not** implement RFC 9728/8414 discovery or dynamic client registration yet. You must set `authorization_endpoint`, `token_endpoint`, and `client_id` manually. One-time DCR via `curl` is the easiest way to get a `client_id` for DCR-native hosts.

| Server | MCP URL | Auth model | tamtri setup |
|--------|---------|------------|--------------|
| **Linear** (recommended) | `https://mcp.linear.app/mcp` | OAuth 2.1 + DCR | `curl` register → paste `client_id` + endpoints below |
| **Sentry** | `https://mcp.sentry.dev/mcp` | OAuth 2.1 + DCR | Same pattern; free Sentry account |
| **GitHub** | `https://api.githubcopilot.com/mcp` | OAuth via GitHub AS; **no DCR** | Register a [GitHub OAuth App](https://github.com/settings/developers); callback `http://127.0.0.1:3847/callback` |

### Linear example

```bash
curl -sS -X POST "https://mcp.linear.app/register" \
  -H "Content-Type: application/json" \
  -d '{
    "client_name": "tamtri-dev",
    "redirect_uris": ["http://127.0.0.1:3847/callback"],
    "grant_types": ["authorization_code"],
    "response_types": ["code"],
    "token_endpoint_auth_method": "none"
  }'
```

`config.json` fragment (use `client_id` from the response):

```json
{
  "id": "linear",
  "display_name": "Linear",
  "enabled": true,
  "scope": "user",
  "transport": {
    "type": "streamable_http",
    "endpoint": "https://mcp.linear.app/mcp"
  },
  "oauth": {
    "authorization_endpoint": "https://mcp.linear.app/authorize",
    "token_endpoint": "https://mcp.linear.app/token",
    "client_id": "<from-register-response>",
    "scopes": ["read", "write"],
    "token_ref": "keychain://linear-oauth"
  }
}
```

### Sentry example

Same DCR call against `https://mcp.sentry.dev/oauth/register`. Endpoints from `/.well-known/oauth-authorization-server`: authorize `https://mcp.sentry.dev/oauth/authorize`, token `https://mcp.sentry.dev/oauth/token`. Scopes: `org:read`, `project:write`, etc.

### GitHub example

Create an OAuth App; use `client_id` from the app settings. Endpoints: `https://github.com/login/oauth/authorize` and `https://github.com/login/oauth/access_token`. Scopes depend on toolsets (see GitHub MCP protected-resource metadata). No `client_secret` in tamtri token exchange (public client + PKCE only).

## Known tamtri gaps

- No automatic discovery from `/.well-known/oauth-protected-resource` or `WWW-Authenticate` on 401.
- No in-app DCR; one-time `curl` register (or a pre-registered GitHub OAuth App) is required.
- No `resource=` parameter on authorize requests (RFC 8707); strict servers may reject flows later.
- `OAuthConfig.issuer` is stored but unused today.
- No disconnect UI; reset via keychain + restart.

## Local mock (dev only)

The vault may still point at a loopback mock (`http://127.0.0.1:9876/mcp` with matching `/authorize` and `/token`). Any completed Connect against that mock leaves a real keychain bundle and shows `connected` until reset. See reset steps above.

Automated mock-token tests: `cargo test -p tamtri-core oauth`.
