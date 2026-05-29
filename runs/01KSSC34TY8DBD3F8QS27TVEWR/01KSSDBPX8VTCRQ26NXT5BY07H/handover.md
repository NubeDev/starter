## Done

- starter-ext-spi: new src/issue.rs (ExtensionIssue/IssueCode/Severity/IssueSource), exported from lib.rs barrel; IssueCode serialises to stable ext.issue.* codes
- starter-ext-supervisor: pure derive_issues() in new issues.rs + SupervisorHandle::issues(); derives from event ring + capability-violation counter
- starter-ext-host: ExtensionRecord::issues() → single Fatal ManifestInvalid for Failed records
- starter-ext-server: GET /extensions/{id}/issues (new issues.rs) merging record+handle issues, sorted by at desc, ?severity= / ?since= filters; route wired in router.rs
- Unit tests for all derivations pass; clippy -D warnings + fmt --check green for the four touched crates; committed as 89bee6d

## Next

- Stage 2: pid() accessor, ProcessStats, GET /extensions/{id}/process, /proc sampler on the health tick (starter-ext-spi process.rs + supervisor + server)

## What you need to know

- A full-workspace `cargo build`/`clippy` fails with a pre-existing duplicate-symbol error (`__STARTER_EXT_FLAVOUR_MARKER`) in starter-ext-sdk — caused by the example crates' mutually-exclusive builtin/wasm/process SDK features unifying in one build. It is unrelated to this stage; verify per-crate with `-p starter-ext-spi -p starter-ext-supervisor -p starter-ext-host -p starter-ext-server`.
- Had to fix two pre-existing clippy 1.94 lints to clear the -D warnings bar: io_other_error in server/lifecycle.rs and dead test scaffolding (marker_factory) in server/capabilities.rs.
- The supervisor does NOT own worker states today (no WorkerState type exists), so derive_issues never emits IssueCode::WorkerFailed yet — the variant exists in the contract for a future workers-adapter wiring. ExtensionIssue.at uses SystemTime::now() for record-level and counter-aggregate issues (no event clock available there).
- There is an unrelated pre-existing git stash (user-WIP-aside-during-stage5-commit) — left untouched.

## Open questions

- (none)
