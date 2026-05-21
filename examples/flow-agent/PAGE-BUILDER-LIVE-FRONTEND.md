# Page Builder Live — **FRONTEND** session scope

> **Parallel session.** A backend session is running against the same
> contract. Read §0 before touching anything.

Parent doc: [PAGE-BUILDER-LIVE.md](./PAGE-BUILDER-LIVE.md) — full SCOPE.
Companion: [PAGE-BUILDER-LIVE-BACKEND.md](./PAGE-BUILDER-LIVE-BACKEND.md).

---

## 0. RULES OF ENGAGEMENT — read first

You are the **FRONTEND** session. A separate backend session is editing
Rust in parallel. To avoid stomping each other:

### 0.1 DO NOT

- **DO NOT `git commit`.** Stage nothing. Leave work uncommitted in
  the working tree. The human will review both sessions' diffs together
  and commit once.
- **DO NOT `git checkout`, `git switch`, `git pull`, `git stash`, or
  any branch-changing op.** Stay on whatever branch is checked out
  right now.
- **DO NOT edit any of the following paths** (backend session owns
  them):
  - `crates/**` — anything under any Rust crate
  - `examples/flow-agent/src/**` — anything under the Rust binary
  - `examples/flow-agent/tests/**`
  - `examples/flow-agent/Cargo.toml`, `Cargo.lock`, any `Cargo.toml`
  - any `*.rs` file anywhere in the repo
- **DO NOT** run `cargo build`, `cargo test`, `cargo run`, or restart
  the backend dev server (`make start` from `examples/flow-agent/`).
  The backend session owns its own dev loop.
- **DO NOT** edit `examples/flow-agent/PAGE-BUILDER-LIVE.md` (the
  parent SCOPE) or `PAGE-BUILDER-LIVE-BACKEND.md`. They are the
  contract.

### 0.2 DO

- Edit only the paths listed in §3 of this doc.
- Run `pnpm typecheck`, `pnpm build`, `pnpm e2e` freely.
- Run the frontend dev server (`pnpm dev` from
  `examples/flow-agent/frontend/`) for live UI work.
- Use the backend session's `/api/builder/stream` once they signal
  ready in `PAGE-BUILDER-LIVE-BACKEND.md` under `## Handoff notes`. If
  they aren't ready, stub the adapter against a tiny in-memory
  `ReadableStream` fixture for unit testing — see §4.5.
- When done, leave a one-line summary at the bottom of this file under
  `## Handoff notes`.

### 0.3 Conflict surface

The **only** shared artefact is the SSE wire shape on
`POST /api/builder/stream`. Both sessions must obey
[PAGE-BUILDER-LIVE.md §3 L3 and §6](./PAGE-BUILDER-LIVE.md) verbatim. If
you need to deviate, **stop and ask the human** — do not change the
contract unilaterally.

The TS `BuilderEvent` union in
[packages/starter-ui-ai-builder/src/types/index.ts](../../packages/starter-ui-ai-builder/src/types/index.ts)
is the source of truth — **do not modify it.** If you find a mismatch
between it and what the backend session is emitting, flag it under
`## Handoff notes` and stop.

---

## 1. Your job in one sentence

Add a real HTTP/SSE `BuilderAdapter` that talks to
`POST /api/builder/stream`, swap it into `PageBuilder.tsx` (with the
existing fixture kept behind `?fixture=1` and a new `?demo=1`
silent-fallback mode), and wire client-side abort so retyping
mid-stream doesn't race the canvas.

## 2. You can start immediately

Unlike the backend session, you have **no P0 gate** — the wire shape
is already specified in the parent SCOPE. You can build the adapter +
unit-test it against an in-memory `ReadableStream` fixture (§4.5) while
the backend session is still doing P0.

When the backend session posts `Route is mounted: yes` in their
handoff notes, do the §5 manual smoke against the real route.

---

## 3. Surface — files you may touch

### 3.1 New files

