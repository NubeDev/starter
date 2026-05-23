# ADR 0001 — Postgres only

**Status:** accepted, 2026-05-23
**Cites:** [SCOPE Non-goals](../../SCOPE.md), [docs/scope/GAPS.md](../scope/GAPS.md)

## Decision

Rubix uses **Postgres only** for state. SQLite is excluded. ClickHouse
remains the analytical warehouse for history (a separate layer).

## Context

- Starter ships **both** `starter-store-sqlite` and
  `starter-store-postgres` deliberately; this is a *rubix* decision,
  not inherited.
- Goal 4 (ClickHouse) and Goal 6 (analytics) demand the same
  Postgres-tier consistency for state — running SQLite alongside
  would multiply migration surface area for no operator benefit.
- Multi-tenant authz via `starter-authz` is more straightforward
  on Postgres (row-level security available; tenant-scoped query
  filters are first-class).

## Consequences

- Every rubix migration targets Postgres syntax only.
- The "Postgres-only" smoke test in SCOPE asserts zero
  `starter-store-sqlite` resolution in `cargo tree -p rubix-agent`.
- Headless single-host deployments lose SQLite's "zero ops" pitch
  — they have to run a local Postgres. Accepted cost.

## Alternatives considered

- **Both stores supported.** Doubled migration paths; every rubix
  tool would need to be SQL-dialect-portable. Cost not justified.
- **SQLite only.** Rules out hosted multi-tenant deployments
  immediately. Wrong constraint for the target product.
