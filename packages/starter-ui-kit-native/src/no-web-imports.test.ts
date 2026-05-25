// Structural guard: the native kit MUST NOT depend on the web kit.
// If a primitive ever sprouts `import "@nube/starter-ui-kit"` we want
// the test suite to fail loudly, not the first time someone tries to
// bundle this package into an Expo build.

import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const here = path.dirname(fileURLToPath(import.meta.url));

function* walk(dir: string): Generator<string> {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "__mocks__" || entry.name === "types") continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) yield* walk(full);
    else if (
      /\.(ts|tsx)$/.test(entry.name) &&
      !/\.test\.tsx?$/.test(entry.name) &&
      entry.name !== "test-utils.tsx"
    )
      yield full;
  }
}

describe("starter-ui-kit-native bounds", () => {
  it("does not import @nube/starter-ui-kit anywhere under src/", () => {
    const bad: string[] = [];
    for (const file of walk(here)) {
      const source = fs.readFileSync(file, "utf8");
      if (/@nube\/starter-ui-kit(?!-)/.test(source)) {
        bad.push(path.relative(here, file));
      }
    }
    expect(bad).toEqual([]);
  });
});
