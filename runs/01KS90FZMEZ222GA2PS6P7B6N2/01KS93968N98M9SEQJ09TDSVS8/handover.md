## Done

- Rewired `examples/flow-agent` to Postgres-only (stage 4 / slice D).
- `Cargo.toml`: dropped `starter-store-sqlite`, added `starter-store-postgres` with `{flow, agent-session}` features, switched `sqlx` feature from `sqlite` to `postgres`, switched `starter-prefs` feature from `["routes","sqlite"]` to `["routes","postgres"]`, added `starter-store-postgres` with `testing` to dev-dependencies.
- `migrations/flow_agent/0001_init.sql`: rewritten in Postgres dialect (JSONB, TIMESTAMPTZ, no SQLite-specific affinity tricks).
- `src/store.rs`: `SqlitePool → PgPool`, `?N → $N`, timestamp bindings are `DateTime<Utc>`, JSON bindings are `serde_json::Value`, `parse_dt` helper removed.
- `src/main.rs`: removed `DEFAULT_DATABASE_URL` constant; `DATABASE_URL` env var is now required with a named error message.
- `src/server.rs`: `SqliteAgentSessionStore → PgAgentSessionStore`, `SqlitePrefsStore → PgPrefsStore`, all imports updated.
- `src/migrations.rs`: `starter_store_sqlite` → `starter_store_postgres` migration sources; `MIGRATIONS` → `PG_MIGRATIONS` for prefs.
- `tests/agent_tool_bridge.rs` + `tests/insights_agent_tools.rs`: replaced `sqlite::memory:` with `starter_store_postgres::testing::with_database()`, added `#[ignore = "requires docker"]` on every test function; `insights_agent_tools.rs::make_runtime` now returns `(AiRuntime, TempDir, ContainerGuard)` to keep the container alive.
- `cargo check -p flow-agent` and `cargo check -p flow-agent --tests` both pass with zero errors.
- Committed as `stage 4 (slice D) — rewire examples/flow-agent to Postgres-only`.

## Next

- Stage 5: REVIEW gate 2 (after slice D) — verify the full change set against ADR-001 acceptance criteria.
- Stage 6/7: any remaining slices per the job description.

## What you need to know

- SQLite is NOT removed from any library crate — only the example binary was changed.
- `starter-store-postgres`'s `flow` migration ships its own `runs` table; the flow-agent's `runs` table (different schema) lives under `migrations/flow_agent/0001_init.sql` — the two don't collide because the example intentionally excludes `FLOW_MIGRATION_SOURCE` from its chain.
- `cargo run -p flow-agent` now requires `DATABASE_URL` to be set to a Postgres URL; there is no SQLite fallback.
- Tests are marked `#[ignore = "requires docker"]` and pass with `cargo test -p flow-agent -- --ignored` against a live Postgres (via testcontainers).
- The DRAIN_TIMEOUT in `agent_tool_bridge.rs` was bumped from 10 s to 30 s to accommodate the Docker startup overhead inside the ignored test path.

## Open questions

- (none)
