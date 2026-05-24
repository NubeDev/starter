## Done

- Confirmed via `grep -rn 'CREATE DATABASE rubix\|USE rubix' crates/starter-store-clickhouse rubix/` that no pre-existing bootstrap relied on the `rubix` DB — chose Option (a) per SCOPE Open Question 2 (route tables to `rubix`).
- Added `RUBIX_CH_DATABASE = "rubix"` constant + `rubix_ch_config(url)` helper in `rubix/crates/rubix-agent/src/boot/clickhouse.rs`; re-exported from `boot/mod.rs`.
- `apply_ch_migrations` now `CREATE DATABASE IF NOT EXISTS rubix` via a `default`-bound bootstrap client, then runs the `MigrationRunner` against a `rubix`-bound client.
- Threaded the helper through `main.rs` and `bin/rubix_admin/mcp/serve.rs` so every `ChClient` rubix-agent builds is pinned to the `rubix` DB.
- Added `rubix/crates/rubix-agent/tests/ch_history_db_routing_test.rs` (feature-gated + `#[ignore]`): spins a CH testcontainer, runs the migration, inserts a row, asserts it lands in `rubix.system_disk_history` and no `default.system_disk_history` exists. Added `starter-store-clickhouse` with `testing` dev-feature.
- Rewrote `rubix/docs/design/warehouse/README.md` §"Database routing" present-tense.
- `cargo test -p rubix-agent` green; `./rubix/scripts/lint-doc-refs.sh` clean.
- Committed as `stage 5: fix(rubix-agent) route warehouse tables to the rubix database` (bc42a9f) and pushed to `codeless/rubix-smoke-followups`.

## Next

- Stage 6: B10 — fix stale `docker volume rm` names (`docker_rubix_postgres_data` / `docker_rubix_clickhouse_data`) in `rubix/docs/sessions/2026-05-24-handover-codeless-orchestration.md` §2 and scan §§5–8 for other stale paths.

## What you need to know

- The B9 integration test is `#[ignore]`'d (requires Docker) — run with `cargo test -p rubix-agent --test ch_history_db_routing_test -- --ignored`.
- The pre-existing `SUPER_ADMIN_TENANT` dead-code warning in `starter-auth-users` is still present — that is N4, scheduled for a later stage; intentionally untouched here.
- `apply_ch_migrations` still uses the same `MigrationRunner` API; consumers outside rubix-agent are unaffected (no `starter-store-clickhouse` API change).

## Open questions

- (none)
