/**
 * End-to-end encrypted channel. The relay server only forwards ciphertext; keys
 * never leave the paired devices.
 *
 * Structural stub. The relay step wires tweetnacl (Curve25519 key exchange +
 * XSalsa20-Poly1305 box) behind this interface, so the client and daemon share
 * one channel implementation.
 */

export interface EncryptedChannel {
  encrypt(plaintext: Uint8Array): Uint8Array;
  decrypt(ciphertext: Uint8Array): Uint8Array;
}

export interface KeyPair {
  publicKey: Uint8Array;
  secretKey: Uint8Array;
}
