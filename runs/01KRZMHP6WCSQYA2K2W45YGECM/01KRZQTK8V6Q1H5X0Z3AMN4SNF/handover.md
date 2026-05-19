## Done

- Stage 9 landed in commit `1f72e35` on branch `codeless/starter-tools-services`.
- New workspace member `crates/smoke-tests` (`starter-smoke-tests`, `publish = false`) holds one integration-test binary per SCOPE smoke check, each documenting which design rule it enforces. `cargo test -p starter-smoke-tests` passes locally (1+2+1+1+1 tests).
- Smoke 1 shells out to `scripts/check-spi-dep-baseline.sh`, which canonicalises `cargo tree -p starter-spi --edges normal --prefix none` (LC_ALL=C, `(*)`-marker strip, workspace-local path strip) and diffs against `DOCS/tools/scope/starter-spi-deps.baseline.txt`. `--update` regenerates the file in-place preserving the header. The baseline header was rewritten to document the new normalisation recipe; the baseline body was regenerated once with the canonical sort (single-time normalisation contemplated by Decision D1's revisit trigger).
- Smoke 2 builds both registries with all five provider artefacts via plain `.register(...)`. `starter-mcp::ToolRegistry` is restated locally (`starter_mcp_substitute`) so the smoke crate doesn't pull MCP transitively; the real `ToolRegistry` is exercised by the notes example.
- Smoke 3 toggles a per-test env var (`STARTER_SMOKE_3_SLACK_ENABLED`) around `.register(...)`, asserts `is_empty()` then `len() == 1`.
- Smoke 4 hand-rolls `FakeFileStore` / `FakeKeyringStore` `SecretStore` impls (avoids requiring an OS keyring or age key on CI) and builds every provider Config twice through the same `resolve(&dyn SecretStore, name) -> SecretString` helper, proving the Config bodies are byte-identical across backends.
- Smoke 5 registers Slack + Telegram services at `http://127.0.0.1:1`, waits 200 ms, calls `ServiceRegistry::shutdown`, asserts elapsed ≤ `SHUTDOWN_DEADLINE_DEFAULT` and zero `ServiceShutdownOutcome::Aborted`. Also asserts the constant is still `Duration::from_secs(5)` so a future bump trips this test.
- CI gate: new `spi-dep-baseline` job in `.github/workflows/ci.yml` runs `scripts/check-spi-dep-baseline.sh` directly so a provider leak fails the PR independently of the cargo-level test.

## Next

- Stage 9 was the final stage; nothing further in this rollout. A fresh session can pick up the next job from `DOCS/tools/scope/SCOPE.md` follow-ups (e.g. inbound Gmail, `RestartingService` adapter) when those are scoped.

## What you need to know

- Pre-existing clippy lint `clippy::module-inception` fires on `crates/starter-spi/src/service/mod.rs` (`mod service;`) under `-D warnings`. This is not Stage 9 work; the lint pre-dates this commit. Cargo `test` and plain `clippy --no-deps` for the smoke crate are clean.
- The baseline header was edited to drop the "CI strips workspace-local paths before comparing" language because the script now strips paths from both sides — the old sentence is no longer accurate.
- `starter-secrets-file` and `starter-secrets-keyring` are listed as dev-deps of the smoke crate even though smoke 4 uses in-memory stand-ins. This is intentional: keeping them on the dep list ensures a workspace build still resolves them, and leaves the door open to swap in the real backends if a future check needs to (the in-memory fakes are documented as the reason the real backends aren't used today).
- `cargo fmt --all -- --check` still reports a diff in an unrelated `examples/notes` gRPC test (the `GetRequest { id: note_id.clone() }` block-vs-inline reformat shown during fmt check). That predates Stage 9; fixing it is not in scope.
- Run order for verification on a fresh checkout: `scripts/check-spi-dep-baseline.sh` then `cargo test -p starter-smoke-tests`.

## Open questions

- (none)
