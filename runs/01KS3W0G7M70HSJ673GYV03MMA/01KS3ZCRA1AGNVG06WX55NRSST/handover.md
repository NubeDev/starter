## Done

- Added `skill_approvals` table migrations to both `starter-store-sqlite/migrations/skills/0001_init.sql` (`approved_at INTEGER` Unix-ms) and `starter-store-postgres/migrations/skills/0001_init.sql` (`approved_at TIMESTAMPTZ`), PK `(skill_id, hash)`.
- Added `SkillApprovalStore` impl of `starter_skills::ApprovalStore` in `crates/starter-store-sqlite/src/skills/` and `crates/starter-store-postgres/src/skills/`, plus the `SKILL_APPROVALS_MIGRATOR` / `SKILL_APPROVALS_MIGRATION_SOURCE` pair in each.
- Wired default-on `skill-approvals` feature in both crates (gates `starter-skills` + `starter-flow-spi` opt-deps and the new modules); `--no-default-features` builds verified clean.
- Postgres impl avoids chrono/time by marshaling ms via `to_timestamp` / `EXTRACT(EPOCH FROM ...)::BIGINT` inline.
- Added round-trip smoke tests (record → lookup → list → revoke → lookup-after-revoke returns None) at `crates/starter-store-sqlite/tests/skills.rs` (runs in-memory, passes) and `crates/starter-store-postgres/tests/skills.rs` (`#[ignore]`, Docker-required, twin of `migrate.rs`).
- Commit `d0ecca1` on branch `codeless/starter-skills`.

## Next

- Stage 10 of 12 per the job goal (next phase per SKILLS.md / stage list — likely ext-flow contributes.skills wiring and/or reference SKILL.md bundles).

## What you need to know

- Feature is named exactly `skill-approvals` (hyphen) and is default-on in both store crates' `[features]`.
- Tests gate on `#[cfg(all(feature = "skill-approvals", feature = "testing"))]`. Run sqlite: `cargo test -p starter-store-sqlite --features "testing skill-approvals" --test skills`. Run postgres (requires Docker): `cargo test -p starter-store-postgres --features "testing skill-approvals" --test skills -- --ignored`.
- Idempotent `record()` uses `ON CONFLICT (skill_id, hash) DO UPDATE` on both backends — a second approval of the same pair refreshes `approved_at` / `approved_by` rather than erroring (the trait contract allows either behaviour).
- `row_to_approval()` clamps negative `approved_at` to 0 rather than wrapping into a giant u64, in case an operator hand-edits a row.

## Open questions

- (none)
