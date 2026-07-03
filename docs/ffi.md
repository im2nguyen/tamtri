# FFI Surface

Milestone 3 exposes a facade for UniFFI rather than binding internal modules directly.

The exported Rust types live in `core/src/app.rs` and use UniFFI proc-macro metadata (`Record`, `Object`, `Error`, and `export`). `core/src/ffi/tamtri.udl` is kept as a readable contract mirror for reviewers and downstream implementers.

- conversation lifecycle
- send/cancel/respond-permission
- callback events through `ConversationObserver`

The Rust facade owns a Tokio runtime internally. No Tokio, chrono, serde, UUID, or harness types cross the boundary. Swift receives strings, records, and JSON payloads.

Install the generator once:

```sh
cargo install uniffi --version 0.32.0 --features cli
```

Generate Swift bindings from the compiled core library metadata:

```sh
cargo build -p tamtri-core
uniffi-bindgen generate target/debug/libtamtri_core.dylib --language swift --out-dir macos/Sources/Tamtri/Generated
```

The Swift package exposes the generated C shim through `macos/Sources/tamtri_coreFFI/module.modulemap` and links `../target/debug/libtamtri_core.dylib`. `TamtriBindingClient` adapts generated bindings to the app-facing `CoreClient`; `makeDefaultCoreClient()` falls back to `MockCoreClient` only if the native core cannot initialize.
