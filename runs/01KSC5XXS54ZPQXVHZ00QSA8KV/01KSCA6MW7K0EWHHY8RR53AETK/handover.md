## Done

- Reviewed Phase C diff (3 commits between 87d7f75..HEAD, 16 files, +1752/-39).
- Confirmed Layer-1 invariants R1/R2/R4/R5 + wire-format compatibility all hold.
- Ran `cargo test -p rubix-agent --test goal_4_clickhouse_ruler_test` → 1 passed.

## Next

- PASS: proceed to Phase D (Goal 3 flow-programmer). A fresh session picks up stage 12.

## What you need to know

- Phase C commits: b16b487 (verbs + i18n), d453f38 (skill + flow YAML), 4af90cd (integration test + design doc).
- Integration test count for Goal 4: 1 (retention.set round-trip with snapshot + undo restore).
- Operator-runnable manual flow once `rubix-admin mcp` wiring lands the verbs in `boot::mcp::register::build_flow_registry`: `curl -X POST $MCP/tools/call -d '{"name":"clickhouse-ruler","arguments":{"intent":"set retention on system_disk_history to 30 days"}}'` → verify with `clickhouse-client -q "SELECT engine_full FROM system.tables WHERE name='system_disk_history'"` shows the new `TTL ts + toIntervalDay(30)`; then `curl -X POST $MCP/tools/call -d '{"name":"rubix.undo.last","arguments":{}}'` → re-query system.tables and assert the prior TTL is restored.
- Caveat (already in design doc + test header): production CH-backed `ChWriter` is deferred; the verb logic and snapshot/undo plumbing run today against `InMemoryChWriter`. Trait shape is the contract — production swap is a one-line wiring change in agent boot.
- PASS: Phase C Layer-1 invariants hold — crate direction intact, no new transport, trust boundary unchanged, DTO additions are backward-compatible optional fields.

## Open questions

- (none)
