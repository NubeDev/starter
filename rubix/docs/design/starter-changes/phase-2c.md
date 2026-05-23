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
