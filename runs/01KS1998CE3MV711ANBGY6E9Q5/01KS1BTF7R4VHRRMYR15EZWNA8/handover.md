## Done

- Added `crates/starter-server/tests/canonical_logs.rs`: capturing `tracing_subscriber::fmt` layer writing into a process-global `Mutex<Vec<u8>>`, drives Accept-Units middleware in preferred + canonical modes via a handler that logs canonical SI fields and converts only for the response, then asserts no captured log line contains `"°F"`, `" psi"`, `" mph"`, `" lb"`. Companion canary test verifies the capturing writer records text.
- Added `tracing-subscriber` to `starter-server` dev-dependencies.
- Created top-level `CONTRIBUTING.md` documenting the canonical-SI-logs rule with do/don't examples, audit command, and PR-review checklist.
- `cargo test -p starter-server --test canonical_logs` → 2 passed.
- Committed as "stage 10 — Phase 2 canonical-only-logs audit…".

## Next

- Stage 11 (next session) picks up per SCOPE Phase 2 → Phase 3 transition.

## What you need to know

- Forbidden substrings use a leading space for `psi`/`mph`/`lb` to avoid false positives on words like `display`/`compliance`; `°F` is unique enough on its own. This matches how those units appear after a numeric literal in real log lines.
- The capturing subscriber is installed via `OnceLock` + `try_init` so it survives multiple tests in the same binary; if another global default is already set, the call is a no-op rather than a panic.
- The handler in the audit currently exercises Temperature/Pressure/Speed/Length/Mass conversion paths. As new starter crates emit unit-bearing logs, extend the request battery in this file so the audit covers them too — this is called out in CONTRIBUTING.md.

## Open questions

- (none)
