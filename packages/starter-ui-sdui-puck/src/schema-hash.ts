// Build-time import of the schema-hash sidecar emitted by
// `scripts/emit-schema-hash.mjs`. The sidecar is committed so the
// package builds without a pre-step on consumer machines; CI
// re-runs the emitter from `pnpm test` and fails if it would
// change the file.
//
// The hash is sha256 over the raw bytes of the committed
// `starter-ui-ir.schema.json` artifact. `PuckBuilder` exposes this
// to the host frontend on a prop so the host can compare against
// the live hash served by rubix-agent and surface a banner inside
// the canvas when they diverge — see scope §B6.

// eslint-disable-next-line import/extensions -- JSON resolution.
import sidecar from "./schema-hash.json" with { type: "json" };

interface SchemaHashSidecar {
  hash: string;
  algorithm: string;
}

const typed = sidecar as SchemaHashSidecar;

/** sha256 of the committed `starter-ui-ir.schema.json` bytes. */
export const IR_SCHEMA_HASH: string = typed.hash;

/** Hash algorithm — currently always `"sha256"`. */
export const IR_SCHEMA_HASH_ALGORITHM: string = typed.algorithm;
