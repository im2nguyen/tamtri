/**
 * @tamtri/protocol
 *
 * The shared wire-protocol contract for every tamtri surface. The Rust core
 * (core/src/protocol) is the source of truth; TypeScript types are generated
 * into ./generated.ts by typeshare and re-exported here alongside any
 * hand-written helpers (envelope builders, method-name constants).
 */

export * from "./generated.js";
