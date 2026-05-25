## Done

- created `packages/starter-theme-tokens/` (pure data, no peerDeps): `palette.ts`, `density.ts`, `radius.ts`, `type.ts`, `motion.ts`, `role.ts`, `index.ts` barrel, `package.json`, `tsconfig.json`, `README.md`
- `palette.ts` carries `platformLightPalette` / `platformDarkPalette` (oklch values lifted verbatim from `globals.css` + `defaults.ts`), `NAMED_PALETTES` (10 presets lifted from `presets.ts` with tweakcn attribution), `NON_COLOR_KEYS`, and `CSS_EMISSION_ORDER_LIGHT/DARK`
- `packages/starter-ui-kit/scripts/generate-css.ts` reads tokens and renders `globals.css`; output is byte-identical to the live file (verified via `pnpm --filter @nube/starter-ui-kit verify:css`)
- regression fixture `packages/starter-ui-kit/scripts/__fixtures__/globals.expected.css` pins the current bytes; `verify:css` fails on drift; wired as kit `prebuild`
- `starter-ui-core/src/theme-editor/defaults.ts` and `presets.ts` refactored to re-export from `@nube/starter-theme-tokens`; hardcoded palette tables deleted; `starter-ui-core` gains the `workspace:*` dep
- starter-ui-kit gains devDeps `@nube/starter-theme-tokens`, `@types/node`, `tsx`
- committed as `stage 4: phase 2 (slice 2) — @nube/starter-theme-tokens new package`

## Next

- (none) — stage 5 picks up in a fresh session

## What you need to know

- `pnpm-workspace.yaml` was NOT edited: it already globs `packages/*` so the new package is auto-included (verified via `pnpm ls -r`). The stage spec wording overestimated what was needed.
- Stage spec says "ESM + CJS dual build" — the workspace convention is **source-only TS** (`main`/`types` → `./src/index.ts`, `build` = `tsc --noEmit`); every existing `packages/*` follows that. I followed the workspace convention. If/when a true dual build is wanted (e.g., for npm publish), it should be a workspace-wide decision.
- Stage spec says "HSL triplets" — the actual source data in `globals.css` and `defaults.ts` is oklch, and the byte-identical acceptance bar forced oklch. Palette stores raw oklch strings.
- `palette.ts` ThemeStyleKey union is structurally identical to the one in `starter-ui-core/src/theme-editor/types.ts`; assignment between them is type-safe by structural typing — kept duplicated for now so `starter-theme-tokens` stays standalone.
- `density.ts` / `motion.ts` had no existing source data; populated with minimal sensible defaults aligned with Tailwind v4 + Material/HIG, documented as adoptable later. `role.ts` introduces the semantic role → token-key mapping the RN kit needs.
- `pnpm --filter @nube/starter-ui-core test`: 2 pre-existing failures in `src/auth/auth.test.tsx` reproduce on master with `git stash` — NOT introduced here. Theme-editor tests (33) pass.
- `pnpm -w build` is clean; rubix/frontend and all examples build through the refactored core/kit unchanged.

## Open questions

- Should `starter-ui-core/types.ts`'s `ThemeStyleKey` be replaced with a re-export from `@nube/starter-theme-tokens` to eliminate the duplicated union? Deferred — out of scope for stage 4 (additive only).
- Web visual review is the formal acceptance bar for the REVIEW gate after this stage; the byte-identical CSS + `verify:css` give strong mechanical assurance but a human eyeball on rubix/frontend dark/light + each preset is still owed before stage 5.
