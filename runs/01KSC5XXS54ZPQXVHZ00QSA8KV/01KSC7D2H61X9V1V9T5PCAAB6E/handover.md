## Done

- filled rubix-tools/src/user/{create,disable}.rs and rubix-tools/src/team/{create,assign}.rs per SCOPE Phase B: each Tool impl emits a Diagnostic and the companion ReversibleTool::change_for produces a ChangeDraft on the success path
- added per-kind helper modules rubix-tools/src/user/store.rs (UserAdminStore + InMemoryUserStore + UserReversible) and rubix-tools/src/team/store.rs (TeamAdminStore + InMemoryTeamStore + TeamReversible + TeamPatch sparse-update payload)
- filled DTO placeholders in rubix-spi/src/dto/user/{create,disable}.rs and rubix-spi/src/dto/team/{create,assign}.rs with utoipa request/response structs + five-field ToolDescriptor + REQUIRED_PERMISSION
- added 5 MessageKeys to rubix-spi/catalogues/en.json + es.json: rubix.user.created, rubix.user.disabled, rubix.user.already_disabled, rubix.team.created, rubix.team.assigned
- wrote present-tense rubix/docs/design/user-admin/README.md documenting verb surface, store seam, and snapshot shape per kind
- cargo test -p rubix-tools --lib: 31 passed (8 new tests for the four verbs + store helpers)
- committed as `phase B.1 — Goal 2 verbs (writes first) — feat(rubix-tools) user-admin write verbs` (b87d797), commit body lists the five MessageKeys

## Next

- (none — next stage is the gate / phase B.2 picked up by a fresh session)

## What you need to know

- Tools take an Arc<dyn UserAdminStore>/Arc<dyn TeamAdminStore>; only the in-memory impl exists today, the PG-backed impl is a follow-up stage
- user.disable response DTO grew a `role` field so the UserRow snapshot can be reconstructed in change_for without a re-read of the store
- team.assign stashes `_prior_members` / `_new_members` on the response JSON (alongside the typed DTO fields) so change_for can build the sparse TeamPatch; these keys are an internal contract between the verb and TeamReversible and are not part of the public DTO
- idempotent re-calls (`user.already_disabled`, `team.assigned` with `already_member=true`) intentionally return `None` from change_for so undo cannot unwind a no-op
- lint-doc-refs reports one forbidden ref in rubix-store-postgres/src/lib.rs that pre-exists this stage (introduced in stage A.2); did not touch it

## Open questions

- (none for this stage)
