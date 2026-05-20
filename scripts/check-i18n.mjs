#!/usr/bin/env node
// `pnpm -w run check:i18n` — workspace catalog parity gate.
//
// Walks every `i18n/<lang>.json` under the workspace (extensions
// catalogs + platform catalogs) and asserts:
//
//   (a) every non-`en` catalog has the same key set as `en`,
//   (b) every key resolves to a non-empty string,
//   (c) ICU placeholder names match across languages (a missing
//       `{name}` in es.json that exists in en.json fails the build).
//
// Production teams that inherit this framework cannot ship a
// partially-translated locale by accident
// (`examples/notes/user-pref.md` § Stage 5).
//
// Exit codes:
//   0 — every catalog checks
//   1 — at least one violation; details printed to stderr
//
// Implementation note: zero external deps so the CI image only needs
// node 18+. `fs.glob` lands in node 22 — we walk manually.

import { readdir, readFile, stat } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

const ROOT = resolve(process.cwd());

/** Directories the walker should skip. Standard ignore set; keep the
 *  list short — adding entries here is the only way a catalog goes
 *  unchecked, which is the failure mode we want to make loud. */
const SKIP = new Set([
  "node_modules",
  "target",
  ".git",
  "dist",
  "build",
  ".pnpm",
  ".cache",
  ".turbo",
  "coverage",
]);

/** Recursively find every directory named `i18n/`. Returns absolute
 *  paths. */
async function findI18nDirs(dir) {
  const out = [];
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const ent of entries) {
    if (!ent.isDirectory()) continue;
    if (SKIP.has(ent.name)) continue;
    const full = join(dir, ent.name);
    if (ent.name === "i18n") {
      out.push(full);
      continue;
    }
    out.push(...(await findI18nDirs(full)));
  }
  return out;
}

/** Return `{key: placeholders[]}` for one catalog file. Empty values
 *  surface as a violation later; placeholder extraction tolerates
 *  the small ICU subset our catalogs use (`{name}`, `{count, plural,
 *  …}`). */
function extractPlaceholders(value) {
  const out = new Set();
  // Match identifiers in ICU placeholder positions only: `{ident}` or
  // `{ident,…}` (optionally with whitespace before the comma). Plain
  // text inside plural arms — e.g. `{No unread notes}` — must not be
  // treated as a placeholder, which is why we require the lookahead.
  const rx = /\{(\w+)\s*(?=[,}])/g;
  let m;
  while ((m = rx.exec(value)) !== null) out.add(m[1]);
  return out;
}

async function loadCatalog(file) {
  const text = await readFile(file, "utf8");
  let json;
  try {
    json = JSON.parse(text);
  } catch (err) {
    throw new Error(`${relative(ROOT, file)}: invalid JSON — ${err.message}`);
  }
  if (typeof json !== "object" || json === null || Array.isArray(json)) {
    throw new Error(`${relative(ROOT, file)}: top-level must be a JSON object`);
  }
  return json;
}

function diffSets(reference, candidate) {
  const missing = [];
  const extra = [];
  for (const k of reference) if (!candidate.has(k)) missing.push(k);
  for (const k of candidate) if (!reference.has(k)) extra.push(k);
  return { missing, extra };
}

const violations = [];

/** Check one i18n directory: requires an `en.json` and validates each
 *  sibling against it. */
async function checkI18nDir(dir) {
  let files;
  try {
    files = (await readdir(dir)).filter((f) => f.endsWith(".json"));
  } catch {
    return;
  }
  if (files.length === 0) return;
  if (!files.includes("en.json")) {
    violations.push(
      `${relative(ROOT, dir)}: missing en.json — every catalog dir needs the en floor (D-NP.6).`,
    );
    return;
  }
  const ref = await loadCatalog(join(dir, "en.json"));
  const refKeys = new Set(Object.keys(ref));
  for (const k of refKeys) {
    if (typeof ref[k] !== "string" || ref[k].length === 0) {
      violations.push(
        `${relative(ROOT, dir)}/en.json: key "${k}" must be a non-empty string`,
      );
    }
  }
  for (const f of files) {
    if (f === "en.json") continue;
    const lang = f.slice(0, -".json".length);
    let cat;
    try {
      cat = await loadCatalog(join(dir, f));
    } catch (err) {
      violations.push(err.message);
      continue;
    }
    const candKeys = new Set(Object.keys(cat));
    const { missing, extra } = diffSets(refKeys, candKeys);
    for (const k of missing) {
      violations.push(`${relative(ROOT, dir)}/${f}: missing key "${k}" (present in en)`);
    }
    for (const k of extra) {
      violations.push(
        `${relative(ROOT, dir)}/${f}: extra key "${k}" — absent in en. Add to en first.`,
      );
    }
    for (const k of refKeys) {
      const v = cat[k];
      if (typeof v !== "string" || v.length === 0) {
        if (candKeys.has(k)) {
          violations.push(
            `${relative(ROOT, dir)}/${f}: key "${k}" must be a non-empty string`,
          );
        }
        continue;
      }
      const refPh = extractPlaceholders(ref[k]);
      const candPh = extractPlaceholders(v);
      const phDiff = diffSets(refPh, candPh);
      for (const p of phDiff.missing) {
        violations.push(
          `${relative(ROOT, dir)}/${f}: key "${k}" missing placeholder {${p}} — present in en (${lang})`,
        );
      }
      for (const p of phDiff.extra) {
        violations.push(
          `${relative(ROOT, dir)}/${f}: key "${k}" introduces placeholder {${p}} — not in en`,
        );
      }
    }
  }
}

const dirs = await findI18nDirs(ROOT);
if (dirs.length === 0) {
  console.warn("[check:i18n] no i18n/ directories found — nothing to check.");
  process.exit(0);
}

for (const d of dirs) {
  // eslint-disable-next-line no-await-in-loop
  await checkI18nDir(d);
}

if (violations.length === 0) {
  console.log(`[check:i18n] ${dirs.length} catalog dir(s) OK.`);
  process.exit(0);
}

console.error(`[check:i18n] ${violations.length} violation(s):`);
for (const v of violations) console.error(`  - ${v}`);
process.exit(1);