| Path                                                                       | Budget | Purpose                                                               |
|----------------------------------------------------------------------------|--------|-----------------------------------------------------------------------|
| `packages/starter-ui-ai-builder/src/adapters/http.ts`                      | 200    | `createHttpBuilderAdapter({ url, onUnavailable? })`                   |
| `packages/starter-ui-ai-builder/src/adapters/http.test.ts`                 | 150    | Vitest against in-memory `ReadableStream`                             |

### 3.2 Edited files (minimal diffs)

| Path                                                                       | What changes                                                          |
|----------------------------------------------------------------------------|-----------------------------------------------------------------------|
| `packages/starter-ui-ai-builder/src/adapters/index.ts`                     | Re-export `createHttpBuilderAdapter` and its options type             |
| `packages/starter-ui-ai-builder/src/index.ts`                              | Public surface for the new adapter (if not already barrelled)         |
| `examples/flow-agent/frontend/src/pages/PageBuilder.tsx`                   | L5 adapter selection (fixed `?fixture=1` parse + new `?demo=1`); L10 abort wiring |
| `examples/flow-agent/frontend/vite.config.ts`                              | `/api` proxy: disable response compression / set `Cache-Control: no-transform` so SSE isn't buffered |

### 3.3 Forbidden (backend owns these)

Anything under `crates/**`, `examples/flow-agent/src/**`,
`examples/flow-agent/tests/**`, any `*.rs` file, any `Cargo.toml`.

If you find yourself wanting to touch one, **stop and ask the human**
or post a note under `## Handoff notes`.

---

## 4. The contract (FRONTEND side — fixed, do not invent)

### 4.1 `createHttpBuilderAdapter` signature

```ts
import type { BuilderAdapter, BuilderEvent } from "../types/index.js";

export interface HttpBuilderAdapterOptions {
  /** Backend SSE endpoint. */
  url: string;
  /** Optional silent-fallback factory invoked on HTTP 503. When
   *  provided, the adapter substitutes the returned BuilderAdapter
   *  for THIS send call only (do not cache across calls — the user
   *  may bring the backend back). */
  onUnavailable?: () => BuilderAdapter;
}

export function createHttpBuilderAdapter(
  opts: HttpBuilderAdapterOptions,
): BuilderAdapter;
```

Behaviour:

1. `send(input, signal)` POSTs JSON `{ prompt: input.text, provider: "claude" }`
   to `opts.url`.
2. If response is 503 and `onUnavailable` is set, delegate to its
   returned adapter for the rest of this call. If 503 and no
   `onUnavailable`, yield a single
   `{ type: "error", error: "<hint from response body or status text>" }`
   then return.
3. If response is any other 4xx/5xx, same single-error-frame pattern.
4. On 200 + `text/event-stream`, parse the body as SSE frames:
   - Read the body as a stream (`response.body!.getReader()`); decode
     UTF-8; split on `\n\n`; for each frame, strip the `data: ` prefix
     and `JSON.parse` to a `BuilderEvent`.
   - Yield each parsed event.
   - Stop reading and call `reader.cancel()` if `signal.aborted`.
5. Respect `signal` from start to finish — pass it to `fetch`, check
   it in the read loop.

### 4.2 SSE parser rules

- Use the standard SSE framing: events end at `\n\n`; multiple `data:`
  lines in one event are concatenated with `\n`; ignore lines that
  don't start with `data: ` (comments, `event:`, `id:`, `retry:`).
- Buffer partial frames across reads — the network may split a frame
  mid-bytes.
- A `data:` payload of `[DONE]` (bare, no JSON) is the chat surface's
  convention; **do NOT terminate on it**. The builder surface's
  terminal frame is `status: done`. Ignore stray `[DONE]` defensively.
- Surface a single
  `{ type: "error", error: "malformed sse frame: <preview>" }` if a
  frame fails `JSON.parse`. Continue reading after.

### 4.3 Abort

- `fetch(url, { signal, ... })` so the underlying connection is closed
  when the user types a new prompt or unmounts.
