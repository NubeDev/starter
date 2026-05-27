# 2026-05-27 — SDUI package consolidation + React Native removal

> **Superseded by [`2026-05-27-sdui-consolidation-final.md`](./2026-05-27-sdui-consolidation-final.md).**
> Stream B (the SDUI half-port) is obsolete: tracing the real consumers
> showed `starter-ui-ai-builder` had none, so both it and
> `starter-sdui-react` were deleted outright instead of being ported.
> Stream A (React Native removal) is unaffected.

Two cleanup streams that surfaced while smoke-testing the dashboard
API. Both are mechanical refactors with limited blast radius if done
in the order below.

## Stream A: Drop React Native, keep Flutter

Decision: rubix's mobile target is **Flutter** (`rubix/flutter/`),
not React Native. Everything under the "native" / `rubix/mobile/`
banner exists only because the `mobile-chassis` job laid groundwork
for an RN port that is no longer planned.

### Packages and directories to delete

| Path | Why | Consumers |
|---|---|---|
| `packages/starter-ui-sdui-native` | RN port of SDUI renderer | only `rubix/mobile` |
| `packages/starter-ui-kit-native` | RN port of UI kit | only the two packages below |
| `packages/starter-ui-dashboard-native` | RN dashboard shell | only `rubix/mobile` |
| `packages/starter-theme-tokens` | Shared design tokens authored for RN parity | check before delete — may also be imported by the kept web kit |
| `rubix/mobile/` | The RN app itself | the only thing that consumes the four packages above |
| `.codeless/jobs/mobile-chassis/` | Job spec that drove the RN work | finished work; not a runtime artifact |

### Things to keep

- `rubix/flutter/` — the real mobile target.
- `packages/starter-ui-sdui-react/src/headless/` — the platform-neutral
  headless API split out by mobile-chassis Phase 0. It's also useful
  for SSR / non-DOM consumers; the split itself is not RN-specific
  and stays.

### Suggested order

1. **Verify `starter-theme-tokens` consumers.** `grep -rln
   "@nube/starter-theme-tokens" --include=package.json` outside
   `node_modules`. If only native packages use it, delete it. If
   `starter-ui-kit` (web) imports it, keep it.
2. **Delete `rubix/mobile/`.** Single-app delete; nothing imports the
   app.
3. **Delete the three `*-native` packages** and **`starter-theme-tokens`**
   (if step 1 cleared it).
4. **Delete `.codeless/jobs/mobile-chassis/`.**
5. `pnpm install` to update the lockfile and drop dangling workspace
   entries.
6. `pnpm -w build` and `pnpm -w test` to confirm no straggler imports.
7. Grep the docs tree for `react-native`, `mobile/`, `kit-native`,
   `sdui-native`, `dashboard-native`, and `mobile-chassis`; remove or
   redirect to the Flutter docs as appropriate (
   `rubix/docs/scope/mobile/`, `rubix/docs/design/mobile/`,
   `rubix/docs/design/flutter/`).

### Out of scope

- `packages/starter-ui-sdui-react/src/headless/` — keep.
- Anything under `rubix/flutter/` — keep.

## Stream B: Consolidate to one SDUI react package

There are two packages today. Both were created in commit `f6e4b51`
(Phase 4 — @nube/starter-sdui-react port, 2026-05-21). No design doc
explains why two; the second appears to be an in-progress port that
was never finished and never wired into the app.

### Inventory

| Package | Runtime consumers | Renderer count | Status |
|---|---|---|---|
| `packages/starter-ui-sdui-react` | `rubix/frontend`, `rubix/mobile` (deleted in Stream A), `starter-ui-sdui-puck`, `starter-ui-sdui-native` (deleted in Stream A) | ~16 (`page`, `row`, `col`, `grid`, `kpi`, `kpi_grid`, `chart`, `table`, `form`, `tabs`, `select`, `slider`, `toggle`, `date_range`, `divider`, `custom`, `repeat`) | **Keep** — this is what serves dashboards today |
| `packages/starter-sdui-react` | `starter-ui-ai-builder` only (type imports) | ~19 (`page`, `row`, `col`, `grid`, `tabs`, `stack`, `card`, `text`, `heading`, `badge`, `kpi`, `kpi_grid`, `button`, `link`, `table`, `form`, `field`, `select`, `toggle`) | **Delete** after migrating ai-builder |

