## Done

- Added `crates/starter-flow/tests/r3_no_policy_match_arms.rs` — the executable R3 grep-contract test. Dep-free line-oriented tokeniser strips `//`, `///`, `//!`, and nested `/* … */` comments; preserves string literals so `"<kw>" =>` and `== "<kw>"` patterns are still inspectable. Scans for three violation shapes per keyword: identifier match-arm pattern (`safe_state =>`), string match-arm pattern (`"safe_state" =>`), and string-equality compare (`x == "safe_state"` / `"safe_state" == x`). Header doc comment documents why `safe_state` (the WritableOutput trait method per R12) needs no explicit allow-list entry — it never appears as `safe_state =>` in source.
- Self-tests cover: identifier match arm, string match arm, string equality, ignore in line/doc/block comments, ignore as function call, ignore in non-arm string literals.
- `cargo test -p starter-flow --test r3_no_policy_match_arms`: 9 passed. `cargo fmt -p starter-flow` clean. `cargo clippy -p starter-flow --tests -- -D warnings` clean.
- Committed as `stage 6 — R3 grep-contract test` (bdf7e80) on branch `codeless/starter-flow-engine-finish`.

## Next

- Stage 7 of the Phase 2 catch-up job (per the SCOPE — likely final review / SCOPE-doc Phase 2 tick / changelog). A fresh session picks it up.

## What you need to know

- The tokeniser intentionally diverges from the literal "word-boundary search over all stripped source" reading of the user prompt — that interpretation would flag every `tokio::time::timeout(...)` call and the `triggers` field on `Topology`, contradicting the prompt's "expected match count is zero" and the SCOPE D1g definition ("hits inside a `match` expression's arms"). I followed SCOPE D1g (DOCS/flow/scope/SCOPE.md:840–858), which is explicit and gives count=0 today. The header doc comment in the test makes the call-site/match-arm distinction explicit so future readers don't loosen the matcher by accident.
- No `regex` dep was added — the test stays dep-free as SCOPE D1g requires. Byte-level scan over the comment-stripped source.
- No engine source was modified; the existing `src/` tree already satisfies R3 (zero match arms on the seven policy slot names).

## Open questions

- (none)
