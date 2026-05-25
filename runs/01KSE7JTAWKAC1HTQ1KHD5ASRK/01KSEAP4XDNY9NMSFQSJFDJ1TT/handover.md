## Done

- Phase C.2 landed: CreateDashboardRequest/Response + UpdateDashboardRequest/Response DTOs in rubix-spi/src/dto/dashboard/{create,update}.rs with utoipa ToSchema and five-field descriptors.
- DashboardCreateTool + DashboardUpdateTool in rubix-tools/src/dashboard/{create,update}.rs implementing Tool + ReversibleTool. Create validates `dashboard.<slug>`, refuses duplicates (rubix.dashboard.create.duplicate_id), writes via DashboardStore::insert_revision, re-asserts the rubix.dashboard.page ResourceSpec on StaticRegistry. Update enforces optimistic concurrency on expected_revision_id (rubix.dashboard.update.conflict, Error::Conflict→HTTP 409), preserves owner_principal + prior title/tags when omitted.
- New rubix-tools/src/dashboard/store.rs: DashboardReversible (resource kind `rubix.dashboard.page`) + DashboardSnapshot payload. Op::Create reverses as mark_superseded; Op::Update reverses by re-inserting the `before` body (insert-only model auto-supersedes the post-update head).
- Four MessageKeys added en + es catalogues same commit: rubix.dashboard.created, rubix.dashboard.updated, rubix.dashboard.update.conflict, rubix.dashboard.create.duplicate_id.
- starter-authz added to rubix-tools/Cargo.toml.
- Sibling tests: create→undo→supersede, update→conflict-on-stale, duplicate-id refusal, invalid page_id, missing page → NotFound, omitted title/tags preserved, change_for drafts shape.
- cargo test -p rubix-spi -p rubix-tools green (16 dashboard:: tests pass, no regressions).
- Commit 3a632b8.

## Next

- Stage 9 (next session): phase C.3 — dashboard.delete + dashboard.duplicate + dashboard.history per 04-tools.md.

## What you need to know

- Update's ReversibleTool::change_for currently records `before: None` (only `after` snapshot) — full byte-for-byte undo of an update needs a `prior_snapshot` capture seam (store returning prior body alongside the insert). Inline doc comment in update.rs flags it as a follow-up; create-side undo path is complete.
- DashboardCreateTool::new takes (store, registry). Wiring it into the tool registry / SDUI HandlerRegistry will need an Arc<StaticRegistry> handed in at boot — that wiring lands later in the phase D mounts.
- DashboardSnapshot is the on-the-wire payload stamped into Change::before/after; bump it carefully if the kind grows new columns.

## Open questions

- (none)
