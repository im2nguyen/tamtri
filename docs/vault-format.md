# Vault Format

tamtri stores conversations as a legible vault, not an opaque app database. The vault root contains a `conversations/` directory, and each conversation lives in one user-visible folder:

```text
<vault>/config.json
<vault>/conversations/<yyyy-mm-dd>-<slug>--<id-suffix>/
  meta.json
  messages.jsonl
  events.jsonl
  attachments/
  workdir/
```

`config.json` is vault-level app configuration. Milestone 4 uses it for the default harness id, hand-editable agent roster, and MCP gateway registry. It stores downstream server definitions, scopes, timeout overrides, and credential references only. It never stores resolved secret values. Writes are atomic via `config.json.tmp` in the same vault root. At run time, core reads this registry, starts a run-scoped Tamtri gateway endpoint, and exposes that single endpoint to the ACP harness instead of exposing downstream server definitions directly.

`meta.json` is the small mutable header. It contains `schema_version`, conversation identity, timestamps, harness/model ids, working directory mode, MCP server refs, roots, and fork lineage. Writes are atomic: tamtri writes `meta.json.tmp` in the same folder, then renames it over `meta.json`.

`messages.jsonl` is the transcript and the complete render source. It is append-only: one compact JSON `Message` per line. Streaming deltas are buffered in memory and committed only when the message is complete, so in-flight tokens never hit the log. `load` reads the full transcript into memory in V1; long sessions can therefore produce multi-megabyte in-memory transcripts. A streaming reader is a future implementation option, not a format change.

`events.jsonl` is the local audit log for permission receipts, command execution, and gateway routing. It is not portable by default, and secrets never persist to either log.

`attachments/` contains curated rendered artifacts. Anything the transcript renders is a content-hashed snapshot under `attachments/`, frozen at render time. `workdir/` stays messy, mutable, and local. Milestone 5 snapshots renderable `FileChanged` paths from the conversation workdir into `attachments/<sha256-prefix>-<safe-basename>` before appending an `Artifact` block, so reloading from `messages.jsonl` plus `attachments/` does not depend on the mutable workdir copy.

ACP agents are launched with the conversation `workdir/` as their process cwd, and the same path is sent in `session/new.cwd`. At turn end Tamtri snapshots only paths collected in the turn reducer's `referenced_paths` list (from `FileChanged`, tool-result diffs, and write/edit tool inputs). There is no full `workdir/` directory scan, so incidental files such as a dropped input CSV are not snapshotted unless the harness actually touched them.

Construct artifact blocks through `ContentBlock::artifact(path, mime_type, size, sha256, inline)`. `ContentBlock::Artifact.path` must be a vault-relative file under `attachments/`. Absolute paths and any `.` or `..` component are rejected. This blocks path traversal from imported bundles. Small UTF-8 text artifacts may inline in `messages.jsonl` up to `ARTIFACT_INLINE_MAX_BYTES` (`32 KiB`); larger text, binary artifacts, and inline content with non-text MIME types are stored as files and referenced by path, size, MIME type, and SHA-256. The message codec runs the same validator on deserialization, so `load` and import reject malformed artifact blocks with `MalformedVault`.

`workdir/` is reserved for the default `VaultLocal` working directory. Harness outputs can be messy here; rendered artifacts are snapshotted separately into `attachments/` before being referenced by the transcript.

Artifact previews use a conservative MIME policy: HTML, markdown, CSV/TSV, SVG, JSON, plain text, and common image formats are recognized by extension plus lightweight byte sniffing. Small UTF-8 text-like artifacts inline in the transcript up to the 32 KiB threshold; larger text and binary artifacts remain file-backed under `attachments/`.

Renderers must read file-backed artifacts through a verified attachment path. The verifier re-runs the artifact path rules, checks the file exists, and rejects size or SHA-256 mismatches before any active renderer such as HTML or SVG receives bytes.

## Repair Rules

Reads are lock-free and read-only. A torn final `messages.jsonl` line is ignored in memory because it was never an acknowledged commit. This includes a syntactically complete JSON object that lacks the final newline. Any malformed newline-terminated line is a hard `MalformedVault` error.

Writes take an exclusive advisory lock on that conversation's `messages.jsonl`, then repair any torn final line on disk before appending. There is no vault-wide lock, so different conversations can be written concurrently and external tools can browse the vault at any time.

Folder names are cosmetic. The id in `meta.json` is the truth, so Finder renames do not break load or list. The `<id-suffix>` is the conversation id in simple form: 32 lowercase hex characters with no hyphens. An earlier design truncated this to 8 hex chars for shorter Finder names; that was superseded because UUID v7 ids generated in rapid succession (multiple forks in one session) made 8-char suffix collisions plausible, especially on APFS case-insensitive volumes. The full simple form guarantees folder uniqueness without relying on slug and date alone. Duplicate ids from Finder copies or sync conflicts resolve deterministically to the newest `updated_at`, with path-name ordering as the tie breaker. tamtri never auto-deletes the losing folders; it reports them through `VaultIssue::DuplicateId`.

## Sync Stance

The vault is designed to sync through user-owned tools such as iCloud, Dropbox, or Git. `flock` protects single-machine writers only. Multi-machine conflicts belong to the sync tool, and tamtri tolerates their artifacts by surfacing duplicate ids and unreadable folders as vault issues.

## Migration Seam

`meta.json.schema_version` governs the format. Milestone 1 writes version `1`. A future reader may migrate lower versions before reconstructing a conversation. Readers reject future versions with `UnsupportedSchemaVersion`.
