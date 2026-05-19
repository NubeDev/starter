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
export * from "./endpoints/index.js";
