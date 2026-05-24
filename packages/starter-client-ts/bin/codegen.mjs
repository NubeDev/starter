#!/usr/bin/env node
// Generalised OpenAPI -> TS types codegen wrapper around
// openapi-typescript. Accepts --input <path> and --output <path>;
// defaults preserve the previous inline `pnpm codegen` behaviour for
// @nube/starter-client-ts so other packages (e.g. rubix-client-ts) can
// reuse it by passing different flags.

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkgRoot = resolve(here, "..");

const DEFAULT_INPUT = resolve(pkgRoot, "../../openapi.json");
const DEFAULT_OUTPUT = resolve(pkgRoot, "./src/generated/index.ts");

function parseArgs(argv) {
  let input = DEFAULT_INPUT;
  let output = DEFAULT_OUTPUT;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--input" || a === "-i") {
      input = resolve(process.cwd(), argv[++i]);
    } else if (a === "--output" || a === "-o") {
      output = resolve(process.cwd(), argv[++i]);
    } else if (a === "--help" || a === "-h") {
      process.stdout.write(
        "Usage: codegen.mjs [--input <openapi.json>] [--output <index.ts>]\n",
      );
      process.exit(0);
    } else {
      process.stderr.write(`unknown arg: ${a}\n`);
      process.exit(2);
    }
  }
  return { input, output };
}

const { input, output } = parseArgs(process.argv.slice(2));

const result = spawnSync(
  "npx",
  ["--no-install", "openapi-typescript", input, "-o", output],
  { stdio: "inherit" },
);

process.exit(result.status ?? 1);
