## Done

- Phase C gate evaluated against L1 invariants R1/R2/R4/R5; all hold.
- Verified Phase C commits: fd4c5cc (C.1 scaffold), f618555 (C.2 system+alert+audit), b3b926c (C.3 user+team+tenant), 964d904 (C.4 clickhouse+flow_ops+undo), 97377e8 (C.5 MCP + round-trip test).
- Ran `pnpm --filter @nube/rubix-client-ts typecheck` — clean.
- Ran `pnpm --filter @nube/rubix-client-ts test` — 10 files, 33 tests pass (alert 2, clickhouse 4, flow_ops 5, mcp 4, system 5, team 3, tenant 2, undo 3, user 4, round-trip 1).
- Counted endpoint files: 9 (alert, clickhouse, flow_ops, mcp, system, team, tenant, undo, user) vs 11 rubix-tools verb dirs in Rust.
- PASS: gate holds.

## Next

- Phase D per SCOPE — fresh session will pick up.

## What you need to know

- Per-method-file count is 9 TS vs 11 Rust verb dirs. Deviation is intentional and SCOPE-aligned: TS omits analytics/clipboard/dashboard/tags (Goals 1/6 still stubbed upstream; tags/clipboard out-of-scope for Phase C) and adds alert + mcp because they are agent routes rather than rubix-tools dirs. No L1 violation.
- Every mutating verb threads `readCsrfHeader()` into the request headers; reads do not (matches starter-client-ts convention).
- All endpoints dispatch through `fetchJson(this.starter, …)` — single transport invariant intact.
- Operator manual flow: boot `cargo run -p rubix-agent`, then run a tiny ESM file that does `new RubixClient(new StarterClient(...))` and calls `rubix.disk()` + `rubix.undoLast()` (script body included in the stage reply). `undoLast` on an empty queue returns a Diagnostic with `code = rubix.undo.nothing-to-undo`.

## Open questions

- (none)

PASS: rubix-client-ts depends only on starter-client-ts, every endpoint uses the starter fetch helpers, mutating verbs thread readCsrfHeader, generated types come from the committed openapi.json snapshot, typecheck clean, 33/33 tests green.
