/**
 * @tamtri/relay
 *
 * E2EE relay primitives shared by the daemon (which opens an outbound relay
 * connection) and remote clients (which pair via QR/URL). The relay server is
 * zero-knowledge; it routes encrypted frames by session id only.
 */

export type { EncryptedChannel, KeyPair } from "./e2ee.js";
export {
  createClientChannel,
  decryptFromBase64,
  encryptToBase64,
  generateKeyPair,
} from "./e2ee.js";
export {
  decodeOfferFragment,
  encodeOfferFragment,
  parseConnectionOfferInput,
  parsedToWire,
  wireToParsed,
  type ParsedConnectionOffer,
} from "./offer.js";

/** Pairing offer encoded into a QR code / deep-link URL fragment. */
export interface ConnectionOffer {
  version: 1;
  serverId: string;
  daemonPublicKeyB64: string;
  relay: {
    endpoint: string;
    useTls: boolean;
  };
}
