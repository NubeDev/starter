#!/usr/bin/env node
// scripts/check-public-api.mjs
//
// Snapshots and verifies the public API surface of every workspace package
// in `packages/*`. The "public API" is what `src/index.ts` (or `.tsx`)
// re-exports, expanded one level through `export * from "./foo"` and
// `export { … } from "./foo"` re-exports inside the same package.
//
// Why this exists:
//   The `exports` field in package.json is the contract with consumers.
//   It is easy to accidentally add or remove a re-export in a barrel file
//   without anyone noticing — until a downstream app breaks at build time.
//   This script writes a deterministic snapshot to
//   `packages/<pkg>/api.snapshot.txt` and fails CI when it diverges.
//
// Usage:
//   node scripts/check-public-api.mjs          # verify (CI mode)
//   node scripts/check-public-api.mjs --update # rewrite snapshots
//
// Limitations (by design — keep this script dependency-free):
//   • Regex-based parser, not a real TS AST. Comments and strings that
//     contain "export …" can produce false positives. Don't put fake
//     export statements in source comments.
//   • Only flattens one level into sibling files via `export *` /
//     `export { … } from "./x"`. Deeper graphs are summarised as
//     `* from <path>` lines so changes there still show up in the diff.
//   • Default exports are recorded as `default`.

import { readFileSync, writeFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, dirname, resolve, relative } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..");
const packagesDir = join(repoRoot, "packages");
const UPDATE = process.argv.includes("--update");

// ---------------------------------------------------------------------------
// Parse a single source file. Returns { entries: Set<string>, reexports: [path] }
// where each entry is a normalized "kind name" record and `reexports` is a
// list of relative module specifiers to recurse into for `export *`.
// ---------------------------------------------------------------------------
function stripCommentsAndStrings(src) {
  // Replace line/block comments with whitespace of the same length so line
  // numbers stay aligned and regex offsets are unaffected. We intentionally
  // leave string literals alone — we need the contents of `from "./foo"` to
  // resolve re-exports. (False positives from `export …` text inside string
  // literals are extremely rare in our codebase; the regex still requires
  // a `from "..."` clause or a declaration keyword.)
  let out = "";
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    const n = src[i + 1];
    if (c === "/" && n === "/") {
      const end = src.indexOf("\n", i);
      const stop = end === -1 ? src.length : end;
      out += " ".repeat(stop - i);
      i = stop;
    } else if (c === "/" && n === "*") {
      const end = src.indexOf("*/", i + 2);
      const stop = end === -1 ? src.length : end + 2;
      out += src.slice(i, stop).replace(/[^\n]/g, " ");
      i = stop;
    } else {
      out += c;
      i += 1;
    }
  }
  return out;
}

function parseFile(absPath) {
  const raw = readFileSync(absPath, "utf8");
  const src = stripCommentsAndStrings(raw);
  const entries = new Set();
  const reexports = []; // [{ specifier, kind: 'all' | 'named' | 'namespace', names?, ns?, isType? }]

  // export * from "./foo"
  // export * as Ns from "./foo"
  for (const m of src.matchAll(/\bexport\s+(type\s+)?\*\s+(?:as\s+([A-Za-z_$][\w$]*)\s+)?from\s+["']([^"']+)["']/g)) {
    const [, isType, ns, spec] = m;
    if (ns) {
      entries.add(`${isType ? "type " : ""}namespace ${ns}`);
    } else {
      reexports.push({ specifier: spec, kind: "all", isType: !!isType });
    }
  }

  // export { a, b as c, type d } from "./foo"
  // export { a, b as c, type d }
  for (const m of src.matchAll(/\bexport\s+(type\s+)?\{([^}]+)\}(?:\s+from\s+["']([^"']+)["'])?/g)) {
    const [, outerType, body, spec] = m;
    const names = body.split(",").map((s) => s.trim()).filter(Boolean);
    for (const name of names) {
      const typed = /^type\s+/.test(name) || !!outerType;
      const clean = name.replace(/^type\s+/, "");
      const parts = clean.split(/\s+as\s+/);
      const exposed = parts[parts.length - 1].trim();
      if (!exposed || exposed === "default") continue;
      entries.add(`${typed ? "type " : ""}${exposed}`);
    }
    if (spec) {
      // Recorded too, so that diffs catch when the source file moves
      // even if exposed names are unchanged.
      reexports.push({ specifier: spec, kind: "named", names, isType: !!outerType });
    }
  }

  // export const|let|var|function|async function|class|interface|enum|type|abstract class Name
  const declRe = /\bexport\s+(type\s+)?(?:declare\s+)?(?:async\s+)?(const|let|var|function\*?|class|abstract\s+class|interface|enum|type)\s+([A-Za-z_$][\w$]*)/g;
  for (const m of src.matchAll(declRe)) {
    const [, outerType, kind, name] = m;
    const typed = !!outerType || kind === "type" || kind === "interface";
    if (name === "default") continue;
    entries.add(`${typed ? "type " : ""}${name}`);
  }

  // export default
  if (/\bexport\s+default\b/.test(src)) {
    entries.add("default");
  }

  return { entries, reexports };
}

