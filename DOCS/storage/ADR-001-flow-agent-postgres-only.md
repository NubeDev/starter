# ADR-001 — `flow-agent` is Postgres-only

- **Status:** Accepted
- **Date:** 2026-05-23
- **Scope:** [`examples/flow-agent`](../../examples/flow-agent) only.
  Does **not** affect the starter store crates.

## Context

The starter workspace ships two parallel SQL backends as sibling
library crates:

- [`crates/starter-store-sqlite`](../../crates/starter-store-sqlite)
- [`crates/starter-store-postgres`](../../crates/starter-store-postgres)

Both implement the same trait surface defined in
[`starter-flow-spi`](../../crates/starter-flow-spi) (`FlowStore`,
`RunStore`, `SessionStore`, `AgentSessionStore`) plus per-backend
helpers (`pool/`, `migrate/`, `paging/`, `skills/`). The pattern is
*two parallel implementations behind one trait*, not a single
sqlx-generic layer.

Until this ADR, `flow-agent` consumed `starter-store-sqlite` only,
because:

1. SQLite is simple to run on a Raspberry Pi.
2. The dev loop is fast (in-memory `sqlite::memory:` for tests).
3. SCOPE.md explicitly deferred Postgres as out of scope.

Two things changed:

1. **Drift between backends has already started.** The `flow/`
   module lives only in `starter-store-sqlite`; the `agent_session/`
   module lives only in `starter-store-postgres`. Each new feature
   risks landing in one backend and not the other.
2. **`flow-agent` is an application, not a library.** The starter
   crates have a legitimate reason to support both backends (other
   consumers will pick one or the other). `flow-agent` does not —
   it picks one, ships it, and lives with that choice.

## Decision

**`flow-agent` is Postgres-only.** SQLite support is removed from
this example. The `starter-store-sqlite` crate stays in the
workspace unchanged and continues to serve other consumers
(e.g. [`examples/notes`](../../examples/notes)).

Concretely:

- `examples/flow-agent/Cargo.toml` drops `starter-store-sqlite` and
  the `sqlx` sqlite feature; it adds `starter-store-postgres` with
  the `flow` and `agent-session` features.
- `examples/flow-agent/migrations/flow_agent/` is rewritten in
  Postgres dialect (`JSONB`, `TIMESTAMPTZ`, `$N` placeholders).
- `starter-store-postgres` gains a `flow/` module that mirrors the
  layout of `starter-store-sqlite::flow` and implements the same
  `starter-flow-spi` traits. Translation rules: `?N` → `$N`,
  `TEXT` timestamps → `TIMESTAMPTZ`, JSON-as-TEXT → `JSONB`,
  `CURRENT_TIMESTAMP` → `NOW()`, `INTEGER` booleans → `BOOLEAN`.
- `starter-prefs` gains a Postgres backend so `flow-agent` is not
  forced to keep a side-SQLite database just for preferences.
- `examples/flow-agent/tests/*` switches from `sqlite::memory:` to
  the existing
  [Postgres testcontainers helper](../../crates/starter-store-postgres/src/testing).

## Consequences

### Positive

- **One backend to test, one to deploy, one to debug.** No double
  query maintenance for `flow-agent`'s store code.
- **Dev/prod parity.** Dev, CI, and prod (and Pi deployments) all
  run the same engine and the same query dialect.
- **Removes a quiet drift vector.** Whatever lands in `flow-agent`
  next does not need a SQLite story.

### Negative

- **Postgres becomes a hard prerequisite.** `cargo run -p
  flow-agent` no longer "just works" with a file path; it needs a
  reachable Postgres. Mitigated by a one-line `docker run` in
  [`examples/flow-agent/README.md`](../../examples/flow-agent/README.md).
- **CI weight.** Test runs that previously used an in-memory SQLite
  now need a Postgres container. Already paid for `starter-store-postgres`
  itself; we extend that pattern to `flow-agent`.
- **Raspberry Pi caveat.** Postgres runs fine on a Pi 4/5 with 4 GB+,
  but **SD cards are not acceptable** — WAL fsync on an SD card is
  both slow and destructive. Pi deployments require SSD/USB3
  storage. Documented in the README.

### What this ADR does NOT do

- **Does not remove `starter-store-sqlite` from the workspace.**
  It stays as a library crate for any consumer who wants SQLite
  (small embedded apps, single-file deployments,
  [`examples/notes`](../../examples/notes)).
- **Does not remove the `flow/` module from
  `starter-store-sqlite`.** It's preserved so the SQLite backend
  retains feature parity for any future flow consumer that wants
  SQLite.
- **Does not address starter-crate-level drift between SQLite and
  Postgres backends.** That is a library concern and warrants its
  own work — likely a conformance test suite that runs the same
  trait-level tests against both backends. Tracked separately.

## Rationale

**Dual-backend support belongs in libraries, not applications.**

A library crate that supports SQLite *and* Postgres lets every
downstream consumer pick the right backend for their deployment.
An application that supports both pays the dual-backend tax —
duplicated queries, doubled test matrix, drift risk — without ever
benefiting from the choice at runtime: the application's operator
picks one and uses one.

`flow-agent` is an application. Its operator picks Postgres.
The starter crates remain a menu; `flow-agent` orders from it.

## References

- [`examples/flow-agent/README.md`](../../examples/flow-agent/README.md)
- [`examples/flow-agent/SCOPE.md`](../../examples/flow-agent/SCOPE.md)
- [`crates/starter-store-sqlite`](../../crates/starter-store-sqlite)
- [`crates/starter-store-postgres`](../../crates/starter-store-postgres)
- [`crates/starter-flow-spi`](../../crates/starter-flow-spi)
- [`DOCS/storage/SCOPE.md`](./SCOPE.md) — the broader storage scope
  (this ADR does not modify it; that doc covers blob storage and
  acknowledges the dual SQL crates by design)