### Migration plan for `starter-ui-ai-builder`

ai-builder only imports **types** (`UiComponent`, `UiComponentTree`)
and one helper (`replaceAt`) from `starter-sdui-react`. Six files
touched, all in
`/home/user/code/rust/starter/packages/starter-ui-ai-builder/src/`:

- `lib/utils.ts`
- `components/ai-builder-canvas.tsx`
- `components/ai-builder.tsx`
- `hooks/use-builder.ts`
- `adapters/fixture.ts`
- `types/index.ts`

Migration steps:

1. **Confirm parity.** `UiComponent` / `UiComponentTree` exist in
   `@nube/starter-ui-ir` (the IR package that the kept SDUI react
   re-exports). ai-builder should import from
   `@nube/starter-ui-ir` for types and from
   `@nube/starter-ui-sdui-react/headless` for `replaceAt` and other
   tree helpers.
2. **Swap imports** (six sed-able edits):
   - `from "@nube/starter-sdui-react"` for types →
     `from "@nube/starter-ui-ir"`.
   - `from "@nube/starter-sdui-react"` for `replaceAt` /
     tree helpers → `from "@nube/starter-ui-sdui-react/headless"`.
3. **Update `packages/starter-ui-ai-builder/package.json`:** remove
   `"@nube/starter-sdui-react"` dependency; add (or keep)
   `"@nube/starter-ui-ir"` and `"@nube/starter-ui-sdui-react"`.
4. `pnpm install`, `pnpm --filter @nube/starter-ui-ai-builder build`,
   `pnpm --filter @nube/starter-ui-ai-builder test`.

### Widget types unique to `starter-sdui-react` we lose by deleting

Renderers that exist in the rich package but **not** the lean one:
`stack`, `card`, `text`, `heading`, `badge`, `button`, `link`,
`field`. ai-builder does not appear to instantiate any of these
(it imports types only, not the renderers). Confirm with one final
grep before deleting; if any are referenced, port them into
`starter-ui-sdui-react` first (each is ~30–60 lines).

### Suggested order

1. Run the grep above to confirm ai-builder doesn't render any of the
   "rich-only" widget types.
2. Swap ai-builder imports.
3. Delete `packages/starter-sdui-react/`.
4. `pnpm install` + `pnpm -w build` + `pnpm -w test`.
5. Grep the docs for `@nube/starter-sdui-react` and remove or
   redirect.

## Why this matters beyond cleanup

- **Confusion.** Both packages export `SduiPage`, `UiComponent`,
  `registerRenderer`. The two are not interchangeable — wiring a
  rubix consumer to the wrong one would render nothing and fail
  silently (same failure mode as the Tailwind `@source` bug
  documented in [`design/sdui/dashboard-api-usage.md`](../design/sdui/dashboard-api-usage.md)).
- **Drift risk.** Two parallel renderers will diverge: someone fixes a
  bug in one and not the other.
- **CI cost.** Every PR runs builds and tests on the unused tree.

## Verification after both streams land

- `find packages/ -maxdepth 2 -name 'package.json' | xargs grep -l '"name"'` returns no `*-native` or `starter-sdui-react`.
- `pnpm -w build` green.
- `pnpm -w test` green.
- Loading `http://127.0.0.1:5173/dashboards/claude-hello` still
  renders the page with the row/col layout intact.
- Loading the AI builder UI (whichever route mounts it) still
  renders.

## Open questions

- Is `starter-ui-dashboard-native` truly unused by anything outside
  `rubix/mobile`? Worth a `grep -rn` before delete.
- Does any internal doc or ADR reference the deleted packages by
  name? `docs/design/mobile/` is the most likely place.
- Should `starter-theme-tokens` survive as a web-only design-tokens
  package, or fold into `starter-ui-kit`? Independent of Stream A;
  decide after the consumer audit.
