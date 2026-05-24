## Done

- Added `packages/starter-client-ts/src/client/stream_json.ts` exposing `streamJson<T>(client, path, opts?): AsyncIterable<T>` with both EventSource (withCredentials: true) and fetch+ReadableStream transports, exponential backoff (1s base, 30s cap, ±10% jitter), `onReconnecting(attempt, delayMs)` callback, and AbortSignal-driven clean shutdown.
- Added sibling `stream_json.test.ts` (5 tests) covering frame parsing, reconnect on 5xx, abort mid-stream, and the injected-EventSource path.
- Exported `streamJson` and `StreamJsonOptions` from `src/index.ts`.
- `pnpm --filter @nube/starter-client-ts typecheck` and `test` both green (15/15 tests pass).
- Committed as `35076c9` on branch `codeless/rubix-frontend-wire`.

## Next

- Stage 2 of 16 (next session picks up).

## What you need to know

- The `onReconnecting` callback is the "synthetic reconnecting event" referenced in the spec — fires before each backoff sleep with `(attempt, delayMs)`.
- Backoff sleep is itself abort-aware so consumers (and tests) can short-circuit reconnect by calling `ctrl.abort()` inside the callback.
- The fetch fallback joins multi-line `data:` payloads with `\n` per the SSE spec; comment lines (`: …`) and frames without `data:` are skipped silently.
- `opts.forceFetch` and `opts.eventSourceCtor` exist as test seams (also useful if a browser app wants to opt out of native EventSource).
- Ran `pnpm install --filter @nube/starter-client-ts` to populate node_modules in this worktree before running scripts.

## Open questions

- (none)
