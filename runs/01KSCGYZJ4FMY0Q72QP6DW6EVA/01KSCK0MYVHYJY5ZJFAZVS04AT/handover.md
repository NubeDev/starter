## Done

- Added `packages/rubix-client-ts/src/endpoints/mcp.ts` with `mcpToolsList(opts?)` and `mcpToolsCall(name, args, opts?)`. Both build JSON-RPC 2.0 envelopes, POST to `/api/v1/mcp` (rubix-agent nests starter-mcp's `/mcp` under `/api/v1` in `rubix/crates/rubix-agent/src/main.rs`), thread `opts.acceptLanguage` into `params._meta.acceptLanguage`, and `mcpToolsCall` returns `result.structuredContent`.
- Added sibling `mcp.test.ts` covering tools/list, tools/call, en-US + es-AR `_meta` threading, and JSON-RPC error surfacing (4 tests).
- Added `tests/round-trip.test.ts` exercising one method per endpoint family (system, alert, user, team, tenant, clickhouse, flow_ops, undo, mcp) against a fetch-mock; module header documents the operator flow for live runs (`RUBIX_ROUND_TRIP_BASE` + cookie env vars).
- Updated `vitest.config.ts` include glob and `src/endpoints/index.ts` barrel.
- `pnpm --filter @nube/rubix-client-ts test` green: 10 files, 33 tests. `pnpm typecheck` green.
- Committed as `a1a697c` on branch `codeless/rubix-client-ts`.

## Next

- Stage 14 picks up next (per the 15-stage plan).

## What you need to know

- MCP path resolution: confirmed `/api/v1/mcp` via grep of `rubix-agent/src/main.rs` (line 142 nests `mcp.router` under `/api/v1`). SCOPE has no explicit `OQ-2` section, but the source is unambiguous.
- Locale: stage spec said thread via `_meta.acceptLanguage`. The HTTP transport (`crates/starter-mcp/src/server/http.rs`) also reads the `Accept-Language` header, but `_meta` is the portable contract used by the stdio transport tests; we send only `_meta` per the stage spec.
- Round-trip test uses `as any` for the cross-family invocations to avoid replicating per-method DTOs; this is a wiring test, not a type test (per-method tests already assert types).
- Live-mode round-trip is documented but not auto-wired — operator runs it manually; CI only runs the fetch-mock path. There is no dedicated `docs/design/...` directory for rubix-client-ts in this repo, so the design note lives in the test module's header comment.

## Open questions

- The `acceptLanguage` opts struct only ships `_meta`; if a future stage wants to also send the matching `Accept-Language` HTTP header for HTTP-transport consistency, extend `dispatch()` to merge it into the `headers` object.
- Should `mcpToolsCall` expose the `content` text-rendered parts alongside `structuredContent`? Currently dropped on the floor.
