# Milestone 1: Core Skeleton (vault + conversation model)

**Status: Complete.** Legible vault, conversation model, fork/import, and enumerated tests.

First Claude Code session. Build the Rust core (`tamtri-core`): the canonical conversation model, the **legible vault** (folder per conversation, `meta.json` + append-only `messages.jsonl` + `attachments/`), and fork/import. No UI. No MCP. No harnesses. This milestone locks the format everything else depends on, and it is the format the future team/enterprise sync layer will reuse, so get it clean.

## Definition of done

- `cargo build` succeeds, `cargo test` all green, `cargo clippy` no warnings.
- No `unwrap()` / `expect()` in non-test code.
- A conversation persists as a legible vault folder: `meta.json`, append-only `messages.jsonl`, `attachments/`.
- `meta.json` writes are atomic (write temp, rename). A crash never leaves a half-written meta.
- Reads are lock-free and tolerate a torn final `messages.jsonl` line in memory; the write path repairs it on disk. Corruption anywhere else is a hard error.
- Folder names are cosmetic; the id in `meta.json` is the truth. A renamed folder still loads. Duplicate ids (Finder-duplicated or sync-conflicted folders) resolve deterministically with a warning, and are reported as `VaultIssue`s for later UI surfacing.
- Concurrent access is safe: any number of readers at any time, one writer per conversation (exclusive `flock` on that conversation's `messages.jsonl`). A contended write returns `CoreError::ConversationBusy` after a short bounded retry. There is no vault-wide lock; browsing while another instance runs is a supported feature.
- The vault format, repair rules, and sync-conflict stance are documented in `/docs/vault-format.md`.
- Round-trip, append-only, repair, fork, import, and list/delete behaviors are covered by the tests below.

## Prerequisites

- Rust stable via rustup, edition 2024, MSRV 1.89+ (the vault lock uses `std::fs::File::try_lock`, stabilized in 1.89; no locking dep needed). No network, no cloud, no accounts, no telemetry.

## Task 1: Workspace scaffold

Monorepo cargo workspace. Only the core crate this milestone. The macOS shell comes later.

```
/                       repo root
  Cargo.toml            [workspace] members = ["core"]
  /core
    Cargo.toml          package = "tamtri-core", edition = "2024"
    /src
      lib.rs
      error.rs
      conversation/
        mod.rs
        model.rs         # domain types (the contract)
        meta.rs          # ConversationMeta + meta.json (de)serialization
        ops.rs           # new / fork / import_as_new / touch
      vault/
        mod.rs           # ConversationVault trait
        fs.rs            # filesystem vault (folder per conversation)
        naming.rs        # folder-name slug + short id
  /docs
    vault-format.md
```

Dependencies (core `Cargo.toml`):

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }   # or jiff, if preferred
thiserror = "2"

[dev-dependencies]
tempfile = "3"
pretty_assertions = "1"
```

Notes:
- UUID v7 for all ids (time-sortable). `chrono::DateTime<Utc>`, serialized RFC 3339.
- No SQLite this milestone. `list()` scans `meta.json` files (they are tiny). A SQLite index is an optional later cache, rebuildable from the vault, never a source of truth.
- Hand-roll the slug (no slug crate) to keep deps minimal.

## Task 2: Domain model (`conversation/model.rs`)

The contract. Derive `Debug, Clone, PartialEq, Serialize, Deserialize` on all. Skip `None` optionals on serialize (`#[serde(skip_serializing_if = "Option::is_none")]`).

