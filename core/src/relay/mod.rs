//! E2EE relay primitives for remote client access.
//!
//! The relay server routes ciphertext only; pairing uses a Curve25519 keypair
//! persisted under the tamtri home directory. Full outbound relay attachment is
//! wired when a relay endpoint is configured (`TAMTRI_RELAY_ENDPOINT`).

mod crypto;
mod keypair;
mod pairing;
mod server_id;

pub use crypto::{KeyPair, decode_base64, encode_base64};
pub use keypair::{load_or_create_keypair, keypair_path, KeypairFile};
pub use pairing::{ConnectionOffer, RelayEndpoint, build_pairing_offer, DEFAULT_RELAY_ENDPOINT};
pub use server_id::load_or_create_server_id;
