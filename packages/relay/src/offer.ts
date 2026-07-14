/**
 * Encode/decode relay pairing offers into tamtri:// deep links or URL fragments.
 *
 * Wire format matches core/src/relay/pairing.rs (snake_case JSON).
 */

import type { ConnectionOffer as WireOffer } from "@tamtri/protocol";

export interface ParsedConnectionOffer {
  v: number;
  serverId: string;
  daemonPublicKeyB64: string;
  relay: {
    endpoint: string;
    useTls: boolean;
  };
}

const OFFER_FRAGMENT_PREFIX = "offer=";

export function wireToParsed(offer: WireOffer): ParsedConnectionOffer {
  return {
    v: offer.v,
    serverId: offer.server_id,
    daemonPublicKeyB64: offer.daemon_public_key_b64,
    relay: {
      endpoint: offer.relay.endpoint,
      useTls: offer.relay.use_tls,
    },
  };
}

export function parsedToWire(offer: ParsedConnectionOffer): WireOffer {
  return {
    v: offer.v,
    server_id: offer.serverId,
    daemon_public_key_b64: offer.daemonPublicKeyB64,
    relay: {
      endpoint: offer.relay.endpoint,
      use_tls: offer.relay.useTls,
    },
  };
}

export function encodeOfferFragment(offer: WireOffer): string {
  const json = JSON.stringify(offer);
  const b64 = btoa(json);
  return `${OFFER_FRAGMENT_PREFIX}${b64}`;
}

export function decodeOfferFragment(fragment: string): ParsedConnectionOffer {
  const trimmed = fragment.trim().replace(/^#/, "");
  const raw = trimmed.startsWith(OFFER_FRAGMENT_PREFIX)
    ? trimmed.slice(OFFER_FRAGMENT_PREFIX.length)
    : trimmed;
  if (!raw) {
    throw new Error("Missing offer payload");
  }
  let json: string;
  try {
    json = atob(raw);
  } catch {
    throw new Error("Offer payload is not valid base64");
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    throw new Error("Offer payload is not valid JSON");
  }
  return normalizeOffer(parsed);
}

export function parseConnectionOfferInput(input: string): ParsedConnectionOffer {
  const trimmed = input.trim();
  if (!trimmed) {
    throw new Error("Paste a pairing offer URL or JSON");
  }

  if (trimmed.startsWith("{")) {
    return normalizeOffer(JSON.parse(trimmed) as unknown);
  }

  try {
    const url = new URL(trimmed);
    if (url.hash) {
      return decodeOfferFragment(url.hash);
    }
    if (url.pathname.includes("offer")) {
      return decodeOfferFragment(url.pathname);
    }
  } catch {
    // fall through — try raw fragment/base64
  }

  if (trimmed.startsWith(OFFER_FRAGMENT_PREFIX) || trimmed.startsWith("#")) {
    return decodeOfferFragment(trimmed);
  }

  return decodeOfferFragment(`${OFFER_FRAGMENT_PREFIX}${trimmed}`);
}

function normalizeOffer(value: unknown): ParsedConnectionOffer {
  if (!value || typeof value !== "object") {
    throw new Error("Invalid pairing offer");
  }
  const record = value as Record<string, unknown>;
  const v = typeof record.v === "number" ? record.v : typeof record.version === "number" ? record.version : null;
  const serverId =
    typeof record.server_id === "string"
      ? record.server_id
      : typeof record.serverId === "string"
        ? record.serverId
        : null;
  const daemonPublicKeyB64 =
    typeof record.daemon_public_key_b64 === "string"
      ? record.daemon_public_key_b64
      : typeof record.daemonPublicKeyB64 === "string"
        ? record.daemonPublicKeyB64
        : null;
  const relayRaw = record.relay;
  if (!relayRaw || typeof relayRaw !== "object") {
    throw new Error("Pairing offer missing relay endpoint");
  }
  const relayRecord = relayRaw as Record<string, unknown>;
  const endpoint = typeof relayRecord.endpoint === "string" ? relayRecord.endpoint : null;
  const useTls =
    typeof relayRecord.use_tls === "boolean"
      ? relayRecord.use_tls
      : typeof relayRecord.useTls === "boolean"
        ? relayRecord.useTls
        : endpoint?.endsWith(":443") ?? false;

  if (v !== 1 || !serverId || !daemonPublicKeyB64 || !endpoint) {
    throw new Error("Pairing offer is missing required fields");
  }

  return {
    v,
    serverId,
    daemonPublicKeyB64,
    relay: { endpoint, useTls },
  };
}
