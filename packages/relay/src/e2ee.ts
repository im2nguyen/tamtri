/**
 * E2EE crypto primitives using NaCl (tweetnacl).
 *
 * Key exchange: Curve25519 (nacl.box.before)
 * Encryption: XSalsa20-Poly1305
 *
 * Wire format: base64([nonce (24 bytes)][ciphertext...])
 */

import nacl from "tweetnacl";
import { fromByteArray, toByteArray } from "base64-js";

export interface KeyPair {
  publicKey: Uint8Array;
  secretKey: Uint8Array;
}

export type SharedKey = Uint8Array;

const NONCE_LENGTH = nacl.box.nonceLength;

function encodeBase64(bytes: Uint8Array): string {
  return fromByteArray(bytes);
}

function decodeBase64(base64: string): Uint8Array {
  return toByteArray(base64);
}

export function generateKeyPair(): KeyPair {
  const pair = nacl.box.keyPair();
  return { publicKey: pair.publicKey, secretKey: pair.secretKey };
}

export function deriveSharedKey(localSecret: Uint8Array, remotePublic: Uint8Array): SharedKey {
  return nacl.box.before(remotePublic, localSecret);
}

export function encrypt(sharedKey: SharedKey, plaintext: Uint8Array): Uint8Array {
  const nonce = nacl.randomBytes(NONCE_LENGTH);
  const ciphertext = nacl.box.after(plaintext, nonce, sharedKey);
  const out = new Uint8Array(NONCE_LENGTH + ciphertext.length);
  out.set(nonce, 0);
  out.set(ciphertext, NONCE_LENGTH);
  return out;
}

export function decrypt(sharedKey: SharedKey, packed: Uint8Array): Uint8Array | null {
  if (packed.length <= NONCE_LENGTH) return null;
  const nonce = packed.subarray(0, NONCE_LENGTH);
  const ciphertext = packed.subarray(NONCE_LENGTH);
  return nacl.box.open.after(ciphertext, nonce, sharedKey);
}

export function encryptToBase64(sharedKey: SharedKey, plaintext: string): string {
  return encodeBase64(encrypt(sharedKey, new TextEncoder().encode(plaintext)));
}

export function decryptFromBase64(sharedKey: SharedKey, encoded: string): string | null {
  const plain = decrypt(sharedKey, decodeBase64(encoded));
  if (!plain) return null;
  return new TextDecoder().decode(plain);
}

export interface EncryptedChannel {
  encrypt(plaintext: Uint8Array): Uint8Array;
  decrypt(ciphertext: Uint8Array): Uint8Array;
}

export function createClientChannel(localSecret: Uint8Array, daemonPublicB64: string): EncryptedChannel {
  const daemonPublic = decodeBase64(daemonPublicB64);
  const shared = deriveSharedKey(localSecret, daemonPublic);
  return {
    encrypt: (plaintext) => {
      const out = encrypt(shared, plaintext);
      if (!out) throw new Error("encrypt failed");
      return out;
    },
    decrypt: (ciphertext) => {
      const out = decrypt(shared, ciphertext);
      if (!out) throw new Error("decrypt failed");
      return out;
    },
  };
}