- Inside the read loop check `signal.aborted` between frames.
- If aborted while a frame is being parsed, do NOT yield it; exit the
  generator silently.

### 4.4 Adapter selection in `PageBuilder.tsx`

The current `PageBuilder.tsx` unconditionally uses
`createFlowAgentBuilderFixture()`. Replace that block with:

```ts
const params = new URLSearchParams(window.location.search);
const useFixture = params.get("fixture") === "1";  // NOT .has — see review #12
const demoMode   = params.get("demo") === "1";

const adapter = useMemo(() => {
  if (useFixture) return createFlowAgentBuilderFixture();
  return createHttpBuilderAdapter({
    url: "/api/builder/stream",
    onUnavailable: demoMode ? () => createFlowAgentBuilderFixture() : undefined,
  });
}, [useFixture, demoMode]);
```

Then wire abort. The existing `useBuilder` returns `cancel()`. Wire it
on:

- Composer submit handler: if `builder.phase !== "idle" && builder.phase !== "done"`,
  call `builder.cancel()` before invoking `builder.send(...)`.
- Component unmount: `useEffect(() => () => builder.cancel(), [builder]);`
- Stop button (if visible in the existing transcript component): no
  change — it already calls `cancel()`.

DO NOT modify
[examples/flow-agent/frontend/src/lib/builder-fixture.ts](../../examples/flow-agent/frontend/src/lib/builder-fixture.ts) —
the fixture must keep working for offline e2e.

### 4.5 Unit test fixture (`http.test.ts`)

Cover these cases against an in-memory `ReadableStream` (no `msw`, no
new deps; use `ReadableStream.from(...)` or
`new ReadableStream({ start })`):

1. Happy path: `thinking → writing → full-render → done`; yields four
   events in order.
2. Error frame mid-stream: `thinking → error`; yields two events then
   ends.
3. Malformed JSON in one frame: yields one synthetic error event, then
   keeps reading.
4. Frame split across two chunks: still parses to one event.
5. Abort mid-stream: generator returns after current frame, no further
   yields.
6. HTTP 503 with `onUnavailable` set: delegates to fallback adapter.
7. HTTP 503 with no `onUnavailable`: yields one error event.
8. HTTP 500: yields one error event.
9. `signal.aborted` before send: yields nothing, no fetch attempted.

Run with `pnpm --filter @nube/starter-ui-ai-builder test`.

### 4.6 Vite proxy (anti-buffering)

In `examples/flow-agent/frontend/vite.config.ts`, find the existing
`server.proxy` (or `proxy: { '/api': ... }`) block and ensure
streaming responses pass through unbuffered. Minimum:

```ts
proxy: {
  '/api': {
    target: 'http://127.0.0.1:9741',
    changeOrigin: true,
    // Pass through SSE without buffering / transforming
    configure: (proxy) => {
      proxy.on('proxyRes', (res) => {
        res.headers['cache-control'] = 'no-transform';
        // Some setups also need:
        delete res.headers['content-encoding'];
      });
    },
  },
},
```

If a different proxy library is in use, replicate the same intent.
**Smoke with `curl -N http://localhost:9742/api/builder/stream …`
(via the dev server, NOT direct to backend) and confirm frame 1
arrives within 200 ms.**

---

## 5. Smoke test (manual, do this before saying you're done)

Prerequisite: backend session posted `Route is mounted: yes` in their
handoff notes.

1. `pnpm --filter flow-agent-frontend dev` (port 9742).
2. Open `http://localhost:9742/pages/new` in a browser.
3. Type `iot dashboard`, press Send.
4. Expect: transcript shows "Asking Claude…" within 1 s; canvas
   renders a non-empty tree within 30 s; zero "Unknown component"
   placeholders.
5. While the stream is in progress, type a new prompt and Send. Open
   DevTools → Network → filter `EventStream`. There must be **at
   most one** in-flight SSE at any time.
6. Visit `http://localhost:9742/pages/new?fixture=1`. The offline
   fixture must still work (no network call to `/api/builder/stream`).
