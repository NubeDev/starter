## Done

- Verified workspace: `cargo build --workspace --all-features` green; `cargo clippy --workspace --all-targets --all-features -- -D warnings` green.
- Verified Phase 2 dep-tree gates: `cargo tree -p starter-flow|starter-flow-nodes --edges normal` show zero `adk-rust` matches; `cargo tree -p starter-flow-spi` diffs clean against `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` (after worktree-path normalisation); none of the four flow crates path-dep on `starter-mcp` / `starter-server` / `starter-cli`.
- Landed those gates as a real automated integration test: `crates/starter-flow/tests/workspace_dep_tree_gates.rs` (5 tests, all green) so future regressions break CI immediately rather than relying on a manual shell-grep step.
- Re-confirmed stage-5 SCOPE smokes (`smoke_one_write_chokepoint`, `smoke_engine_is_reader_of_policies`) and the stage-6 R3 grep-contract test (`r3_no_policy_match_arms`) pass green.
- Appended SCOPE Decision **D1h** recording the dep-tree-gate test placement + the SPI-baseline revisit trigger.
- Committed as `stage 7 — workspace verify + Phase 2 dep-tree gates as an automated test`.

## Next

- (none) — Phase 2 of the flow SCOPE is now honestly green. A fresh job should pick up Phase 3 (persistence + surface wrappers: `starter-flow-surfaces` wiring into `starter-mcp` / `starter-server` / `starter-cli`).

## What you need to know

- `cargo test --workspace --tests` is not clean *for unrelated reasons*: `crates/starter-grpc/tests/tools_service.rs` imports `starter_grpc::testing`, which is gated behind the `testing` feature that the test does not enable. This is pre-existing breakage from the upstream merge (commit `e6fd120 added example ext and grpc server`), not from any flow change. Targeted `cargo test -p starter-flow -p starter-flow-nodes -p starter-flow-spi -p starter-flow-surfaces --all-features` is fully green.
- `cargo fmt --all -- --check` reports drift across `starter-grpc/build.rs`, `examples/notes/src/server.rs`, and a number of files under `starter-extensions/crates/starter-ext-cli/` — also pre-existing upstream drift, no flow files involved. I deliberately did not reformat unrelated crates from inside this catch-up job.
- The new gate test shells out to `cargo` from inside `cargo test`. Re-entrancy is fine because `cargo tree` doesn't take the build lock, but if a future test runner wants strict hermeticity this is the seam to watch.
- The SPI baseline normaliser collapses any line containing `/worktrees/job-<id>/…` to `<WORKTREE>/…`; if the project ever moves out of `worktrees/job-…/` paths the regex anchor in `normalise_worktree_paths` will need updating (and the baseline file regenerated).

## Open questions

- Should the pre-existing fmt + `starter-grpc` test-feature drift be cleaned up in a follow-up housekeeping job, or absorbed into the Phase 3 wiring job? Not a blocker for Phase 2 exit either way.
