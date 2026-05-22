# Scope — flow-agent-postgres-only

The authoritative design lives at
[/home/user/code/rust/starter/DOCS/storage/ADR-001-flow-agent-postgres-only.md](/home/user/code/rust/starter/DOCS/storage/ADR-001-flow-agent-postgres-only.md).
This brief is the trimmed per-job scope. Where this disagrees with
the ADR, **the ADR wins** — fix this file rather than diverge.

## Goal

Make `examples/flow-agent` Postgres-only on the `starter` repo via
the `codeless/flow-agent-postgres-only` branch. After this job:

1. `examples/flow-agent` builds, boots, and tests against Postgres
   only. SQLite is removed from its dependency graph entirely; no
   transitive `sqlx-sqlite` dep is reachable from the `flow-agent`
   crate.
2. `starter-store-postgres` gains a `flow` feature exporting
   `PgFlowStore`, `PgRunStore`, `PgSessionStore`,
   `PgAgentSessionStore`, `FLOW_MIGRATION_SOURCE`, and
   `AGENT_SESSION_MIGRATION_SOURCE` — the Postgres twin of the
   existing `starter-store-sqlite::flow` module, satisfying the
   same traits in `starter-flow-spi`.
3. `starter-prefs` gains a `postgres` feature parallel to its
   existing `sqlite` feature, exporting `PgPrefsStore`. The
   library keeps both backends behind features; only the example
   switches.
4. `flow-agent` boot fails fast with a clear error if
   `DATABASE_URL` is unset. No hidden default, no fallback to a
   SQLite file path.
5. `starter-store-sqlite` and its `flow/` module are **unchanged**.
   Other consumers (`examples/notes`, future embedded apps) keep
   their SQLite path.

## In scope (three slices)

- **Slice A (stage 1) — `starter-store-postgres::flow` (small
  files first):**
  - New `flow` feature on `starter-store-postgres`'s `Cargo.toml`,
    enabling `sqlx/chrono`, `sqlx/json`, and whatever else the
    audit decides is missing transitively (`uuid` likely).
  - New `crates/starter-store-postgres/src/flow/` module behind
    `#[cfg(feature = "flow")]`. Files: `mod.rs`, `schema.rs`,
    `flow_store.rs` (PgFlowStore), `run_store.rs` (PgRunStore),
    `session_store.rs` (PgSessionStore).
  - New migrations tree
    `crates/starter-store-postgres/migrations/flow/` — Postgres
    dialect rewrite of the SQLite twin.
  - Unit tests parallel to the SQLite twin, running against the
    existing `testing::with_database()` testcontainers helper.

- **Slice B (stage 2) — port `agent_session_store.rs` (the 716-LOC
  one) + reload-time invariants:**
  - Port `agent_session_store.rs` from `starter-store-sqlite::flow`
    to `starter-store-postgres::flow`. Watch JSONB path operators,
    `TIMESTAMPTZ` arithmetic, and `ON CONFLICT` conflict-target
    explicitness.
  - New migrations tree
    `crates/starter-store-postgres/migrations/agent_sessions/` —
    Postgres dialect rewrite of the SQLite twin.
  - Extend or reuse the existing `agent-session` feature on
    `starter-store-postgres` so flow-agent can pull `flow +
    agent-session` together.
  - Per-store integration tests mirroring the SQLite suite,
    asserting the same trait-level behaviour (no behavioural
    regression).

- **Slice C (stage 4) — `starter-prefs` Postgres backend:**
  - New `postgres` feature on `starter-prefs`'s `Cargo.toml`
    parallel to the existing `sqlite` feature, mutually
    composable (consumers pick one or both).
  - New `PgPrefsStore` implementing the same trait surface as
    `SqlitePrefsStore`. Same public methods, same error
    semantics.
  - New migrations tree
    `crates/starter-prefs/migrations/postgres/` — Postgres
    dialect rewrite of the SQLite migrations.
  - Tests covering the Postgres backend; the existing SQLite
    tests stay green untouched.

