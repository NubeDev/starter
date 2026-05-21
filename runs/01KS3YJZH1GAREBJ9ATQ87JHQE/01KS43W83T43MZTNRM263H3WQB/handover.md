## Done

- Added `scripts/check-sdui-domain-leak.sh` (R3 allowlist gate) + per-crate `words.txt` files for `starter-ui-ir` (558 tokens), `starter-ui-bindings` (234), `starter-sdui-react` (681). Allowlist is the contract; `--update` flag for the PR that legitimately adds vocabulary.
- Added `scripts/check-sdui-ir-deps.sh` (R1 transitive dep-graph denylist on starter-ui-ir: axum, axum-core, reqwest, hyper, tokio, tokio-util, tower, tower-http, h2, http-body). Currently passes.
- Added `scripts/check-sdui-routes-isolation.sh` — smoke #10. Asserts `starter-server`'s `--all-features` transitive normal closure contains no `starter-sdui-routes` / `starter-ui-*` crate (M6 / D4 consumer-opt-in).
- Added `scripts/check-sdui-size-budget.sh` — Renderer.tsx ≤ 800 (currently 68), components total ≤ 4000 (currently 1932).
- Added `packages/starter-sdui-react/src/capability.test.tsx` — smoke #3. Pins R2 at `checkIrVersion` boundary + an SSR fixture that proves the dispatcher is never called for a V+1 tree. Avoids importing `SduiRenderPage` directly to dodge the Vite-only `@/...` aliases that vitest's node env can't resolve.
- Wired all five gates into `.github/workflows/ci.yml` under a new `sdui-gates` job. All four scripts run green locally; `pnpm --filter @nube/starter-sdui-react test` reports 14/14.

## Next

- (none) — Stage 12 is the final stage of `DOCS/frontend/sdui/SCOPE.md`. ai-builder Phase 1 was unblocked at Stage 2 (Phase-1 schema artifact); ai-builder Phase 2 is unblocked now (Phase 5 routes landed in Stage 6).

## What you need to know

- The R3 allowlist was bootstrapped from current source via the script's `--update` mode and committed verbatim. Future PRs that add a new identifier in any of the three crates must run `scripts/check-sdui-domain-leak.sh --update` and justify the new tokens in the PR description per SCOPE § R3. This is intentional: the allowlist is what makes future drift visible.
- The tokeniser strips `//` and `/* … */` comments best-effort, then matches `[A-Za-z_][A-Za-z0-9_]*` of length ≥ 4, lowercased. Tokens shorter than 4 chars are ignored (too noisy: `id`, `ok`, `to`, `for`). `tests/`, `fixtures/`, `*.test.ts`, `*.test.tsx`, and `starter-ui-ir/src/bin/` are excluded as the SCOPE specifies.
- `check-sdui-routes-isolation.sh` uses `--all-features` deliberately — M6 says a Cargo feature on starter-server that pulls SDUI still breaks the opt-out claim. The gate currently denies the four SDUI crates by name; extend the list if a sibling SDUI crate ships later.
- The capability smoke avoids the renderer chain on purpose. The integration that `SduiPage` / `SduiRenderPage` themselves call `checkIrVersion` is already covered by the static read in `Custom.test.tsx`'s harness and by hand-inspection in source; promoting to a full `@testing-library/react` mount would require jsdom + path-alias plumbing for one assertion.
- `cargo test -p starter-ui-ir -p starter-ui-bindings -p starter-ui-builder -p starter-sdui-routes` and `pnpm --filter @nube/starter-sdui-react test` both green at HEAD.

## Open questions

- (none)
