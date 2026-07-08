/*
 * GENERATED FILE — do not edit by hand.
 *
 * These types are generated from the Rust source of truth in core/src/protocol
 * (see core/src/protocol/mod.rs and params.rs) via typeshare.
 *
 * Regenerate with:  npm run protocol:generate
 *
 * This placeholder ships a hand-written stub so the workspace type-checks before
 * the typeshare pipeline is wired in the protocol+client spine step. Once
 * generation runs, this file is overwritten with the full set of protocol types.
 */

/** Wire protocol version. Mirrors PROTOCOL_VERSION in core/src/protocol/mod.rs. */
export const PROTOCOL_VERSION = "1.0";

/** Client identifies itself in the hello handshake. */
export type ClientType = "desktop" | "web" | "mobile" | "cli";
