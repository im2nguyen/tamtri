/**
 * E2EE relay transport for remote clients (mobile/web away from home).
 *
 * The relay server routes base64 ciphertext by server_id; pairing uses the
 * daemon's long-lived public key from a ConnectionOffer (QR / deep link).
 *
 * Note: relay.tamtri.dev is not deployed yet and the daemon does not bridge
 * relay frames into the wire protocol. Prefer direct LAN for physical-device dev.
 */

import { fromByteArray, toByteArray } from "base64-js";
import {
  createClientChannel,
  generateKeyPair,
  parseConnectionOfferInput,
  parsedToWire,
  type EncryptedChannel,
  type ParsedConnectionOffer,
} from "@tamtri/relay";

import type { DaemonTransport, DaemonTransportFactory } from "./transport.js";

interface WebSocketLike {
  send(data: string): void;
  close(): void;
  addEventListener(type: string, listener: (event: unknown) => void): void;
}

type WebSocketCtor = new (url: string) => WebSocketLike;

export interface RelayTransportOptions {
  offer: ParsedConnectionOffer | string;
  webSocketImpl?: WebSocketCtor;
  connectTimeoutMs?: number;
}

interface RelayConnectAck {
  type: "connected";
  session_id?: string;
}

function resolveImpl(override?: WebSocketCtor): WebSocketCtor {
  if (override) return override;
  const global = (globalThis as { WebSocket?: WebSocketCtor }).WebSocket;
  if (!global) {
    throw new Error("No global WebSocket available; pass webSocketImpl explicitly");
  }
  return global;
}

function buildRelayClientUrl(offer: ParsedConnectionOffer): string {
  const scheme = offer.relay.useTls ? "wss" : "ws";
  const endpoint = offer.relay.endpoint.trim().replace(/\/+$/, "");
  return `${scheme}://${endpoint}/v1/client/connect?server_id=${encodeURIComponent(offer.serverId)}`;
}

function parseOffer(input: ParsedConnectionOffer | string): ParsedConnectionOffer {
  return typeof input === "string" ? parseConnectionOfferInput(input) : input;
}

function encryptText(channel: EncryptedChannel, text: string): string {
  const bytes = new TextEncoder().encode(text);
  const packed = channel.encrypt(bytes);
  return fromByteArray(packed);
}

function decryptText(channel: EncryptedChannel, encoded: string): string | null {
  const packed = toByteArray(encoded);
  const plain = channel.decrypt(packed);
  if (!plain) return null;
  return new TextDecoder().decode(plain);
}

export function relayTransport(options: RelayTransportOptions): DaemonTransportFactory {
  return () =>
    new Promise<DaemonTransport>((resolve, reject) => {
      const offer = parseOffer(options.offer);
      const Impl = resolveImpl(options.webSocketImpl);
      const clientKeys = generateKeyPair();
      const channel = createClientChannel(clientKeys.secretKey, offer.daemonPublicKeyB64);
      const url = buildRelayClientUrl(offer);
      const socket = new Impl(url);
      let settled = false;
      let onMessageHandler: ((data: string) => void) | undefined;
      let onCloseHandler: (() => void) | undefined;

      const fail = (error: Error) => {
        if (!settled) {
          settled = true;
          reject(error);
        }
      };

      const timeout = setTimeout(() => {
        fail(
          new Error(
            `Relay connect timed out (${offer.relay.endpoint}). ` +
              "The hosted relay is not available yet — use direct LAN pairing for iPhone dev.",
          ),
        );
        socket.close();
      }, options.connectTimeoutMs ?? 12_000);

      socket.addEventListener("open", () => {
        const register = JSON.stringify({
          type: "connect",
          v: parsedToWire(offer).v,
          server_id: offer.serverId,
          client_public_key_b64: fromByteArray(clientKeys.publicKey),
        });
        socket.send(register);
      });

      socket.addEventListener("message", (event) => {
        const data = (event as { data?: unknown }).data;
        if (typeof data !== "string") return;

        if (!settled) {
          let ack: RelayConnectAck | null = null;
          try {
            ack = JSON.parse(data) as RelayConnectAck;
          } catch {
            // encrypted early frame — ignore until ack
          }
          if (ack?.type === "connected") {
            clearTimeout(timeout);
            settled = true;
            resolve({
              send: (frame) => socket.send(encryptText(channel, frame)),
              close: () => socket.close(),
              onMessage: (handler) => {
                onMessageHandler = handler;
              },
              onClose: (handler) => {
                onCloseHandler = handler;
              },
            });
            return;
          }
        }

        const plaintext = decryptText(channel, data);
        if (plaintext != null) {
          onMessageHandler?.(plaintext);
        }
      });

      socket.addEventListener("error", () => {
        clearTimeout(timeout);
        fail(
          new Error(
            `Relay connection to ${offer.relay.endpoint} failed. ` +
              "The hosted relay is not available yet — use direct LAN pairing for iPhone dev.",
          ),
        );
      });

      socket.addEventListener("close", () => {
        clearTimeout(timeout);
        if (!settled) {
          fail(new Error("Relay closed before connect completed"));
        }
        onCloseHandler?.();
      });
    });
}
