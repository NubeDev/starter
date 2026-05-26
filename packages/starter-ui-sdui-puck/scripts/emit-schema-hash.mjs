#!/usr/bin/env node
// Schema-hash emitter — scope §B6 (runtime banner piece).
//
// Hashes the committed `starter-ui-ir` JSON Schema artifact and
// writes a JSON sidecar at `src/schema-hash.json` that the package
// imports at build time. The PuckBuilder exposes this hash on a
// prop so the host frontend can compare it against the live hash
// served by rubix-agent and surface a "schema drifted — refresh to
// reload the palette" banner inside the canvas.
//
// This is complementary to `check-schema-drift.mjs`: that script
// catches drift at PR time (Rust IR vs. committed artifact); this
// one catches drift at *runtime*, when rubix-agent and the
// frontend bundle were built against different schema revisions
// and deployed independently.
//
// The sidecar is committed so the package builds without a
// pre-step on consumer machines. CI re-runs the emitter via
// `pnpm test` and fails if the committed sidecar is stale (the
// emitter exits non-zero when its write would change the file
// unless `RUBIX_PUCK_HASH_WRITE=1` is set, which the emitter sets
// for its own self-call).

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = resolve(HERE, "..");
const REPO_ROOT = resolve(PKG_ROOT, "..", "..");
const SCHEMA_PATH = resolve(
  REPO_ROOT,
  "crates",
  "starter-ui-ir",
  "schema",
  "starter-ui-ir.schema.json",
);
const SIDECAR_PATH = resolve(PKG_ROOT, "src", "schema-hash.json");

function main() {
  const schemaBytes = readFileSync(SCHEMA_PATH);
  const hash = createHash("sha256").update(schemaBytes).digest("hex");
  const next = JSON.stringify({ hash, algorithm: "sha256" }, null, 2) + "\n";

  let prev = "";
  try {
    prev = readFileSync(SIDECAR_PATH, "utf8");
  } catch {
    // First emission — fall through and write.
  }

  if (prev === next) {
    return;
  }

  if (process.env.RUBIX_PUCK_HASH_WRITE === "1") {
    writeFileSync(SIDECAR_PATH, next);
    return;
  }

  writeFileSync(SIDECAR_PATH, next);
  if (prev !== "") {
    console.error(
      "[puck/schema-hash] src/schema-hash.json was stale and has been rewritten.\n" +
        "  Commit the regenerated sidecar so the runtime banner reflects the committed schema.",
    );
    process.exit(1);
  }
}

main();
