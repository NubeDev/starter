#!/usr/bin/env node
// scripts/check-no-alias-imports.mjs
//
// Guards every package in `packages/*` against `@/...` path-alias imports
// in its source.
//
// Why: when a package is consumed via `workspace:*`, the downstream app's
// bundler doesn't know about the package's `tsconfig.json` `paths` mapping.
// An import like `import { cn } from "@/lib/utils"` resolves fine while
// developing inside that package, then explodes the first time a consumer
// imports the file (we hit this with `@nube/starter-ui-kit/components/sheet`).
//
// Always use relative imports inside a package, and import other workspace
// packages by name (`@nube/starter-ui-core`).
//
// Tools like `npx shadcn add` re-introduce `@/...` imports automatically;
// run `pnpm fix:alias-imports` (or rewrite by hand) after using them.

import { readdirSync, statSync, readFileSync, existsSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..");
const packagesDir = join(repoRoot, "packages");

const ALIAS_RE = /from\s+["']@\//;

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) {
      if (name === "node_modules" || name === "dist" || name === ".turbo") continue;
      walk(p, out);
    } else if (/\.(ts|tsx|mts|cts)$/.test(name)) {
      out.push(p);
    }
  }
  return out;
}

let hits = 0;
for (const pkgName of readdirSync(packagesDir)) {
  const srcDir = join(packagesDir, pkgName, "src");
  if (!existsSync(srcDir)) continue;
  for (const file of walk(srcDir)) {
    const src = readFileSync(file, "utf8");
    const lines = src.split("\n");
    for (let i = 0; i < lines.length; i += 1) {
      if (ALIAS_RE.test(lines[i])) {
        hits += 1;
        process.stdout.write(`${relative(repoRoot, file)}:${i + 1}: ${lines[i].trim()}\n`);
      }
    }
  }
}

if (hits > 0) {
  process.stderr.write(
    `\nFound ${hits} alias import(s). Use relative paths inside a package — ` +
    `aliases break the workspace contract for downstream consumers.\n`,
  );
  process.exit(1);
}
process.stdout.write("No '@/' alias imports found in packages/*/src.\n");
