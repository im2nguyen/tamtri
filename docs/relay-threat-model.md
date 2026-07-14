# Relay threat model

tamtri's remote access path lets web/mobile clients reach a home daemon through
an **E2E-encrypted relay**. The relay operator sees only ciphertext and routing
metadata; it cannot read conversation content, vault paths, or credentials.

## Assets

| Asset | Owner | Must not leak to relay |
|-------|-------|------------------------|
| Vault transcript + attachments | User device (daemon) | Yes |
| Gateway credentials / OAuth tokens | Daemon (`credentials.sealed`) | Yes |
| Localhost bearer token | Daemon (`daemon.token`) | Yes — never sent to relay; pairing uses a separate key exchange |
| Daemon long-term relay keypair | Daemon (`daemon-keypair.json`) | Public key only in pairing offer |

## Cryptography

- **Key exchange:** Curve25519 (`crypto_box` / NaCl box)
- **Encryption:** XSalsa20-Poly1305
- **Pairing offer:** `{ v, server_id, daemon_public_key_b64, relay: { endpoint, use_tls } }` encoded in a URL fragment or QR

Client and daemon derive a shared key from their keypairs; the relay forwards
base64 ciphertext frames without access to plaintext.

## Trust assumptions

1. **Relay operator** is honest-but-curious: may drop, delay, or rate-limit traffic; must not break E2EE.
2. **Client** verifies the pairing offer out-of-band (QR scan, trusted link) before trusting `daemon_public_key_b64`.
3. **Daemon** binds to localhost for direct access; relay attachment is an optional outbound connection initiated by the daemon.

## Out of scope (V1)

- Multi-device concurrent write (single-writer per conversation via vault `flock` still applies).
- Relay-side authentication beyond routing by `server_id`.
- Hosting the relay infrastructure itself (endpoint configurable via `TAMTRI_RELAY_ENDPOINT`).

## Recovery

If the relay keypair is rotated, existing paired clients must re-scan a fresh
pairing offer. The vault and credentials are unaffected.