```rust
pub type Id = uuid::Uuid;

pub struct Conversation {
    // --- meta fields (persist to meta.json) ---
    pub id: Id,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active_harness_id: Option<String>,
    pub model_id: Option<String>,
    pub working_dir: WorkingDir,
    pub mcp_servers: Vec<McpServerRef>,
    pub roots: Vec<Root>,
    pub forked_from: Option<Id>,
    // --- messages (persist to messages.jsonl, one per line) ---
    pub messages: Vec<Message>,
}

pub struct Message {
    pub id: Id,
    pub role: Role,
    pub harness_id: Option<String>,   // None for user/system; Some for assistant/tool
    pub content: Vec<ContentBlock>,
    pub created_at: DateTime<Utc>,
}

#[serde(rename_all = "snake_case")]
pub enum Role { User, Assistant, Tool, System }

#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Thinking { text: String },   // reasoning; renders collapsibly, persists so it re-renders after the session
    ToolCall { id: String, name: String, input: serde_json::Value },
    ToolResult { call_id: String, output: serde_json::Value },   // carries rendered content (tool output, diff hunks) so the card redraws
    AppResource { uri: String, template_ref: String, state: serde_json::Value },
    Artifact {
        path: String,           // vault-relative, under attachments/
        mime_type: String,
        size: u64,
        sha256: String,
        inline: Option<String>, // small text artifacts may inline; binary/large go to attachments/
    },
    ElicitationRequest {
        request_id: String,
        mode: ElicitationMode,
        message: String,
        schema: Option<serde_json::Value>,  // form mode
        url: Option<String>,                // url mode
    },
    ElicitationResponse {
        request_id: String,
        action: ElicitationAction,
        data: Option<serde_json::Value>,
    },
    TaskRef { task_id: String, status: TaskStatus },
}

#[serde(rename_all = "snake_case")]
pub enum ElicitationMode { Form, Url }

#[serde(rename_all = "snake_case")]
pub enum ElicitationAction { Accept, Decline, Cancel }

#[serde(rename_all = "snake_case")]
pub enum TaskStatus { Pending, Running, Completed, Failed }

// Repo-optional working directory. VaultLocal = a workdir/ inside the conversation's own vault folder.
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum WorkingDir {
    VaultLocal,
    External { path: String },
}
impl Default for WorkingDir { fn default() -> Self { WorkingDir::VaultLocal } }

pub struct McpServerRef {
    pub id: String,
    pub name: String,
    pub transport: String,   // "stdio" | "http"
    pub endpoint: String,
}

pub struct Root {
    pub uri: String,
    pub name: Option<String>,
}
```

`harness_id` on `Message` powers transcript provenance later. `model_id` and `working_dir` are unused until later milestones but live in the format now so the schema does not bump. The `Artifact` block references a file under `attachments/`; the artifact bytes themselves are written by later milestones, but the block type is locked now.

Inline threshold: `pub const ARTIFACT_INLINE_MAX_BYTES: usize = 32 * 1024;`. Construct artifacts through `ContentBlock::artifact(path, mime_type, size, sha256, inline) -> Result<ContentBlock>`. `Artifact.inline` is permitted only for UTF-8 text at or under this size. Anything larger, and all binary content, goes to `attachments/` and is referenced by path. Artifact paths must be vault-relative files under `attachments/`; absolute paths and any `.` or `..` component are rejected. Enforce the same validator from the message codec so `load` and `import_folder_as_new` reject malformed imported bundles with `CoreError::MalformedVault`. This validator is the single enforcement point; render paths add no checks. This bounds transcript and share-bundle bloat and blocks path traversal; document the constant in `/docs/vault-format.md`.

Known ceiling, accepted for V1: `load` reads the entire transcript into memory as `Vec<Message>`, and `ToolResult.output` carries rendered content, so long agent sessions produce multi-megabyte transcripts held whole. Fine for now; note it in the docs so no one builds UI that assumes transcripts are always cheap to hold, and so a streaming reader is a known future option, not a format change.

## Task 3: Vault persistence format (`conversation/meta.rs`)

Storage is a legible vault. One folder per conversation. Conversation-level metadata lives in a small, freely-rewritten `meta.json`; messages live in an append-only `messages.jsonl`.

```
<vault>/conversations/<yyyy-mm-dd>-<slug>--<id-suffix>/
  meta.json        mutable, tiny
  messages.jsonl   append-only, exactly one Message object per line
  events.jsonl     RESERVED this milestone (created empty, not written). The local audit log; written from milestone 3.
  attachments/     curated rendered artifacts, hashed (later milestones)
  workdir/         RESERVED this milestone (created empty). Harness working directory for VaultLocal; populated from milestone 3.
```

**The transcript is the complete render source.** `messages.jsonl` must, on its own, redraw the conversation exactly as it looked live after the session closes: thinking, text, tool calls with their results and diffs, artifacts, elicitation exchanges. The reduction from the live event stream into `ContentBlock`s is lossless with respect to anything rendered; only sub-message token deltas (they collapse into the final block) and pure protocol chatter are excluded. `ToolResult.output` therefore carries the rendered content (tool output, diff hunks) so a tool card redraws without the live stream.

