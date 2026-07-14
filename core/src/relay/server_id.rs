//! Stable daemon identity for relay pairing.

use std::fs;
use std::path::Path;

use crate::Result;

const SERVER_ID_FILE: &str = "server-id";

pub fn load_or_create_server_id(home: &Path) -> Result<String> {
    fs::create_dir_all(home)?;
    let path = home.join(SERVER_ID_FILE);
    if path.exists() {
        let id = fs::read_to_string(&path)?.trim().to_string();
        if !id.is_empty() {
            return Ok(id);
        }
    }
    let id = format!("srv_{}", uuid::Uuid::now_v7().simple());
    let tmp = home.join(format!("{SERVER_ID_FILE}.tmp"));
    fs::write(&tmp, &id)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(tmp, path)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn server_id_is_stable() {
        let dir = TempDir::new().unwrap();
        let first = load_or_create_server_id(dir.path()).unwrap();
        let second = load_or_create_server_id(dir.path()).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("srv_"));
    }
}
