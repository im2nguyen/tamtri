//! Runtime directory management for the daemon: `~/.tamtri` (override with
//! `TAMTRI_HOME`) holds the vault plus the process's socket-adjacent files - the
//! auth token, the bound port, and the pidfile - mirroring paseo's
//! `$PASEO_HOME` layout.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Resolved locations under the tamtri home directory.
pub struct RuntimePaths {
    pub home: PathBuf,
    pub vault: PathBuf,
    pub token_file: PathBuf,
    pub port_file: PathBuf,
    pub pid_file: PathBuf,
}

impl RuntimePaths {
    /// Resolve `TAMTRI_HOME` (or `~/.tamtri`) and create it if missing.
    pub fn resolve() -> io::Result<Self> {
        let home = match std::env::var_os("TAMTRI_HOME") {
            Some(value) => PathBuf::from(value),
            None => dirs::home_dir()
                .ok_or_else(|| io::Error::other("could not resolve home directory"))?
                .join(".tamtri"),
        };
        fs::create_dir_all(&home)?;
        Ok(Self {
            vault: home.join("vault"),
            token_file: home.join("daemon.token"),
            port_file: home.join("daemon.port"),
            pid_file: home.join("daemon.pid"),
            home,
        })
    }
}

/// Load the existing local auth token or mint a new one. The token gates every
/// connection so a stray localhost process cannot attach. Written `0600`.
///
/// This is a localhost bearer token; remote access will go through the E2E relay
/// with its own key exchange rather than shipping this token off-box.
pub fn ensure_token(path: &Path) -> io::Result<String> {
    if let Ok(existing) = fs::read_to_string(path) {
        let trimmed = existing.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    let token = uuid::Uuid::new_v4().simple().to_string();
    write_private(path, &token)?;
    Ok(token)
}

/// Record the bound port and the pid so clients can discover a running daemon.
pub fn write_endpoint_files(paths: &RuntimePaths, port: u16) -> io::Result<()> {
    write_private(&paths.port_file, &port.to_string())?;
    write_private(&paths.pid_file, &std::process::id().to_string())?;
    Ok(())
}

/// Remove the port/pid files on shutdown so a stale endpoint is not advertised.
pub fn clear_endpoint_files(paths: &RuntimePaths) {
    let _ = fs::remove_file(&paths.port_file);
    let _ = fs::remove_file(&paths.pid_file);
}

fn write_private(path: &Path, contents: &str) -> io::Result<()> {
    fs::write(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
