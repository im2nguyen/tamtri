//! Library surface for `tamtri-daemon` so the transport can be integration
//! tested. The binary in `main.rs` is a thin wrapper over these modules.

pub mod runtime_dir;
pub mod server;
