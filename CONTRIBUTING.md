# Contributing

tamtri is AGPL-licensed with a planned contributor rights path that preserves a possible future MIT relicense.

Before outside PRs are accepted, the project will require one of:

- A Contributor License Agreement.
- A Developer Certificate of Origin sign-off.

Until that process is wired into CI, open issues and design discussion are welcome, but code PRs should wait for maintainer guidance.

## Local Checks

Run these before sending core changes:

```sh
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
```

## Project Rules

- Keep the client a dumb shell: no agent loop, prompting strategy, or inference logic in tamtri.
- Keep storage legible: the vault is the source of truth.
- Keep the Rust core platform-agnostic.
- Update the relevant docs when implementation behavior drifts.

