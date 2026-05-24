## Done

- Reviewed Phase B diff (b87d797 + dd5db0c + 6f633e6 plus intermediates) against R1/R2/R4/R5 + wire-format invariants
- cargo check -p rubix-agent -p rubix-tools green on the gate tip
- Recorded sentinel commit dc91085

## Next

- Phase C kickoff: Goal 4 (clickhouse-ruler) per SCOPE. Before Phase C the operator should decide whether the in-memory store under rubix-tools/src/user/store.rs is acceptable for the Goal 2 demo or whether the PG-backed store needs to land first (the design doc already names `starter-auth-users` as the eventual backend)

## What you need to know

- PASS: Layer-1 invariants hold — dependency arrow is clean (rubix-tools → rubix-spi → starter-spi), no new transport bypassing Tool/MCP, dispatch-only tool files with DTOs+descriptors+REQUIRED_PERMISSION in rubix-spi, and DTO populations are first-time fills rather than wire-format breaks
- Three Phase B commits: B.1 user/team write verbs + Reversible + Diagnostic + en.json/es.json (b87d797 + sub b87d797); B.2 user/tenant read verbs + SKILL.md + user-admin.yaml allowed_tools (dd5db0c + sub 32c5608); B.3 integration test rubix-agent/tests/goal_2_user_admin_test.rs + docs/design/user-admin/README.md (6f633e6 + subs c07f19c, e50690a)
- Operator-runnable manual flow (works in-memory; will not write to PG until the PG store lands): start rubix-agent → `curl -X POST http://localhost:8080/mcp -d '{"method":"tools/call","params":{"name":"rubix.user.create","arguments":{"email":"ada@example.com","role":"admin"}}}'` → expect Diagnostic code `rubix.user.created` and a user_id → `curl -X POST … {"method":"tools/call","params":{"name":"rubix.undo.last","arguments":{"scope":{"resource_kind":"user"}}}}` → expect Diagnostic `rubix.undo.applied` and the user's disabled flag flipped (visible via `rubix.user.list`)
- Integration test goal_2_user_admin_test.rs exercises the same two scenarios in-process against the in-memory store and the recorded-LLM harness

## Open questions

- PG-backed user store: ship before Phase C, or accept the in-memory store for the demo and schedule PG wiring after Goal 4? (Operator decision; gate does not require it for Layer-1 PASS.)
