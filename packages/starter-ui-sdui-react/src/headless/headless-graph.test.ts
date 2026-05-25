// Guard: importing `@nube/starter-ui-sdui-react/headless` MUST stay
// free of any `@nube/starter-ui-kit` transitive import so React Native
// builds can consume the headless subpath without bundling the web
// kit. We walk the static `import ... from "..."` graph rooted at
// `src/headless/index.ts` and assert no source touches starter-ui-kit.

import { readFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "index.ts");

const IMPORT_RE =
  /(?:^|\n)\s*(?:import|export)(?:\s+type)?\s+(?:[^"';]*?from\s+)?["']([^"']+)["']/g;

function readSource(file: string): string | null {
  for (const candidate of [file, `${file}.ts`, `${file}.tsx`, `${file}/index.ts`, `${file}/index.tsx`]) {
    if (existsSync(candidate)) return readFileSync(candidate, "utf8");
  }
  return null;
}

function walk(entry: string, seen = new Set<string>()): string[] {
  const found: string[] = [];
  const stack = [entry];
  while (stack.length > 0) {
    const file = stack.pop()!;
    if (seen.has(file)) continue;
    seen.add(file);
    const src = readSource(file);
    if (src === null) continue;
    found.push(file);
    for (const match of src.matchAll(IMPORT_RE)) {
      const spec = match[1];
      if (spec === undefined) continue;
      if (!spec.startsWith(".")) {
        // record bare specifiers via the `found` companion below
        found.push(`<bare>${spec}`);
        continue;
      }
      const next = resolve(dirname(file), spec.replace(/\.js$/, ""));
      stack.push(next);
    }
  }
  return found;
}

describe("headless module graph", () => {
  it("does not pull in @nube/starter-ui-kit", () => {
    const visited = walk(ROOT);
    const kitImporters = visited.filter(
      (entry) => entry === "<bare>@nube/starter-ui-kit",
    );
    expect(kitImporters).toEqual([]);
  });
});
