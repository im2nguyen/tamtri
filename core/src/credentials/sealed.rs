//! Sealed credential file at `{tamtri_home}/credentials.sealed`.
//!
//! Values are encrypted with ChaCha20-Poly1305 under a 32-byte master key stored
//! at `{tamtri_home}/credentials.key` (0600). This is the portable fallback when
//! no OS keychain is available (Linux, headless). A future macOS enhancement can
//! wrap the master key in the login keychain via `security-framework`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use chacha20poly1305::aead::{Aead, KeyInit, OsRng, rand_core::RngCore};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use serde::{Deserialize, Serialize};

use crate::mcp::gateway::CredentialResolver;
use crate::{CoreError, Result};

const KEY_FILE: &str = "credentials.key";
const SEALED_FILE: &str = "credentials.sealed";
const NONCE_LEN: usize = 12;

#[derive(Serialize, Deserialize, Default)]
struct SealedBlob {
    version: u32,
    #[serde(default)]
    entries: HashMap<String, String>,
}

/// In-memory cache backed by an encrypted on-disk file under the tamtri home dir.
pub struct DurableCredentials {
    home: PathBuf,
    cache: Mutex<HashMap<String, String>>,
}

impl DurableCredentials {
    pub fn open(home: impl AsRef<Path>) -> Result<Self> {
        let home = home.as_ref().to_path_buf();
        fs::create_dir_all(&home)?;
        let cache = load_sealed(&home)?;
        Ok(Self {
            home,
            cache: Mutex::new(cache),
        })
    }

    pub fn set(&self, credential_ref: String, value: String) -> Result<()> {
        {
            let mut cache = self.cache.lock().map_err(lock_err)?;
            cache.insert(credential_ref, value);
            persist_sealed(&self.home, &cache)?;
        }
        Ok(())
    }

    pub fn contains(&self, credential_ref: &str) -> Result<bool> {
        Ok(self
            .cache
            .lock()
            .map_err(lock_err)?
            .contains_key(credential_ref))
    }

    pub fn get_stored(&self, credential_ref: &str) -> Result<Option<String>> {
        Ok(self
            .cache
            .lock()
            .map_err(lock_err)?
            .get(credential_ref)
            .cloned())
    }
}

#[async_trait]
impl CredentialResolver for DurableCredentials {
    async fn resolve(&self, credential_ref: &str) -> Result<Option<String>> {
        self.get_stored(credential_ref)
    }

    async fn store(&self, credential_ref: &str, value: &str) -> Result<()> {
        self.set(credential_ref.to_string(), value.to_string())
    }
}

fn lock_err<T>(_: T) -> CoreError {
    CoreError::Protocol("credential store lock poisoned".to_string())
}

fn key_path(home: &Path) -> PathBuf {
    home.join(KEY_FILE)
}

fn sealed_path(home: &Path) -> PathBuf {
    home.join(SEALED_FILE)
}

fn ensure_master_key(home: &Path) -> Result<[u8; 32]> {
    let path = key_path(home);
    if path.exists() {
        let bytes = fs::read(&path)?;
        if bytes.len() != 32 {
            return Err(CoreError::Protocol(
                "credentials.key has invalid length".to_string(),
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        return Ok(key);
    }
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    write_private(&path, &key)?;
    Ok(key)
}

fn load_sealed(home: &Path) -> Result<HashMap<String, String>> {
    let path = sealed_path(home);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let key = ensure_master_key(home)?;
    let blob: SealedBlob = serde_json::from_slice(&fs::read(&path)?)?;
    if blob.version != 1 {
        return Err(CoreError::Protocol(format!(
            "unsupported credentials.sealed version: {}",
            blob.version
        )));
    }
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|_| CoreError::Protocol("invalid master key".to_string()))?;
    let mut out = HashMap::new();
    for (credential_ref, encoded) in blob.entries {
        let bytes = base64_decode(&encoded)?;
        if bytes.len() <= NONCE_LEN {
            return Err(CoreError::Protocol(format!(
                "corrupt sealed entry for {credential_ref}"
            )));
        }
        let (nonce_bytes, ciphertext) = bytes.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CoreError::Protocol(format!("decrypt failed for {credential_ref}")))?;
        let value = String::from_utf8(plaintext).map_err(|_| {
            CoreError::Protocol(format!("invalid utf-8 in sealed entry for {credential_ref}"))
        })?;
        out.insert(credential_ref, value);
    }
    Ok(out)
}

fn persist_sealed(home: &Path, cache: &HashMap<String, String>) -> Result<()> {
    let key = ensure_master_key(home)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|_| CoreError::Protocol("invalid master key".to_string()))?;
    let mut entries = HashMap::new();
    for (credential_ref, value) in cache {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, value.as_bytes())
            .map_err(|_| CoreError::Protocol(format!("encrypt failed for {credential_ref}")))?;
        let mut packed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&ciphertext);
        entries.insert(credential_ref.clone(), base64_encode(&packed));
    }
    let blob = SealedBlob {
        version: 1,
        entries,
    };
    let path = sealed_path(home);
    let tmp = home.join(format!("{SEALED_FILE}.tmp"));
    fs::write(&tmp, serde_json::to_vec_pretty(&blob)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn base64_encode(bytes: &[u8]) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
}

fn base64_decode(text: &str) -> Result<Vec<u8>> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, text)
        .map_err(|err| CoreError::Protocol(format!("base64 decode: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn round_trip_persist_and_reload() {
        let dir = TempDir::new().unwrap();
        let store = DurableCredentials::open(dir.path()).unwrap();
        store.set("api-key".into(), "secret-value".into()).unwrap();
        drop(store);

        let reloaded = DurableCredentials::open(dir.path()).unwrap();
        assert_eq!(
            reloaded.get_stored("api-key").unwrap(),
            Some("secret-value".into())
        );
        assert!(reloaded.get_stored("missing").unwrap().is_none());
    }
}
