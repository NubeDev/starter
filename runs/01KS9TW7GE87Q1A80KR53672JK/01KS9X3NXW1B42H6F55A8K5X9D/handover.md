## Done

- Created `rubix/agent/crates/data-postgres` (empty crate; prod-dep on `starter-store-postgres`, dev-dep with `testing` feature).
- Created `rubix/agent/crates/data-clickhouse` (empty crate; prod-dep on `starter-store-clickhouse`, dev-dep with `testing` feature).
- Added integration smoke per crate that boots a testcontainer via the starter seam and runs `SELECT 1`; both `#[ignore]`'d.
- Added both crates to root `Cargo.toml` members + `[workspace.dependencies]`.
- Updated `rubix/mani.yaml` build/test/lint task lists.
- Wrote `rubix/docs/testing/SETUP.md` documenting the Docker prerequisite.
- Verified `cargo test -p rubix-data-postgres -- --ignored` green and `cargo test -p rubix-data-clickhouse -- --ignored` green against live Docker. `mani run lint --all` green (R1 budget OK).
- Committed as `ab3ebb8` on `codeless/rubix-phase-0`.

## Next

- Stage 3 of 5 (next session): the eight load-bearing design docs (OVERVIEW, EVERYTHING-AS-NODE, NODE-AUTHORING, KIND-MANIFEST, AUTH, MIGRATIONS, TESTS, VERSIONING) per Phase 0 SCOPE.

## What you need to know

- Postgres pool from starter is a wrapper — queries need `pool.sqlx()` rather than `&pool`.
- ClickHouse smoke uses `client.inner().query(..).fetch_one()` with a `#[derive(clickhouse::Row, Deserialize)]` row struct; the `clickhouse` crate is listed in dev-deps with `default-features = false, features = ["lz4"]` to keep TLS surface minimal.
- testcontainers + Docker pulls images on first run; the ClickHouse smoke takes ~30s, Postgres ~2s.
- Starter convention for testcontainer-touching tests is `#[ignore = "requires docker"]`; default `cargo test` skips them.

## Open questions

- (none)
