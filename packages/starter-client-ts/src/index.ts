// # @nube/starter-client-ts
//
// TS HTTP client mirroring starter-server's surface. Generated from
// the server's OpenAPI document via `pnpm codegen` — hand-edits to
// `src/generated/*` are forbidden (SCOPE.md R7).
//
// One responsibility per file:
//
// - `client/` — the `StarterClient` class + builder.
// - `endpoints/` — one file per endpoint family (mirrors starter-server).
// - `generated/` — codegen output. Never hand-edited.
// - `error/` — `StarterError` shape.

export { StarterClient } from "./client/client.js";
export { StarterError } from "./error/starter-error.js";
export type { Problem } from "./error/starter-error.js";
// Shared transport helpers — re-exported so sibling packages (e.g.
// `@nube/rubix-client-ts`) can hang endpoint methods off a wrapped
// `StarterClient` without re-implementing URL build, credentials
// handling, CSRF cookie reads, or typed-error parsing.
export { fetchJson } from "./client/fetch_json.js";
export { fetchVoid } from "./client/fetch_void.js";
export { fetchBytes } from "./client/fetch_bytes.js";
export { readCsrfHeader } from "./client/csrf.js";
export { streamJson } from "./client/stream_json.js";
export type { StreamJsonOptions } from "./client/stream_json.js";
export * from "./endpoints/index.js";