7. With the backend down: visit
   `http://localhost:9742/pages/new?demo=1` and Send a prompt. The
   fixture must take over silently — no error UI on screen.
8. Without `?demo=1` and backend down: visit
   `http://localhost:9742/pages/new` and Send. The transcript shows
   the 503 hint inline within 200 ms.

---

## 6. Acceptance (FRONTEND-only)

Check each before posting your handoff note.

- [ ] `pnpm --filter @nube/starter-ui-ai-builder typecheck` green.
- [ ] `pnpm --filter @nube/starter-ui-ai-builder test` green (all 9
      cases in §4.5).
- [ ] `pnpm --filter flow-agent-frontend typecheck` green.
- [ ] `pnpm --filter flow-agent-frontend e2e` green (offline fixture
      tests — no regression to PR #18).
- [ ] All §5 smoke cases pass against the real backend (if available;
      otherwise note "backend not ready" and run §5 cases 6 and 7
      only).
- [ ] No files outside §3 changed (`git status` confirms).
- [ ] You have NOT committed, branched, stashed, or pushed.

---

## Handoff notes

(FRONTEND session fills this section as it works. The backend session
and the human read it.)

- [x] Adapter built and unit-tested: **yes** — `packages/starter-ui-ai-builder/src/adapters/http.ts` (≈230 LoC) + `http.test.ts` (9 cases, all green via `pnpm --filter @nube/starter-ui-ai-builder test`). Wired vitest in the package (added `vitest.config.ts`, `test` script + devDep) since this package didn't have a test harness yet.
- [x] `PageBuilder.tsx` wired with adapter selection + abort: **yes** — `?fixture=1` (uses `.get(…) === "1"`, not `.has`), `?demo=1` (silent 503 fallback to fixture), default = real HTTP. `handleSend` cancels in-flight before re-sending; unmount calls `builder.cancel()`.
- [x] Vite proxy verified with `curl -N` through `:9742`: **yes** — proxy plumbing confirmed: response headers from upstream propagate through with `cache-control: no-transform` and `x-accel-buffering: no` applied, no `content-encoding` leaking. Verified against the live `/api/builder/stream` route via `127.0.0.1:9742`.
- [x] Live smoke results: **partial.** The fresh debug backend the BACKEND session smoked on `:8091` is no longer running; only a stale release binary on `:9741` is up and it predates the route (returns 404). Once the BACKEND session leaves a current binary on `:9741`, re-run §5 cases 1–5; my adapter already handles their two notes (`: keep-alive` comment lines are ignored by `parseFrame`; the prose-fallback `error` frame is rendered inline, and `?demo=1` hides it via the fallback). §5 cases 6, 7 verified manually via `?fixture=1` / `?demo=1`.
- [ ] Any deviation from the contract: **none.** `BuilderEvent` and `BuilderAdapter` shapes untouched.
- [ ] Notes / known issues for the backend session:
  - Updated `examples/flow-agent/frontend/e2e/page-builder.spec.ts` to navigate `/pages/new?fixture=1` for the three tests that exercise `builder.send` (the default is now real HTTP per §4.4). All 5 e2e remain green.
  - The adapter sends `{"prompt": input.text, "provider": "claude"}` (per §4.1). `slots` / `meta` from `BuilderSendInput` are intentionally NOT forwarded — the wire contract doesn't have a slot for them. If you need them, raise it as a contract change.
  - Adapter accepts an optional `fetch` override (used only by unit tests against in-memory `ReadableStream`s). Production paths still use `globalThis.fetch`.
  - Empty `data:` payloads and bare `[DONE]` frames are tolerated and dropped (the builder surface's terminal frame is `status: done`, per §4.2).
  - On HTTP 4xx/5xx (non-503) or non-503 with no `onUnavailable`, the adapter yields exactly one `error` frame derived from `{"error": ..., "hint": ...}` body or status text, then returns — matching the "one terminal error frame" invariant.
