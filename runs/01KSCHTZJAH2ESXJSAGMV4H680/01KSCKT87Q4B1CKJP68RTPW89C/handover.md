## Done

- Implemented `AnalyticsReportTool` in `rubix/crates/rubix-tools/src/analytics/report.rs` with html/csv/json rendering via starter-export, BlobStore persistence, presigned URL, `ReversibleTool` + sidecar `AnalyticsReportReversible` (revert = `BlobStore::delete`), and the pdf → `rubix.analytics.report.format_unsupported` branch.
- Promoted `run_query` in `analytics/query.rs` to `pub(crate)` so the two verbs share one CH bind path.
- Fleshed out `rubix-spi::dto::analytics::report` (request/response DTOs, `ReportFormat`, `DESCRIPTOR`, `REQUIRED_PERMISSION`).
- Added three MessageKeys (`rendered`, `empty`, `format_unsupported`) to both `catalogues/en.json` and `catalogues/es.json`.
- Added deps: `starter-export` (html/csv/json features) + `bytes` to rubix-tools; dev-deps `starter-blob-fs`, `tempfile`, `futures`.
- Integration test `tests/analytics_report_test.rs` (Docker-gated, `#[ignore]`): seeds `system_disk_history` in testcontainers CH, points the verb at a tempdir-backed FsBlobStore, asserts the rendered HTML contains the disk_history_weekly headers and the seeded peak value 73; second test asserts pdf returns the format_unsupported MessageKey.
- 4 unit tests in `analytics::report::tests` (html/csv/json/empty) pass without Docker.
- Committed as `ea07623`: "stage 9: phase C.2 — analytics.report verb — feat(rubix-tools) analytics.report + starter-export wiring".

## Next

- Stage 10 per WORKFLOW.md picks up the next phase (likely wiring the WeeklyReportStub replacement / scheduled flow that calls analytics.report).

## What you need to know

- The `AnalyticsReportTool::new(client, store)` constructor takes `Arc<dyn BlobStore>` — production wires FsBlobStore, tests can swap any backend. Default presign TTL is 15 min; override with `.with_presign_ttl(...)`.
- Inputs require `template` + `format`; `queries` defaults to `[]` (returns an empty report).
- The Reversible inverse uses the locator from the changelog snapshot to mint a synthetic BlobRef (placeholder etag/size); FsBlobStore's delete reads `opaque_locator` only, so this is safe — other backends may need richer snapshot data.
- `apply_forward` (redo) intentionally returns `Invalid` — re-running mints a new blob with a different locator.
- `cargo build --workspace` fails on unrelated AWS SDK rustc-version requirements; `cargo build -p rubix-tools -p rubix-spi` is clean.

## Open questions

- (none)
