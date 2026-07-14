//! Persist the daemon's long-lived relay keypair.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::crypto::{KeyPair, decode_base64, encode_base64};
use crate::{CoreError, Result};

const KEYPAIR_FILE: &str = "daemon-keypair.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeypairFile {
    pub public_key_b64: String,
    pub secret_key_b64: String,
}

pub fn keypair_path(home: &Path) -> PathBuf {
    home.join(KEYPAIR_FILE)
}

pub fn load_or_create_keypair(home: &Path) -> Result<KeyPair> {
    let path = keypair_path(home);
    if path.exists() {
        let file: KeypairFile = serde_json::from_slice(&fs::read(&path)?)?;
        let public = decode_base64(&file.public_key_b64)?;
        let secret = decode_base64(&file.secret_key_b64)?;
        if public.len() != 32 || secret.len() != 32 {
            return Err(CoreError::Protocol(
                "invalid daemon keypair file".to_string(),
            ));
        }
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 32];
        pk.copy_from_slice(&public);
        sk.copy_from_slice(&secret);
        return KeyPair::from_bytes(pk, sk);
    }
    let pair = KeyPair::generate();
    let file = KeypairFile {
        public_key_b64: encode_base64(&pair.public_key_bytes()),
        secret_key_b64: encode_base64(pair.secret.to_bytes().as_ref()),
    };
    fs::create_dir_all(home)?;
    let tmp = home.join(format!("{KEYPAIR_FILE}.tmp"));
    fs::write(&tmp, serde_json::to_vec_pretty(&file)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(tmp, path)?;
    Ok(pair)
}
