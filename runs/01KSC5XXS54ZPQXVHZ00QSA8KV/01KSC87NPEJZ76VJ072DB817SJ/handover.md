## Done

- Filled `rubix-tools/src/user/list.rs` (read-only `UserListTool`, sorts by email, emits `rubix.user.listed`, no `Reversible`).
- Filled `rubix-tools/src/tenant/list.rs` + new `tenant/store.rs` with `TenantStore`/`InMemoryTenantStore` + `TenantListTool` (sorts by name, emits `rubix.tenant.listed`).
- Filled DTO placeholders `rubix-spi/src/dto/user/list.rs` and `dto/tenant/list.rs` (request/response + five-field `ToolDescriptor`).
- Added `UserAdminStore::list()` to the trait + in-memory impl.
- Catalogue entries `rubix.user.listed` / `rubix.tenant.listed` added to both `en.json` and `es.json` (R5).
- Updated `rubix-skills/skills/user-admin/SKILL.md` with present-tense `## Tools` and `## Localisation` sections naming the six verbs + `rubix.undo.last`.
- Populated `rubix-flows/flows/user-admin.yaml` `allowed_tools: [rubix.user.create, rubix.user.disable, rubix.user.list, rubix.team.create, rubix.team.assign, rubix.tenant.list, rubix.undo.last]`.
- `cargo test -p rubix-tools --lib` → 36 passed; `cargo build -p rubix-tools -p rubix-spi` clean.
- Committed as `32c5608` titled "phase B.2 — Goal 2 reads + skill + flow YAML".

## Next

- Stage 7 / Phase B.3 — Goal 2 integration test under `rubix/crates/rubix-agent/tests/goal_2_user_admin_test.rs` per SCOPE line 25 (fires `tools/call com.rubix.user-admin`, asserts diagnostic + undo registry, then exercises undo).
- Design doc `rubix/docs/design/user-admin/README.md` present-tense covering all six verbs + Reversible contract (some verbs already reference it).

## What you need to know

- Used the full `rubix.*` tool ids in the flow YAML (matches existing `scheduled-system-check.yaml`), not the unprefixed forms shown in the stage prompt — `convert.rs` matches against `ToolDefinition.name` which is the prefixed form.
- `tenant/store.rs` is new; the production binary will swap in a PG-backed `TenantStore` later. `InMemoryTenantStore::seeded` exists for tests.
- `DiagnosticParam::I64` is the int variant (no `Int`).
- Pre-existing `rubix/scripts/lint-doc-refs.sh` failure in `rubix-store-postgres/src/lib.rs:12` (references `rubix/SCOPE.md`) — unrelated to this stage, was already on master.
- Workspace-wide `cargo build` requires rustc ≥ 1.91 (aws-smithy deps); affected crates here build fine on the current toolchain.

## Open questions

- (none)