**Transcript vs audit log.** `messages.jsonl` is the portable, shareable, fork-seed record of the conversation as seen. `events.jsonl` (reserved here, written from milestone 3) is the local receipts: permission resolutions, full tool args, which downstream MCP servers the gateway hit, command executions. It is not portable by default and never leaves in a share bundle unless the user opts in. Secrets never persist to either: the gateway records "injected credential for server X," never the value.

```rust
pub const SCHEMA_VERSION: u32 = 1;

// Exactly the Conversation meta fields, plus the schema version. No messages.
pub struct ConversationMeta {
    pub schema_version: u32,
    pub id: Id,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active_harness_id: Option<String>,
    pub model_id: Option<String>,
    pub working_dir: WorkingDir,
    pub mcp_servers: Vec<McpServerRef>,
    pub roots: Vec<Root>,
    pub forked_from: Option<Id>,
}
```

Functions:
- `ConversationMeta::from_conversation(&Conversation) -> ConversationMeta` (stamps `SCHEMA_VERSION`).
- `ConversationMeta::to_json_pretty(&self) -> Result<String>` and `from_json(&str) -> Result<ConversationMeta>`. On read, if `schema_version > SCHEMA_VERSION` return `CoreError::UnsupportedSchemaVersion`; if lower, run migrations (none yet; leave a documented seam).
- Message line codec: `message_to_line(&Message) -> Result<String>` (compact JSON, no newlines inside) and `message_from_line(&str) -> Result<Message>`.
- Reconstruct: `Conversation::from_parts(meta: ConversationMeta, messages: Vec<Message>) -> Conversation`.

Document the layout and the migration seam in `/docs/vault-format.md`.

## Task 4: Operations (`conversation/ops.rs`)

- `Conversation::new(title) -> Conversation` — fresh v7 id, timestamps now, `WorkingDir::VaultLocal`, empty collections, `forked_from = None`.
- `Conversation::touch(&mut self)` — set `updated_at = now`. Call on any mutation.
- `Conversation::fork(&self) -> Conversation` — deep clone, new id, `forked_from = Some(self.id)`, reset timestamps to now, keep messages and meta. Independent: mutating the fork never touches the original.
- `Conversation::push_message(&mut self, Message)` — append + `touch`.

## Task 5: The vault (`vault/`)

The vault is the source of truth. Folders are user-visible and legible.

Folder naming (`vault/naming.rs`):
- `folder_name(c) = "<yyyy-mm-dd>-<slug>--<id-suffix>"`, where `slug` is the title lowercased, ASCII-folded, non-alphanumerics collapsed to single hyphens, trimmed, truncated to ~40 chars; `id_suffix` is the id in simple form (32 lowercase hex chars, no hyphens). An earlier 8-char suffix was superseded: UUID v7 ids from rapid forks can collide in 8 hex chars, so the full simple form guarantees folder uniqueness and sidesteps title collisions, unicode, and APFS case-insensitivity.

Trait (`vault/mod.rs`):

```rust
pub struct ConversationSummary {
    pub id: Id,
    pub title: String,
    pub updated_at: DateTime<Utc>,
}

pub trait ConversationVault {
    fn create(&self, c: &Conversation) -> Result<()>;          // new folder + meta.json + messages.jsonl + attachments/
    fn save_meta(&self, c: &Conversation) -> Result<()>;       // rewrite meta.json only (tiny)
    fn append_message(&self, id: Id, m: &Message) -> Result<()>; // append ONE line to messages.jsonl; bump updated_at
    fn load(&self, id: Id) -> Result<Conversation>;            // read meta.json + all messages.jsonl lines
    fn list(&self) -> Result<Vec<ConversationSummary>>;        // scan meta.json files, newest first
    fn delete(&self, id: Id) -> Result<()>;                    // remove the folder
    fn import_folder_as_new(&self, src: &Path) -> Result<Conversation>; // read a foreign vault folder, new id, forked_from = None, write it in
    fn issues(&self) -> Result<Vec<VaultIssue>>;                // re-scan and report anomalies; cheap, side-effect free
}
```

