#!/usr/bin/env node
// Schema-drift guard — scope §B6 (PR1 piece, CI-only).
//
// Runs `cargo run -p starter-ui-ir --bin emit_schema` and confirms
// it produces a byte-identical schema artifact to what is committed
// at `crates/starter-ui-ir/schema/starter-ui-ir.schema.json`. If
// they differ, `pnpm test` in this package fails — the operator
// must re-run the emitter and commit the updated artifact so the
// puck palette stays in lockstep with the Rust IR.
//
// Environments without Cargo (CI for frontend-only changes,
// pre-commit on a Rust-less laptop) skip the check with a visible
// warning rather than failing — the Rust-side CI guard
// (`crates/starter-ui-ir/tests/schema_artifact.rs`) is the
// belt-and-braces. Set `RUBIX_PUCK_DRIFT=strict` to turn the
// missing-cargo case into an error.

import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..", "..");
const SCHEMA_PATH = resolve(
  REPO_ROOT,
  "crates",
  "starter-ui-ir",
  "schema",
  "starter-ui-ir.schema.json",
);
const STRICT = process.env.RUBIX_PUCK_DRIFT === "strict";

function haveCargo() {
  try {
    execFileSync("cargo", ["--version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

function main() {
  if (!haveCargo()) {
    const msg = "[puck/schema-drift] cargo not on PATH — skipping drift check.";
    if (STRICT) {
      console.error(msg);
      process.exit(1);
    }
    console.warn(msg);
    return;
  }

  // Snapshot the committed artifact bytes so we can restore if the
  // emitter writes a different file (the check is supposed to be
  // read-only).
  const original = readFileSync(SCHEMA_PATH);

  const result = spawnSync(
    "cargo",
    ["run", "-p", "starter-ui-ir", "--bin", "emit_schema", "--quiet"],
    { cwd: REPO_ROOT, stdio: "inherit" },
  );
  if (result.status !== 0) {
    // Restore in case the emitter wrote a partial file.
    writeFileSync(SCHEMA_PATH, original);
    console.error(
      `[puck/schema-drift] emit_schema failed with exit code ${result.status}`,
    );
    process.exit(1);
  }

  const regenerated = readFileSync(SCHEMA_PATH);
  if (!original.equals(regenerated)) {
    // Restore the committed artifact so the working tree isn't
    // dirtied by a drift detection run.
    writeFileSync(SCHEMA_PATH, original);
    console.error(
      "[puck/schema-drift] starter-ui-ir.schema.json is stale.\n" +
        "  Run `cargo run -p starter-ui-ir --bin emit_schema` and commit the updated artifact.",
    );
    process.exit(1);
  }
}

main();
