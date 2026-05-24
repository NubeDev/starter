## Done

- Reviewed Phase A diff (d6a1827 scaffold + 6d893b3 test+SCOPE). New crate `starter-ext-store-pg` is an adapter depending on starter-ext-spi + starter-ext-server. Only starter-ext-server change is re-exporting the pre-existing `StoreError` type — no new public surface, no transport, no wire-format, no trust-boundary edits.
- Verified `cargo build -p starter-ext-store-pg` and `cargo test -p starter-ext-store-pg --no-run` are green.
- Confirmed two `cargo build --workspace` errors (starter-i18n `DiagnosticParam::Quantity` non-exhaustive match; starter-ext-sdk duplicate `__STARTER_EXT_FLAVOUR_MARKER` symbol) reproduce on master (d6a1827~1) — pre-existing, not introduced by Phase A.
- Confirmed SCOPE update at `DOCS/extensions/scope/SCOPE.md` L1310-1314 names `starter-ext-store-pg` as the default DB-backed `EnablementStore` impl and cites the testcontainers coverage.
- Presented OQ-6 evidence: migration owned at `starter-extensions/crates/starter-ext-store-pg/src/migrations/0001_extensions_enablement.sql` — independent of any rubix migration root; consumer wires its own Migrator pointed at the crate's `migrations/`.
- Emitted gate sentinel and committed `stage 3: gate — Phase A landed — PASS` (empty commit `09e67bb`).

## Next

- PASS: Layer-1 invariants (R1 dependency direction, R2 single transport, R4/R5 trust boundary, wire formats) hold; Phase B (rubix-agent boot wiring of `PgEnablementStore` + admin REST surface) may begin in the next session.

## What you need to know

- The starter-extensions workspace is NOT fully green on master; do not treat a red `cargo build --workspace` in later stages as a Phase A regression unless new errors appear beyond starter-i18n's `DiagnosticParam::Quantity` and starter-ext-sdk's duplicate flavour-marker symbol.
- `StoreError` was re-exported from starter-ext-server in Phase A.1; Phase B consumers can `use starter_ext_server::StoreError` directly.
- Phase B must run the crate-owned migration via its own sqlx `Migrator` (per OQ-6 + WORKFLOW.md L17/L35), not by appending to any rubix migration set.

## Open questions

- (none)
