// Build-time import of the committed IR JSON Schema artifact owned
// by the Rust crate. Per scope §B1 / §B6 the puck package consumes
// the *committed* artifact at build time — it never re-derives the
// schema and never introduces a second copy.
//
// Vite's `resolveJsonModule` + Node's `--experimental-json-modules`
// both honour this path; vitest reads it the same way.

// eslint-disable-next-line import/extensions -- JSON resolution.
import schemaJson from "../../../crates/starter-ui-ir/schema/starter-ui-ir.schema.json" with { type: "json" };

import type { JsonSchema } from "./schema-walker.js";

/** The full committed IR JSON Schema. */
export const IR_SCHEMA: JsonSchema = schemaJson as unknown as JsonSchema;
