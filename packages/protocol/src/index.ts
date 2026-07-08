/**
 * @tamtri/protocol
 *
 * The shared wire-protocol contract for every tamtri surface. The Rust core
 * (core/src/protocol) is the source of truth: data types are generated into
 * ./generated.ts by typeshare (`npm run protocol:generate`). The JSON-RPC
 * envelope, the method registry, and the protocol version are hand-written here
 * because typeshare emits types only, not constants; they are kept in sync with
 * the Rust source by hand.
 */

/** Wire protocol version. Mirrors PROTOCOL_VERSION in core/src/protocol/mod.rs. */
export const PROTOCOL_VERSION = "1.0";

export * from "./generated.js";
export * from "./jsonrpc.js";
export { method, type MethodName } from "./methods.js";