`FilesystemVault` (`vault/fs.rs`):
- Constructed with a root dir. Conversations under `<root>/conversations/`. Create the root if missing. **No vault-wide lock**: browsing a vault while another instance runs is a supported feature, and external tools may read the vault freely at any time.
- **Concurrency model: lock-free reads, per-conversation write locks.** `load` and `list` never take locks. Every write operation (`create`, `append_message`, `save_meta`, `delete`, `import_folder_as_new`) takes an exclusive advisory lock on that conversation's `messages.jsonl` (`std::fs::File::try_lock`, Rust 1.89+) for the duration of the operation. The transcript file is the lock object because it always exists and is never renamed; `meta.json` cannot be (temp+rename swaps its inode), and a separate `.lock` file would pollute the legible folder and leak into bundles and sync. `flock` releases automatically on process death, so there are no stale locks. On contention, retry `try_lock` for up to ~2 seconds, then return `CoreError::ConversationBusy(id)`; writes are millisecond-scale, so real contention means a stuck peer and hanging would be worse. Writers on the same conversation serialize; different conversations write in parallel; interleaved appends from two processes are safe (one line per append under lock) and meta is last-writer-wins.
- **Folder names are cosmetic. The id inside `meta.json` is the truth.** A user renaming a folder in Finder must break nothing: `load` and `list` resolve by scanning `meta.json` contents, never by parsing folder names.
- `create`: make the folder + `attachments/` + `workdir/` (both empty, reserved), write `meta.json`, write `messages.jsonl` (all current messages, usually zero), and create an empty `events.jsonl` (reserved; not written to until milestone 3).
- **All `meta.json` writes are atomic**: write `meta.json.tmp` in the same directory, then rename over `meta.json`. Rename is atomic on APFS. This matters because `append_message` rewrites meta on every append; a crash mid-rewrite must never corrupt the file that `list` and `load` both depend on. Never leave `.tmp` residue on the happy path.
- **Torn tails: readers tolerate, writers repair.** A crash mid-append (or a reader racing a writer) can leave `messages.jsonl` ending in a partial line with no trailing `\n`. `load` handles this *in memory*: parse every good newline-terminated line, ignore an unparseable or unterminated final line, and leave the file untouched (reads must stay read-only). `append_message` repairs it *on disk* under its write lock: truncate back to the end of the last good line before appending, so the new message lands on its own line instead of concatenating onto the torn tail. The torn message was never acknowledged as committed, so dropping it is correct. An unparseable line anywhere *else* is real corruption and returns `CoreError::MalformedVault`; never silently skip interior lines.
- `append_message`: take the write lock, repair any torn tail, append exactly one line to `messages.jsonl` (append mode), then atomically rewrite `meta.json` with a bumped `updated_at`. **Never rewrite `messages.jsonl` on append.**
- `save_meta`: take the write lock, atomically rewrite `meta.json` only.
- `load`: resolve id to folder (scan `meta.json` files matching `id`, or keep a small in-memory `id -> folder` map built on first list), read meta + every line of `messages.jsonl` with in-memory torn-tail tolerance, reconstruct. No locks.
- `list`: scan each conversation folder's `meta.json`, sort by `updated_at` desc. No locks. (Tiny files; fast enough. Add a SQLite cache later only if needed.)
- **Duplicate ids**: a legible vault invites Finder duplication, and sync services produce "conflicted copy" folders, so two folders can carry the same id. Resolution is deterministic: the winner is the folder with the newest `updated_at`; ties break to the lexicographically smallest folder name. `list` shows one entry per id; `load` loads the winner. Never auto-delete the loser. Report duplicates through the issues seam below so the UI can surface them later.
- **Issues seam** (`vault/mod.rs`): scanning already visits every `meta.json`, so collect anomalies cheaply and expose them through `issues()` on the trait for later UI:
  ```rust
  pub enum VaultIssue {
      DuplicateId { id: Id, winner: PathBuf, losers: Vec<PathBuf> },
      TornTailDetected { id: Id },
      UnreadableFolder { path: PathBuf, reason: String },
  }
  ```
  M1 only populates it; rendering (a sidebar badge, reveal-in-Finder) is a later shell concern.
- `delete`: take the write lock, remove the folder. `load` on a missing id returns `CoreError::NotFound`.
- `import_folder_as_new`: read `src`'s meta + messages, assign a new id, clear `forked_from`, write a new folder.
- Sync-conflict stance (document, do not build): the vault syncs through the user's own iCloud/Dropbox/Git. `flock` coordinates processes on one machine only; conflicting concurrent edits from two machines are the sync tool's domain, and tamtri tolerates the artifacts conflicts produce (duplicate-id folders, per above). Say this plainly in `/docs/vault-format.md` so it is a decision, not a surprise.

