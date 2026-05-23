## Done

- Verified `cargo build -p starter-tags` and `cargo test -p starter-tags` green: parser (4), pg (4), ch (per suite), match (10), roundtrip (1), semantic_parity (8/8) — D6 invariant holds without DB.
- Verified `git diff master -- crates/starter-store-sqlite` is empty (slice A/B did not touch SQLite).
- Ran `cargo test -p starter-store-postgres --features 'dimensions testing' -- --ignored`:
- `dimensions_basic`: 4/4 pass (entities+refs round-trip, dedicated `_sqlx_migrations_dimensions` version table, tag_definitions via starter-tags types, ext_manifest_approval idempotency).
- `dimensions_prefix`: 2/2 pass — including `two_packs_claiming_the_same_prefix_fail_the_txn` (T6 BI-4).
- `dimensions_marts`: **1 pass / 1 FAIL** — `live_mart_quota_trigger_only_scans_live_rows` panics at line 104 with "quota trigger must reject the 4th live insert". The W12 partial-index-backed live-quota trigger in migration 0005 is not firing as specified.
- Dimensions migrations apply cleanly against an empty Postgres (testcontainer) — the basic suite would not boot otherwise.

## Next

- (none — gate FAILed, no advancement)

## What you need to know

- Sentinel below is FAIL. Per the stage brief ("confirm ... the live-quota trigger test ... pass[es]" and "do not advance without explicit approval"), this halts the job before slice C.
- Likely root causes to investigate next pass: (a) the trigger in `migrations/dimensions/0005_marts_catalog.sql` may be filtering on the wrong status set or scanning the wrong partial index; (b) the trigger may not be `BEFORE INSERT OR UPDATE`; (c) the partial index predicate (`WHERE status = 'live'`) may not match the trigger's count predicate, so the 4th live row slips through.
- Everything else in slices A+B is sound: tag layer's three compile targets (compile_pg jsonb @> only, compile_ch tags['k']=? only, compile_match in-process oracle) all green; PG dialect uses jsonb containment exclusively per T8a; no `->>` / no array ops appear in compile_pg output (asserted by `no_jsonb_extract_or_array_ops_anywhere`); reserved Bool/Str M-2 fix in place; float-literal parser rejection typed.
- Layer-1 invariants (R1/R2/R4/R5, wire formats) are intact — the failure is a schema-trigger correctness bug local to migration 0005, not a structural drift. But it is exactly the kind of tag/schema drift this gate exists to catch.

## Open questions

- (none — failure mode is concrete; remediation belongs to a follow-up WORK stage on slice B's 0005 migration.)

FAIL: live-mart quota trigger test (`dimensions_marts::live_mart_quota_trigger_only_scans_live_rows`) does not reject the 4th live mart insert, so W12's per-tenant live-quota invariant is not enforced by migration 0005's trigger.
