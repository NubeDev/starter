// Verifies the build-time schema-hash sidecar tracks the committed
// IR JSON Schema bytes — the runtime-drift-banner mechanism (§B6)
// depends on this being a faithful fingerprint.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { IR_SCHEMA_HASH, IR_SCHEMA_HASH_ALGORITHM } from "../schema-hash.js";

const SCHEMA_PATH = resolve(
  __dirname,
  "..",
  "..",
  "..",
  "..",
  "crates",
  "starter-ui-ir",
  "schema",
  "starter-ui-ir.schema.json",
);

describe("IR_SCHEMA_HASH", () => {
  it("matches sha256(committed schema bytes)", () => {
    const bytes = readFileSync(SCHEMA_PATH);
    const expected = createHash("sha256").update(bytes).digest("hex");
    expect(IR_SCHEMA_HASH).toBe(expected);
    expect(IR_SCHEMA_HASH_ALGORITHM).toBe("sha256");
  });

  it("is a 64-char lowercase hex string", () => {
    expect(IR_SCHEMA_HASH).toMatch(/^[0-9a-f]{64}$/);
  });
});