Streaming note (for later milestones, stated now so the format is honored): while a harness streams a message, buffer it and call `append_message` exactly once on completion. In-flight tokens never hit the log.

## Task 6: Errors (`error.rs`)

```rust
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(u32),
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("conversation not found: {0}")]
    NotFound(Id),
    #[error("malformed vault: {0}")]
    MalformedVault(String),
    #[error("conversation is being written by another process: {0}")]
    ConversationBusy(Id),
}

pub type Result<T> = std::result::Result<T, CoreError>;
```

## Task 7: Tests

Unit tests beside their modules; vault tests in `core/tests/` using `tempfile`.

1. `meta_message_round_trip` — build a conversation with at least one message per `ContentBlock` variant (including `Artifact`); `create` then `load` equals the original.
2. `messages_jsonl_is_append_only` — `append_message` twice, assert `messages.jsonl` has exactly two lines; append a third, assert the first two lines are byte-identical (never rewritten).
3. `meta_is_versioned` — `meta.json` parses to an object with `schema_version == 1`.
4. `load_rejects_future_version` — write a `meta.json` with `schema_version = 999`; `load` returns `UnsupportedSchemaVersion(999)`.
5. `content_block_tagging` — each `ContentBlock` variant serializes with the correct `"type"` tag and round-trips unchanged (explicitly include `Thinking` and `Artifact`).
6. `fork_new_id_and_backpointer` — `fork.id != c.id` and `fork.forked_from == Some(c.id)`.
7. `fork_is_deep_copy` — push a message to the fork; the original's `messages` length is unchanged.
8. `import_folder_as_new_assigns_new_id` — id differs from source; `forked_from` is `None`; content preserved.
9. `vault_list_newest_first` — create three with increasing `updated_at`; `list` returns them newest first with matching titles.
10. `vault_delete_then_load_not_found` — `delete`, then `load` returns `NotFound`.
11. `artifact_path_under_attachments` — an `Artifact` block's `path` is vault-relative and under `attachments/`.
12. `load_tolerates_torn_final_line` — truncate `messages.jsonl` mid-line (no trailing newline); `load` returns the intact messages and the file on disk is byte-unchanged (reads are read-only). A syntactically complete final JSON object without a trailing newline is still not committed and is ignored.
13. `append_repairs_torn_tail_on_disk` — with a torn tail on disk, `append_message` truncates back to the last good line first; the appended message occupies its own line and the whole file round-trips.
14. `malformed_interior_line_is_hard_error` — corrupt a middle line; `load` returns `MalformedVault`, never silently skips.
15. `renamed_folder_still_loads` — rename a conversation folder to an arbitrary name; `load` by id and `list` still work.
16. `duplicate_id_resolves_to_newest` — copy a conversation folder under a new name, bump one copy's `updated_at`; `list` shows one entry, `load` returns the newer one, and `issues()` reports a `DuplicateId` naming winner and loser.
17. `read_succeeds_while_write_lock_held` — hold the exclusive lock on a conversation's `messages.jsonl` from a second handle; `load` and `list` still succeed.
18. `contended_write_returns_busy` — with the lock held elsewhere, `append_message` returns `ConversationBusy` after the bounded retry.
19. `parallel_writes_to_different_conversations` — two conversations append concurrently without contention.
20. `artifact_inline_respects_threshold` — an `Artifact` with `inline` text over `ARTIFACT_INLINE_MAX_BYTES` is rejected by the constructor and by `load`; inline content with a non-text MIME type is rejected too.
21. `artifact_path_traversal_rejected` — artifact paths outside `attachments/`, absolute paths, and paths with `.` or `..` components are rejected by the constructor and by `load`.

## Out of scope this milestone (do not build yet)

MCP client, gateway, harness adapters, ACP, FFI/UniFFI, any SwiftUI, cloud sync, accounts, `.tamtri` share bundle (zip), SQLite index, **writing to `events.jsonl`** (reserved only). The core must not import any platform or UI crate.

## Kickoff prompt for Claude Code

> Read CLAUDE.md, then implement Milestone 1 from milestone-1.md. Start with Task 1 (workspace scaffold) and Task 2 (domain model), then stop and show me the types before continuing to the vault persistence format.