- **Slice D (stage 6) — `flow-agent` rewire:**
  - `examples/flow-agent/Cargo.toml`: drop
    `starter-store-sqlite`, drop `sqlx`'s `sqlite` feature; add
    `starter-store-postgres = { workspace = true, features =
    ["flow", "agent-session"] }`, add `sqlx`'s `postgres`
    feature. `starter-prefs` features `["routes", "postgres"]`.
  - `examples/flow-agent/src/main.rs`: remove
    `DEFAULT_DATABASE_URL`; require `DATABASE_URL`; swap
    `starter_store_sqlite::pool` →
    `starter_store_postgres::pool`. Error message on missing
    env var names `DATABASE_URL` explicitly.
  - `examples/flow-agent/src/store.rs`: `SqlitePool` → `PgPool`
    in all three structs (`FlowStore`, `AgentStore`,
    `RunStore`); rewrite every query placeholder `?N` → `$N`;
    swap timestamp bindings to `chrono::DateTime<Utc>`; swap
    JSON column bindings to `serde_json::Value`.
  - `examples/flow-agent/src/server.rs`:
    `SqliteAgentSessionStore` → `PgAgentSessionStore`,
    `SqlitePrefsStore` → `PgPrefsStore`.
  - `examples/flow-agent/src/migrations.rs`: switch migration
    sources to the new Postgres modules.
  - Rewrite
    `examples/flow-agent/migrations/flow_agent/0001_init.sql`
    in Postgres dialect (`JSONB`, `TIMESTAMPTZ`, `$N`, `NOW()`).
  - Switch `examples/flow-agent/tests/agent_tool_bridge.rs` and
    `examples/flow-agent/tests/insights_agent_tools.rs` from
    `sqlite::memory:` to
    `starter_store_postgres::testing::with_database`. Mark
    `#[ignore]` per the project's convention for Postgres tests.

- **Slice E (stage 8) — deletion + final sweep:**
  - Confirm `cargo tree -p flow-agent -e features | grep
    sqlite` returns nothing.
  - Confirm `cargo metadata --format-version 1` shows no
    transitive `sqlx-sqlite` reachable from the `flow-agent`
    crate node.
  - Remove any orphan files in
    `examples/flow-agent/migrations/flow_agent/` that the
    Postgres rewrite left behind.
  - Final `cargo test --workspace` + `cargo clippy --workspace
    --all-features -- -D warnings` + `cargo fmt --check` green.
  - End-to-end smoke: `cargo run -p flow-agent` against a local
    `docker run` Postgres, create a flow via REST, fire it, see
    SSE land in a browser. Smoke transcript captured in the
    final handover.

## Out of scope

- **Removing `starter-store-sqlite` from the workspace.** It
  stays. Other consumers depend on it; this job is about
  flow-agent's choice, not a workspace-level deletion.
- **Removing the `flow/` module from `starter-store-sqlite`.** It
  stays so the SQLite backend retains feature parity for any
  future flow consumer that wants SQLite.
- **Removing the `sqlite` feature from `starter-prefs`.** The
  crate keeps both backends behind features; only the example
  switches.
- **A SQLite → Postgres data migration script.** The ADR is
  explicit: fresh DB is the answer. No production data is at
  stake in this example.
- **A SPI-level conformance test suite that runs the same
  trait-level tests against both backends.** That is library-level
  work and warrants its own job. The current job only ports;
  proving non-drift across backends is a separate concern.
- **Introducing a sqlx-generic abstraction over `Database`.** ADR-001
  is explicit that the starter pattern is *two parallel impls
  behind one trait*. Generalising over `Database` defeats the
  compile-time query benefits of each backend and is not in
  scope.
- **CI infra changes beyond what flow-agent's `#[ignore]` Postgres
  tests already need.** If the existing
  `starter-store-postgres --features testing -- --ignored` CI job
  can cover flow-agent's tests by extension, do so; if it
  cannot, document a local-only recipe in the handover and surface
  the CI gap as a follow-up, do not expand CI infra in this job.
- **Any change to PR #22 surface** (`starter-flow-spi`,
  `Contributes.nodes`, `DynamicNodeKindRegistry`, supervisor
  wiring). The port is orthogonal to PR #22 and must not touch
  its files.
- **`starter-changelog-sqlite`, `starter-clipboard-sqlite`, or
  any other `*-sqlite` crate.** Not consumed by flow-agent;
  out of scope.

## Constraints

- **ADR-1 — trait surface is fixed.** `starter-flow-spi`'s
  `FlowStore`, `RunStore`, `SessionStore`, `AgentSessionStore`
  traits are stable. The Postgres impls must satisfy them with no
  public-API changes. If a trait method has SQLite-specific
  semantics that are hard to replicate, surface — do not silently
  change the trait.
