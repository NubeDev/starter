# SDUI consolidation — progress log

## 2026-05-27T00:00 — Phase 1 audit
- Re-ran the two grep audits from the plan; results match exactly (ai-builder + starter-sdui-react form a closed subgraph; no external consumers).
- Confirmed `rubix/frontend/src/routes/chat.tsx` imports only `@tanstack/react-router`, react, `@nube/rubix-client-react`, and local `@/` paths — no SDUI react package imports.
- Confirmed pnpm-workspace.yaml uses glob `packages/*` only (no explicit refs). Root package.json has no explicit refs.
- No tsconfig*.json contains aliases naming either package.
- ai-builder `src/index.ts` only re-exports its own subdirectories; no leakage from a third package.
- outcome: ok — plan still holds, no red flags.
- next: Step 2 (git rm both packages).

## 2026-05-27T00:05 — Steps 2 & 3 — delete + pnpm install
- `git rm -r packages/starter-sdui-react packages/starter-ui-ai-builder` — clean.
- `pnpm install` — lockfile updated, "Already up to date", no errors. Peer-dep warnings present but pre-existing and unrelated.
- outcome: ok
- next: Step 4 (pnpm -w build, pnpm -w test).

## 2026-05-27T00:10 — Step 4 — pnpm -w build
- Ran `pnpm -w build`.
- Build fails in `@nube/starter-ui-core`:
  ```
  src/preferences/SettingsPage.test.tsx(77,75): error TS2503: Cannot find namespace 'JSX'.
  ```
- Verified pre-existing: stashed the deletion + lockfile change, re-ran build on clean master — same failure reproduces. **Not caused by this cleanup.**
- outcome: failed — pre-existing build break on master in an unrelated package.
- next: user authorized continuing despite pre-existing red on master. Verified the failure reproduces on clean master and is in `starter-ui-core` (unrelated package). Proceeding with doc cleanup + commit on master.

## 2026-05-27T00:25 — Step 4 — pnpm -w test
- Same pre-existing condition: `starter-ui-core/auth.test.tsx` has 2 failing tests on clean master (404 from a missing fake server) — unrelated.
- Targeted tests on the two SDUI packages directly: `starter-ui-sdui-react` 56/56 ✅, `starter-ui-sdui-puck` 22/22 ✅. Both build clean too.
- outcome: ok (scoped). Pre-existing ui-core red is out of scope.
- next: Step 5 (docs).

## 2026-05-27T00:35 — Step 5 — doc + script cleanup
- Archived `DOCS/frontend/ai-builder/SCOPE.md` → `DOCS/frontend/_archive/ai-builder-SCOPE.md`.
- Deleted `DOCS/frontend/sdui/DIVERGENCE.md`.
- Renamed all `@nube/starter-sdui-react` / `packages/starter-sdui-react` refs in `DOCS/frontend/sdui/SCOPE.md` to the `-ui-` variant via sed.
- Updated issue #11 in `rubix/docs/design/sdui/dashboard-api-usage.md` to "resolved 2026-05-27".
- Added "Superseded by" banner to `rubix/docs/sessions/sdui/2026-05-27-sdui-package-consolidation.md`.
- Removed dead-package rows from mobile scope docs: `rubix/docs/design/flutter/NON-GOALS.md`, `rubix/docs/scope/mobile/{NON-GOALS,REUSE,APP-SHELL}.md`.
- Updated `rubix/mobile/eslint.config.cjs` no-restricted-imports list (dropped the two deleted package names).
- Updated `packages/README.md` table + dependency diagram.
- Updated `HOW-TO-CODE.md` and `rubix/HOW-TO-CODE.md` to point to `starter-ui-sdui-react`.
- Fixed CI: removed the deleted-package entry from `scripts/check-sdui-domain-leak.sh`, deleted `scripts/check-sdui-size-budget.sh` (paths it gated no longer exist), and dropped its step from `.github/workflows/ci.yml`.
- Note: `scripts/check-sdui-domain-leak.sh` still fails on master (unrelated drift in starter-ui-ir / starter-ui-bindings words.txt). Verified pre-existing by stashing — out of scope.
- outcome: ok
- next: Step 7 (commit).

