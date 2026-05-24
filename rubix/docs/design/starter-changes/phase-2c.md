# Starter changes — Phase 2c gates

Residual gRPC/CLI rough edges plus the latent `starter-i18n`
interpolate feature-gate bug surfaced during U1. The numbering is
rubix-internal: SCOPE.md's "Phase 2b" (gRPC + CLI on the same
surface) lives here in the ledger as 2c so the discovered-order
chain (2a auth → 2b MCP + flow surfaces → 2c gRPC/CLI + latent
bugs) stays monotonic.

See [README.md](./README.md) for the index and per-item format.
Adjacent: [phase-2a.md](./phase-2a.md) and
[phase-2b.md](./phase-2b.md), both complete.

Expected shape of Phase 2c gaps:

- Possible missing `starter-cli` building blocks for "subcommand
  per tool" auto-generation from a `ToolRegistry`.
- Possible missing `starter-grpc` helpers for streaming the R13
  event taxonomy.

## `starter-i18n` interpolate feature-gate mismatch (latent)

`starter-i18n` fix · surfaced during U1 · **filed (commit `f7b69fd`)**, ~5 LOC + test.

*Bug.* `crates/starter-i18n/src/interpolate.rs:80` gates the
`DiagnosticParam::Quantity` match arm on `starter-i18n`'s own
`units` feature, but the variant itself
(`crates/starter-spi/src/i18n/diagnostic.rs:49`) is gated on
`starter-spi/units` — which `starter-spi/preferences` transitively
enables (`crates/starter-spi/Cargo.toml:82`). Any test graph that
unifies `starter-spi/preferences` from one consumer with
`starter-i18n` from another (no `starter-i18n/units` or
`/preferences`) yields a non-exhaustive match → compile error in
`write_param`. U1 surfaced this when `starter-mcp` took a dev-dep
on `starter-i18n` alongside `starter-server`'s `testing` feature
(which pulls `starter-spi/preferences`).

*Workaround in tree.* `crates/starter-mcp/Cargo.toml:37-42` pins
`starter-i18n = { features = ["preferences"] }` under
`[dev-dependencies]`. Remove that pin once the upstream fix lands.

*Shape.* Either handle `Quantity` unconditionally in `write_param`
(canonical render needs no `uom` types beyond what the variant
already carries), or align the `#[cfg]` so the arm is present
whenever the variant is — i.e. `#[cfg(feature = "units")]` on the
arm must track the same condition as the variant in
`starter-spi`. Unconditional is simpler; the arm only formats
`canonical: f64` and reads `quantity` by reference.

*Test.* `cargo test -p starter-mcp --all-features` (and
`-p starter-mcp` with the dev-dep pin removed) must build and
pass. Add a regression test in `starter-i18n` that enables
`starter-spi/preferences` without `starter-i18n/units` and
interpolates a `Quantity` param.

*Status.* Filed for fix-when-touched; not blocking PR 3.

## `starter-store-clickhouse` TTL on `DateTime64` rejected by CH 24+ (latent)

`starter-store-clickhouse` fix · surfaced during the PR #28 smoke
test · **landed (in-tree)**, three-line fix per file.

*Bug.* `CREATE TABLE` with `TTL <DateTime64 column> + INTERVAL N
DAY` is rejected by ClickHouse 24+ with
`Code: 450. DB::Exception: TTL expression result column should
have DateTime or Date type, but has DateTime64(3).
(BAD_TTL_EXPRESSION)`. The three shared migrations declared
`DateTime64(3)` columns for the timestamp (`received_at`, `ts`)
and used those columns directly in the TTL clause; CH 23.x
accepted the expression, CH 24.x does not.

*Fix.* Cast the TTL expression to `DateTime` so the result type
satisfies the engine's constraint. The retention bound is
day-grained anyway, so the dropped sub-second precision is
irrelevant for TTL.

*Files changed.*
- `crates/starter-store-clickhouse/migrations/0001_raw_events.sql`
- `crates/starter-store-clickhouse/migrations/0002_samples.sql`
- `crates/starter-store-clickhouse/migrations/0003_events.sql`

*Rationale for cast-not-pin.* Pinning the rubix compose image to
ClickHouse 23.x would mask the same bug for every other consumer
that boots starter-store-clickhouse against CH 24+. The cast keeps
the migration valid on every CH version the crate documents
support for.

## `starter-store-clickhouse` `bloom_filter` index on `Map` rejected by CH 24+ (latent)

`starter-store-clickhouse` fix · surfaced alongside the TTL fix
above · **landed (in-tree)**, one-line fix per file.

*Bug.* `CREATE TABLE ... INDEX <name> tags TYPE bloom_filter ...`
where `tags` is `Map(String, String)` is rejected by CH 24+ with
`Code: 44. DB::Exception: Unexpected type Map(String, String) of
bloom filter index. (ILLEGAL_COLUMN)`. CH 23.x silently accepted
the declaration; CH 24.x requires the index to be declared on a
column the bloom filter knows how to hash.

*Fix.* Index `mapKeys(tags)` instead of `tags`. The dominant
"does this tag key exist?" lookup is unchanged; value-level skip
needs a separate `tokenbf_v1` index on `mapValues(tags)` if it
ever becomes a hot path.

*Files changed.* Same three files as the TTL fix above
(`0001_raw_events.sql`, `0002_samples.sql`, `0003_events.sql`).