- **ADR-2 — no `sqlx::query!` compile-time macros.** The SQLite
  source uses runtime `sqlx::query(...)`; the Postgres impls stay
  runtime too. The workspace must not need a live `DATABASE_URL`
  at compile time.
- **ADR-3 — dialect translation rules** (apply consistently
  across every ported file):
  - `?N` → `$N` (positional parameters)
  - `TEXT` timestamps → `TIMESTAMPTZ` with
    `chrono::DateTime<Utc>` bindings
  - JSON-as-`TEXT` → `JSONB` with `serde_json::Value` bindings
  - `CURRENT_TIMESTAMP` → `NOW()`
  - `INTEGER` boolean columns → `BOOLEAN`
  - `ON CONFLICT (…) DO UPDATE` — Postgres requires explicit
    conflict-target columns
  - `INSERT … RETURNING` works on both, no change needed
- **ADR-4 — boot fails fast on missing `DATABASE_URL`.** No
  hidden default. No fallback to a SQLite file path. Error
  message names the env var verbatim.
- **ADR-5 — Pi caveat documented.**
  `examples/flow-agent/README.md` already carries the SSD
  warning; do not regress it.
- **ADR-6 — `starter-store-sqlite::flow` stays.** Do not delete
  it, do not modify it. If something on the SQLite side looks
  wrong while reading it for the port, file it as a follow-up
  note in `handover.md` and move on. This job's scope is the
  port, not a SQLite refactor.
- **ADR-7 — `starter-prefs` keeps both backends.** The
  `sqlite` feature on `starter-prefs` stays compilable and
  tested. Only the example dropping it; the crate is dual-backend
  by design.
- **MSRV / lint gates**: `cargo test --workspace`,
  `cargo clippy --workspace --all-features -- -D warnings`,
  `cargo fmt --check` green at every stage boundary.
- **No `--no-verify` or `--force`.** If a pre-commit hook
  fails, fix the cause.
- **R-trio applies** (CLAUDE.md): every stage ends with
  `checks`, `docs`, `git` per the closing trio block in
  `WORKFLOW.md`.

## Deliverables (what "done" looks like)

1. `codeless/flow-agent-postgres-only` branch with one commit
   per stage (five stages + two REVIEW handovers = seven
   commits), pushed via mani.
2. `cargo test --workspace` green at every stage boundary.
3. `cargo clippy --workspace --all-features -- -D warnings`
   green at every stage boundary.
4. `cargo fmt --check` green at every stage boundary.
5. **Slice A acceptance:** `cargo test -p starter-store-postgres
   --features "flow testing" -- --ignored` green; the new
   `flow_store.rs`, `run_store.rs`, `session_store.rs`
   integration tests pass against an ephemeral Postgres.
6. **Slice B acceptance:** `cargo test -p starter-store-postgres
   --features "flow agent-session testing" -- --ignored` green,
   including a port of the SQLite suite for
   `agent_session_store.rs`. The handover lists every query
   whose **semantics** changed (not just syntax) with a
   one-line justification.
7. **Slice C acceptance:** `cargo test -p starter-prefs
   --features postgres` green; the SQLite tests still pass
   under `--features sqlite`.
8. **Slice D acceptance:** `cargo run -p flow-agent` against a
   local `docker run` Postgres boots cleanly; REST `/api/flows`
   CRUD works; `cargo test -p flow-agent` green (Postgres tests
   `#[ignore]` and runnable locally with `--ignored`).
9. **Slice E acceptance:** `cargo tree -p flow-agent -e
   features | grep -i sqlite` returns nothing reachable from
   `flow-agent`. `cargo metadata` confirms no transitive
   `sqlx-sqlite` from the `flow-agent` crate. End-to-end smoke
   transcript (boot + create flow + fire flow + SSE) in the
   final handover.

## Open questions — RESOLVED (2026-05-23, before start)

The ADR is unusually well-resolved — the backend choice is
explicit, the dialect translation rules are spelled out, the
out-of-scope carve-outs are tight. Three job-specific
resolutions follow.

### Q1 — One job or split prefs into its own?

**Answer: One job, five stages, two REVIEW gates. Slice C
(`starter-prefs`) stays bundled because if it isn't ported, the
example needs a side-SQLite database and the goal isn't
achieved.**