// Resolve a relative module specifier (./foo, ./foo.js, ./foo/index) to an
// actual on-disk .ts(x) file inside the same package.
function resolveSpecifier(fromFile, spec) {
  if (!spec.startsWith(".")) return null; // external import; ignore
  // NodeNext TS source uses .js suffixes that point at .ts files on disk.
  const stripped = spec.replace(/\.(?:js|mjs|cjs|jsx)$/, "");
  const base = resolve(dirname(fromFile), stripped);
  const candidates = [
    base,
    `${base}.ts`,
    `${base}.tsx`,
    `${base}.mts`,
    join(base, "index.ts"),
    join(base, "index.tsx"),
  ];
  for (const c of candidates) {
    if (existsSync(c) && statSync(c).isFile()) return c;
  }
  return null;
}

// Flatten the index file: collect its own entries and recursively pull in
// entries from `export * from "./x"` re-exports (one level of recursion is
// usually enough; we go up to a depth limit to be safe against cycles).
function flattenPackageApi(indexPath, pkgRoot) {
  const visited = new Set();
  const collected = new Set();
  const trace = []; // unresolved or external `* from` lines, shown in the snapshot

  function visit(absPath, depth) {
    if (visited.has(absPath) || depth > 6) return;
    visited.add(absPath);
    const { entries, reexports } = parseFile(absPath);
    for (const e of entries) collected.add(e);
    for (const r of reexports) {
      if (r.kind !== "all") continue;
      const resolved = resolveSpecifier(absPath, r.specifier);
      if (resolved && resolved.startsWith(pkgRoot)) {
        visit(resolved, depth + 1);
      } else {
        const display = resolved
          ? relative(pkgRoot, resolved)
          : r.specifier;
        trace.push(`${r.isType ? "type " : ""}* from ${display}`);
      }
    }
  }

  visit(indexPath, 0);
  return { collected, trace };
}

// ---------------------------------------------------------------------------
// Per-package: build the snapshot text and compare/write.
// ---------------------------------------------------------------------------
function buildSnapshot(pkgDir) {
  const candidates = [
    join(pkgDir, "src", "index.ts"),
    join(pkgDir, "src", "index.tsx"),
  ];
  const indexPath = candidates.find((p) => existsSync(p));
  if (!indexPath) return null;

  const { collected, trace } = flattenPackageApi(indexPath, pkgDir);
  const all = [...collected, ...trace].sort();
  const header = [
    "# Public API snapshot — DO NOT edit by hand.",
    "# Regenerate with: pnpm api:update",
    `# Source: ${relative(pkgDir, indexPath)}`,
    "",
  ];
  return header.join("\n") + all.join("\n") + "\n";
}

function listPackages() {
  return readdirSync(packagesDir)
    .map((name) => join(packagesDir, name))
    .filter((p) => statSync(p).isDirectory() && existsSync(join(p, "package.json")));
}

let hadDiff = false;
let checked = 0;
for (const pkgDir of listPackages()) {
  const snap = buildSnapshot(pkgDir);
  if (snap === null) continue;
  checked += 1;
  const snapPath = join(pkgDir, "api.snapshot.txt");
  const existing = existsSync(snapPath) ? readFileSync(snapPath, "utf8") : null;
  const rel = relative(repoRoot, snapPath);
  if (existing === snap) {
    process.stdout.write(`ok    ${rel}\n`);
    continue;
  }
  if (UPDATE) {
    writeFileSync(snapPath, snap);
    process.stdout.write(`wrote ${rel}\n`);
  } else {
    hadDiff = true;
    process.stdout.write(`DIFF  ${rel}\n`);
    if (existing === null) {
      process.stdout.write("       (no existing snapshot — run `pnpm api:update`)\n");
    }
  }
}

if (hadDiff && !UPDATE) {
  process.stderr.write(
    `\nPublic API surface drifted in one or more packages.\n` +
    `If the change was intentional:  pnpm api:update  and commit the diff.\n`,
  );
  process.exit(1);
}

process.stdout.write(`\n${UPDATE ? "Wrote" : "Checked"} ${checked} package snapshot(s).\n`);
