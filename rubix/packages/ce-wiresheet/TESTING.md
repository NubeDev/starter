# Testing the ce-wiresheet editor

The frontend equivalent of your C++ ctest suite is **Vitest** (the Vite-native
test runner — same toolchain as the build, TypeScript/ESM with no extra config).
Tests live next to the code as `*.test.ts` / `*.test.tsx`.

## Running

```bash
# unit tests — hermetic, no engine needed (the usual loop)
pnpm --filter @nube/ce-wiresheet test          # run once (CI / pre-commit)
pnpm --filter @nube/ce-wiresheet test:watch    # re-run on save while developing

# integration tests — drive a LIVE Control Engine (REST + binary WS)
pnpm --filter @nube/ce-wiresheet test:integration
CE_BASE=http://<ip>:<port> pnpm --filter @nube/ce-wiresheet test:integration

# every package in the workspace (what CI runs) — unit only
pnpm -r run test
```

## Integration tests (live engine)

`src/itest/*.itest.ts` exercise the **whole path** — REST mutate → engine compute
→ binary-WS stream → real `decodeBinaryFrame` — against a running CE with the
NubeIO math extension (`CE_BASE`, default `http://127.0.0.1:7878`). They **skip
themselves** when no engine is reachable, so they're safe in CI / offline.

- Each test gets a **clean slate**: a dedicated `__ce_itest__` folder is deleted +
  recreated in `beforeEach`, so tests never see each other's components and the
  rest of the engine is left untouched.
- Correctness is read off the **value stream** (REST returns stored config, not
  the live computed value). Inputs are seeded with `defaultValues` at creation —
  overrides set the OVERRIDDEN status but not the value on this engine
  (API_REQUESTS §3), so they can't drive a test.
- `dataflow.itest.ts` covers: `add` = in1+in2, `subtract` = in1−in2, a **chain of
  5 adds (each input 1) summing to 5**, and edge-delete halting propagation.
- Gotcha encoded in the helpers: the engine rejects 1-character component names.

Run the suite after any change to `src/` — if a test goes red you've lost
behaviour something pinned. Adding a feature? Add a test for it in the same PR.

## What lives where

- **`vitest.config.ts`** — picks up `src/**/*.test.{ts,tsx}`, jsdom environment.
- Tests sit beside their target: `src/lib/facet.ts` → `src/lib/facet.test.ts`.

## Current coverage (77 tests, 13 files)

Pure logic backing the editor's features, extracted into `lib/` so each is
directly testable:

- `facet.ts` — parse/serialize round-trip, **byte-exact wire-format lock**, uid
  remap on copy, exposed-port extraction, alias resolve + `parseAliasInput`.
- `wire.ts` — binary frame decode for every typeTag (bool/u32/i32/f32/f64/u64/
  i64/str), 64-bit alignment, multi-section + STATUS routing.
- `store.ts` — structural reducers: prop→component index, edge-cascade on delete.
- `routing.ts` — cross-folder edge routing: in/cross partition, exposed-port
  index (+ subscription set), and the per-edge port-vs-ghost classification.
- `grouping.ts` — Group boundary detection (+ the name-fallback ghost-bug guard).
- `layout.ts` — grid fallback + duplicate-position de-stacking.
- `search.ts` — search index build + query ranking (names, labels, aliases).
- `paste.ts` — paste planning: flatten, top-level reposition, facet uid-remap.
- `connect.ts` — "Connect to…" candidate grouping, tiering, path-scope filter.
- `movepicker.ts` — "Move into…" destinations: cycle-exclusion + tiering.
- `subscriptions.ts` — WS subscribe/unsubscribe set diff.
- `naming.ts` — `sanitizeName`, `uniqueName`.
- `rest.ts` — `RestError.debug` dump formatting.

The editor (`CeEditor.tsx`, `FunctionBlock.tsx`) and the WS client now *call*
these, so a feature's core logic is covered even though the React/WS glue around
it isn't. What remains untested is genuinely UI/IO: rendering, drag, menu
plumbing, the optimistic-append paths, and the socket lifecycle — component /
integration territory.

## What to test next (in priority order)

1. **Pure logic — the highest-value, lowest-friction layer.** No DOM, no mocks,
   deterministic. This is where most regressions actually hide:
   - `lib/facet.ts` — parse/serialize round-trips, the byte-exact wire format
     (the "did `__facets` change?" guard), uid remap on copy. → `facet.test.ts`
   - `lib/store.ts` — the structural reducers: prop→component indexing, the
     edge-cascade on `removeComponent`. → `store.test.ts`
   - `lib/wire.ts` — binary frame decode (good next target: feed known bytes,
     assert decoded values per uid).
   - Editor-derivation helpers (boundary detection for Group, exposed-port
     routing, the stacking offset) — extract them as pure functions and test the
     input→output mapping directly.

2. **Component / interaction tests** — `@testing-library/react` renders a
   component into jsdom and asserts on the output / fires events. Heavier (needs
   the REST + WS layers stubbed, and React Flow wants a sized container), so reach
   for these on genuinely UI-level behaviour, not logic that can be pulled into a
   pure function.

## Pattern for adding a test

```ts
import { describe, expect, it } from "vitest";
import { thing } from "./thing";

describe("thing", () => {
  it("does X for input Y", () => {
    expect(thing(Y)).toEqual(expectedX);
  });
});
```

A practical rule that keeps tests cheap: when a bug turns out to live in editor
glue (CeEditor.tsx), lift the offending calculation into a small exported pure
function in `lib/`, test that, and call it from the component. The Group boundary
detection and the duplicate-position stacking are both good candidates to extract
and lock down this way.
