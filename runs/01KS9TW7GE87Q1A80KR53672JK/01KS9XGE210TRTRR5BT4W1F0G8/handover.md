## Done

- Wrote rubix/docs/design/OVERVIEW.md (237 lines): repo map + Rust/TS dep arrows + the "where does my code go" table + 8 smoke tests
- Wrote rubix/docs/design/EVERYTHING-AS-NODE.md (223 lines): R2 in full with concrete node/not-node tables, the load-bearing reader test, the Mutex-without-a-slot trap, R3 corollary
- Wrote rubix/docs/design/NODE-AUTHORING.md (309 lines): KindManifest + NodeBehavior recipe, slot-write chokepoint via R3, propagator/R6 interaction, FSM hook placement, registration site, tests-with-code
- Wrote rubix/docs/design/KIND-MANIFEST.md (321 lines): top-level struct, every field (id/version/title/desc/slots/facets/permissions/capabilities/placement) with R10 bump matrix, block.yaml wrapper, codegen surface, common pitfalls
- Wrote rubix/docs/design/AUTH.md (282 lines): four Studio→agent paths (cookie/gRPC-bearer/MCP-bearer/CLI), Principal resolution, AuthZ layer order (with_role→with_scope→with_permission), Zitadel hookup, tenants/teams composition, Phase 7 smoke tests
- Wrote rubix/docs/design/MIGRATIONS.md (279 lines): namespaced runner sources, boot order (starter_* first then rubix), no-cross-tree-FK rule with CI parser, forward-only rollback, ADR-001 SQLite ban, authoring checklist
- Wrote rubix/docs/design/TESTS.md (350 lines): R11 presence rule, 1:1 file mirror, three tiers (unit/integration/transport), SPI contract-test pattern, in-memory paths under testing feature, mock policy, CI lanes
- Wrote rubix/docs/design/VERSIONING.md (376 lines): R10 contract surfaces, full breaking-change taxonomy per surface (Rust API, KindManifest, per-kind version, Msg, REST DTO, block.yaml, proto, CLI), #[non_exhaustive] discipline, deprecation cycle, manifest migrations, boot compatibility
- `mani run lint --all` green (R1 400-line budget passes for all 8 docs)
- Committed as 2d9aa1d on codeless/rubix-phase-0

## Next

- Stage 4 (per the source SCOPE's stage breakdown): there is no stage 4 in the job — this is stage 3 of 3 substantive stages. A fresh session will pick up Phase 0 final verification or open Phase 1.

## What you need to know

- File sizes: OVERVIEW 237, EVERYTHING-AS-NODE 223, NODE-AUTHORING 309, KIND-MANIFEST 321, AUTH 282, MIGRATIONS 279, TESTS 350, VERSIONING 376. All under 400 per R1.
- Source SCOPE is `/home/user/code/rust/starter/rubix/SCOPE.md` (1238 lines); the worktree has no rubix/SCOPE.md, so the docs cite "rubix/SCOPE.md §..." pointing at the canonical path.
- Each doc opens with a "> Source:" block listing the R-numbers and SCOPE sections it expands on, satisfying the "cites the source" requirement.
- Docs cross-link each other (e.g. NODE-AUTHORING → KIND-MANIFEST, AUTH → MIGRATIONS) so a contributor reading just these eight has the closure the brief asked for.
- The AUTH doc references the Phase 7 work already in this repo (examples/authz-demo, the with_role→with_scope→with_permission layer order, the surface field, the unknown-resource-is-build-error smoke) as the working example a contributor can read.
- The KIND-MANIFEST and VERSIONING docs share the per-kind manifest version matrix; VERSIONING is the master, KIND-MANIFEST restates the per-kind row.

## Open questions

- (none)
