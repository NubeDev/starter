/**
 * Phase 8 falsification — R3 structural domain-leak tripwire.
 *
 * The SCOPE.md § R3 contract says the React renderer never knows
 * what the domain is: it must never name a domain concept — only
 * structural primitives. This test scans every `.ts` / `.tsx`
 * source under `packages/starter-sdui-react/src` (excluding test
 * files and this scanner) for a denylist of domain-specific
 * vocabulary that, if it appeared, would mean a Phase 8 fixture
 * leaked into the renderer.
 *
 * The denylist is small on purpose. R3's real enforcement is the
 * per-crate allowlist in `crates/starter-ui-ir/words.txt` (see
 * SCOPE.md § R3); this file is a defence-in-depth tripwire
 * scoped to the three Phase 8 falsification fixtures (CRUD device
 * list, PR review card, scope board).
 */
import { describe, it, expect } from "vitest";
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));

const FORBIDDEN = [
  // Fixture-1 vocabulary (CRUD device list, BACnet-era Rubix shape).
  "bacnet",
  "modbus",
  "device.list",
  "alarm.active",
  // Fixture-2 vocabulary (PR review card).
  "pull_request",
  "pull request",
  "review.approve",
  "request_changes",
  // Fixture-3 vocabulary (scope board).
  "scope.plan",
  "scope_plan",
  "in_progress_count",
  "open_count",
  // Other domains starter happens to ship — none of them belong
  // in the renderer crate. (`gmail` / `slack` / `telegram` are
  // service-package names; the SDUI renderer must not know them.)
  "gmail",
  "telegram",
  "github.com/",
];

const SKIP_FILES = new Set(["r3-no-domain-leak.test.ts"]);

async function walk(dir: string): Promise<string[]> {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const out: string[] = [];
  for (const e of entries) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) {
      out.push(...(await walk(full)));
    } else if (
      /\.tsx?$/.test(e.name) &&
      !e.name.endsWith(".test.ts") &&
      !e.name.endsWith(".test.tsx") &&
      !SKIP_FILES.has(e.name)
    ) {
      out.push(full);
    }
  }
  return out;
}

describe("R3 — renderer crate has no domain leak (Phase 8 tripwire)", () => {
  it("no source file mentions a fixture-domain term", async () => {
    const files = await walk(HERE);
    expect(files.length).toBeGreaterThan(5);
    const leaks: string[] = [];
    for (const f of files) {
      const text = (await fs.readFile(f, "utf-8")).toLowerCase();
      for (const term of FORBIDDEN) {
        if (text.includes(term.toLowerCase())) {
          leaks.push(`${path.relative(HERE, f)} :: ${term}`);
        }
      }
    }
    expect(leaks, `R3 violation — renderer references domain terms:\n${leaks.join("\n")}`).toEqual([]);
  });
});
