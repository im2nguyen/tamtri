//! macOS login keychain storage for the credentials master key.

use crate::{CoreError, Result};

const SERVICE: &str = "dev.tamtri.credentials";
const ACCOUNT: &str = "master-key";

pub fn load_master_key() -> Result<Option<[u8; 32]>> {
    match security_framework::passwords::get_generic_password(SERVICE, ACCOUNT) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(Some(key))
        }
        Ok(bytes) => Err(CoreError::Protocol(format!(
            "keychain master key has invalid length: {}",
            bytes.len()
        ))),
        Err(_) => Ok(None),
    }
}

pub fn store_master_key(key: &[u8; 32]) -> Result<()> {
    security_framework::passwords::set_generic_password(SERVICE, ACCOUNT, key).map_err(|err| {
        CoreError::Protocol(format!("keychain write failed: {err}"))
    })
}

pub fn delete_master_key_file_fallback(path: &std::path::Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
