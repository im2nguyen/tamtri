//! `tamtri-daemon`: the headless host process. It owns a single
//! [`tamtri_core::daemon::Daemon`] and serves the wire protocol over a
//! localhost WebSocket. Clients (the macOS shell today, web/mobile later)
//! connect to it; the desktop app spawns and supervises it.

use std::net::SocketAddr;
use std::sync::Arc;

use tamtri_core::daemon::Daemon;
use tamtri_daemon::{relay_attachment, runtime_dir, server};

/// Default localhost port. Override with `TAMTRI_PORT`; `0` binds an ephemeral
/// port (the actual port is written to `daemon.port`).
const DEFAULT_PORT: u16 = 8377;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let paths = runtime_dir::RuntimePaths::resolve()?;
    tracing::info!(home = %paths.home.display(), "tamtri home");
    let token = runtime_dir::ensure_token(&paths.token_file)?;

    // Build the core before starting the server runtime. The core owns its own
    // tokio runtime internally; constructing (and later dropping) it must happen
    // off any async executor, which is why `main` is a plain fn.
    let daemon = Arc::new(Daemon::new(paths.vault.to_string_lossy().to_string())?);
    let server_id = daemon.core().server_id().to_string();

    let port: u16 = std::env::var("TAMTRI_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let bind_host = std::env::var("TAMTRI_BIND").unwrap_or_else(|_| "127.0.0.1".to_string());
    let ip: std::net::IpAddr = bind_host
        .parse()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    let addr = SocketAddr::from((ip, port));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let result = rt.block_on(async {
        relay_attachment::spawn_if_enabled(server_id, paths.home.clone());
        let listener = server::bind(addr).await?;
        let local = listener.local_addr()?;
        runtime_dir::write_endpoint_files(&paths, local.port())?;
        tracing::info!(bind = %ip, port = local.port(), "tamtri-daemon listening");
        server::serve(listener, Arc::clone(&daemon), token).await
    });

    runtime_dir::clear_endpoint_files(&paths);
    // `rt` drops here (sync context), then `daemon`, so the core's internal
    // runtime is never dropped from within an async context.
    result?;
    Ok(())
}
