//! Curve25519 keypair helpers for the E2EE relay channel.

use crypto_box::{PublicKey, SecretKey};

use crate::{CoreError, Result};

pub struct KeyPair {
    pub public: PublicKey,
    pub secret: SecretKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let secret = SecretKey::generate(&mut rand::rngs::OsRng);
        let public = secret.public_key();
        Self { public, secret }
    }

    pub fn from_bytes(public: [u8; 32], secret: [u8; 32]) -> Result<Self> {
        Ok(Self {
            public: PublicKey::from(public),
            secret: SecretKey::from(secret),
        })
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }
}

pub fn encode_base64(bytes: &[u8]) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
}

pub fn decode_base64(text: &str) -> Result<Vec<u8>> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, text)
        .map_err(|err| CoreError::Protocol(format!("base64 decode: {err}")))
}