The prefs port is small (key-value-ish store, far smaller than
`agent_session_store.rs`) and the example's rewire (Slice D)
depends on it. Splitting it into a separate job means landing a
broken intermediate state on master where the example pulls both
`sqlx-postgres` and `sqlx-sqlite`. Bundle it.

**Decision.**
1. One job, five stages, two REVIEW gates.
2. Cap at **30000¢ / 4h**, same as the other queued starter
   jobs. Slice A is small (~15% of cap). Slice B is the
   load-bearing remainder (~40%). Slice C is small (~10%).
   Slice D is mechanical but tedious (~25%). Slice E is cheap
   (~10%).
3. Two REVIEW gates:
   - After **Slice B** (stage 3 REVIEW), before the prefs port.
     Gate question: do the Postgres store impls satisfy the SPI
     traits with no semantic regressions?
   - After **Slice D** (stage 7 REVIEW), before deletion. Gate
     question: does `flow-agent` build, boot, and pass its
     `#[ignore]` test suite end-to-end against a real Postgres?
4. If the budget is blown before slice E, halt at the second
   REVIEW gate. Do not silently land a partial deletion (a
   half-deleted SQLite story is worse than no deletion).

### Q2 — `agent_session_store.rs` translation risk

**Answer: the file is the highest-risk port. The job mitigates
by (a) doing it as its own stage after the smaller files are
green and (b) requiring the handover at the REVIEW gate to
enumerate every query whose semantics changed, not just every
query that changed syntactically.**

The 716 LOC includes JSON path manipulations, datetime
arithmetic, and (per the audit) at least one `ON CONFLICT`
upsert chain. SQLite's `json_extract`/`json_set` map onto
Postgres `->`/`->>`/`jsonb_set` with different null-handling
semantics. SQLite's `datetime('now', '+5 minutes')` maps onto
`NOW() + interval '5 minutes'` cleanly, but anything subtler
(epoch math, julian-day tricks) needs explicit reasoning.

Stage 2's handover lists every **semantics-changed** query (not
just syntax) in a section titled `semantics-changed queries`.
The REVIEW gate after stage 3 reads that section. If any query's
semantics drifted unintentionally, the gate fails — fix and
re-request, do not advance.

### Q3 — Postgres in CI for `flow-agent`

**Answer: extend the existing `--ignored` testcontainers job
pattern to cover `flow-agent`. Do not introduce a new CI infra
shape.**

`starter-store-postgres --features testing` already runs in CI
via the `--ignored` job pattern (testcontainers ships Docker on
GitHub-hosted runners). Slice D's tests use the same
`testing::with_database` helper, so adding a `cargo test -p
flow-agent --features pg-tests -- --ignored` invocation to the
same job is a one-line CI tweak.

If stage 6's audit reveals that `flow-agent`'s tests need a
different testcontainers shape than `starter-store-postgres`'s
(e.g. needing both Postgres *and* mosquitto for PR #22's MQTT
demo), surface — that's a CI-infra concern above this job's
authority. Default expectation is that the existing job pattern
covers it.

## References

- ADR (authoritative):
  [/home/user/code/rust/starter/DOCS/storage/ADR-001-flow-agent-postgres-only.md](/home/user/code/rust/starter/DOCS/storage/ADR-001-flow-agent-postgres-only.md)
- Source of the SQLite `flow/` module to port:
  `/home/user/code/rust/starter/crates/starter-store-sqlite/src/flow/`
- Target crate for the Postgres twin:
  `/home/user/code/rust/starter/crates/starter-store-postgres/`
- Trait definitions (the contract both backends satisfy):
  `/home/user/code/rust/starter/crates/starter-flow-spi/`
- `starter-prefs` ground truth (Slice C):
  `/home/user/code/rust/starter/crates/starter-prefs/`
- Example to rewire (Slice D):
  `/home/user/code/rust/starter/examples/flow-agent/`
- Existing testcontainers helper (Slice A/B/D tests):
  `/home/user/code/rust/starter/crates/starter-store-postgres/src/testing/with_database.rs`
- Updated docs (already on master as of this job's start):
  - `examples/flow-agent/README.md` (Postgres-only)
  - `examples/flow-agent/SCOPE.md` (Postgres-only)
  - `crates/starter-store-postgres/README.md` (lists the new
    `flow` feature)
