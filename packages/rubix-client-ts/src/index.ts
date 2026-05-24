// # @nube/rubix-client-ts
//
// TS HTTP client mirroring rubix-agent's REST surface. Generated
// from `rubix/openapi.json` via `pnpm --filter @nube/rubix-client-ts
// codegen` — hand-edits to `src/generated/*` are forbidden.
//
// `RubixClient` wraps a `StarterClient` from `@nube/starter-client-ts`
// so transport configuration lives in exactly one place. Endpoint
// modules will hang methods off `RubixClient` via TS declaration
// merging, mirroring the starter client pattern.

export { RubixClient } from "./client/client.js";
export { RubixError } from "./error/rubix-error.js";
