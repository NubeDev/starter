# Page Builder — Live (Claude-backed) SCOPE

Status: **proposed** · Owner: flow-agent · Companion to
[PAGE-BUILDER.md](./PAGE-BUILDER.md) (fixture slice).

References:
- [crates/starter-ai/src/runners/claude.rs](../../crates/starter-ai/src/runners/claude.rs)
- [packages/starter-ui-ai-builder](../../packages/starter-ui-ai-builder)
- [packages/starter-sdui-react](../../packages/starter-sdui-react)
- [packages/starter-ui-ai-builder/src/types/index.ts](../../packages/starter-ui-ai-builder/src/types/index.ts) — `BuilderEvent` source of truth

> Companion docs (existing, **do not edit as part of this work**):
> - [SCOPE.md](./SCOPE.md) — host example contract
> - [PAGE-BUILDER.md](./PAGE-BUILDER.md) — fixture slice (PR #18, already shipped)
> - [DOCS/frontend/sdui/SCOPE.md](../../DOCS/frontend/sdui/SCOPE.md) — wire format
> - [DOCS/frontend/ai-builder/SCOPE.md](../../DOCS/frontend/ai-builder/SCOPE.md) — authoring mode

---

## 0. P0 — verify before writing anything else

Folded in from peer review #1. The whole SCOPE collapses if this step fails.

**Hypothesis:** the `claude` CLI binary, driven through `claude-wrapper`
the way [runners/claude.rs](../../crates/starter-ai/src/runners/claude.rs)
drives it, will surface a structured `ToolUse` (with parsed JSON input)
when fed a `ToolDef` — not a stringified prose echo.

**Test (≤30 min, before any code):**

1. Stand up a one-shot Rust scratch binary that builds a `RunnerInput::Cli`
   with one `ToolDef` (`emit_ui_tree` per §5) and a minimal prompt
   ("Call the emit_ui_tree tool with a one-node page tree.").
2. Run it against the local `claude` binary the Settings page already
   detects.
3. Inspect the `Event` stream. Pass = at least one
   `EventKind::ToolUse { name: "emit_ui_tree", input: <parsed JSON> }`.
4. Fail = prose reply or stringified tool call.

**Outcomes:**

- **Pass** → proceed with this SCOPE as written (CLI runner, L1 tool-call).
- **Fail** → switch to the **REST runner** path. Same SCOPE, swap
  `Provider::Claude` for `Provider::Anthropic` and require
  `ANTHROPIC_API_KEY`. Update L1 wording; nothing else in the contract
  changes. The Settings page already detects the key.

This gate is non-negotiable. Do **not** start P1 until P0 is green and
the result is pasted into the PR description.

---

## 1. One-line summary

Wire `/pages/new` and `/pages/:id/edit` to the **real** Claude runner so a
prompt like "iot dashboard" produces a valid SDUI tree streamed live into
the canvas, with the existing `createFlowAgentBuilderFixture()` retained
behind `?fixture=1` for e2e determinism (and `?demo=1` for silent
fallback on stage).

## 2. Why this exists

The fixture slice (PR #18) shipped the renderer, the R1 patch buffer, the
save/edit round-trip, and the Playwright suite — but the `BuilderAdapter`
is a hard-coded four-script fixture. The backend already has a working
Claude runner (it powers `/api/agents/{id}/run`); nothing connects the
two. End-users see a prompt box that looks like AI but isn't.

This SCOPE closes the gap with the **minimum** surface area:

- one new backend route, in the existing `flow-agent` binary, on the
  existing `AiRuntime` and `Registry`;
- one new TS adapter, in the existing `starter-ui-ai-builder` package,
  behind the existing `BuilderAdapter` seam;
- zero new crates, zero new packages, zero new auth surface (for the
  demo lane; see §13).

## 3. Hard rules (load-bearing)

### L1 — Tool-call output, not free-form JSON

The model **MUST NOT** be asked to free-form-emit a JSON tree. We define
a single tool, `emit_ui_tree`, whose `input_schema` is the SDUI
`UiComponentTree` shape. The runner already supports `ToolDef` + `ToolUse`
(see `starter_spi::ai::{ToolDef, ToolUse}`, exercised by
[agent_bridge.rs](src/agent_bridge.rs)) — **subject to §0 verification.**

Rationale: free-form JSON streams from LLMs are unstable (mismatched
quotes mid-stream, prose preamble, code-fence wrapping). Tool calls are
validated against the schema by the provider and arrive as one
well-formed payload.

Cost: we lose per-node streaming. We compensate at the UX layer (L4).

### L2 — Schema is the single source of truth; codegen direction inverted

**Invert from the first draft.** The canonical schema lives in
`crates/starter-ai-sdui-tool/schema.json` (or, for the demo lane,
inline in `builder_stream.rs`). The TS `Kind` union is **generated from
it**, not the other way round, using
[json-schema-to-typescript](https://www.npmjs.com/package/json-schema-to-typescript).

Why inverted: parsing a TS discriminated union with a regex `.mjs`
script is fragile (review #10). Going JSON → TS uses an off-the-shelf
typed generator, and the runtime validator + the type stay in lockstep
because both derive from the same JSON.

For the **demo lane** (§13) we skip codegen entirely: the kind list is
written once in `builder_stream.rs`, asserted equal to
`packages/starter-sdui-react/src/registry/types.ts`'s `Kind` union by a
single `cargo test` that reads the .ts file and string-matches the
enum members. Replaced by real codegen post-demo.

### L3 — One route, SSE, BuilderEvent wire shape

New route: `POST /api/builder/stream`. Request body (demo lane):

```json
{
  "prompt": "iot dashboard",
  "provider": "claude"
}
```

`currentTree` is **not** in the demo-lane contract (review #7); when
`/edit` actually pipes a current tree into the prompt, it's added back.

Response: `text/event-stream`. The wire shape is the existing
[`BuilderEvent`](../../packages/starter-ui-ai-builder/src/types/index.ts)
TS union, verbatim — no renames, no parallel shape:

```ts
export type BuilderEvent =
  | { type: "full-render"; tree: UiComponentTree }
  | { type: "patch"; targetComponentId: string; subtree: UiComponent }
  | { type: "token-patch"; patch: TokenPatch }
  | { type: "shell-patch"; patch: ShellPatch }
  | { type: "status"; phase: BuilderPhase; message?: string }
  | { type: "error"; error: string };
```

**Error frame contract (review #8 — pick one shape, document it):**
on failure the backend emits **exactly one** `{type:"error", error:"…"}`
frame followed by stream close. It does **not** also emit a
`status: error` frame. The frontend already handles the `error` variant
in [`useBuilder`](../../packages/starter-ui-ai-builder/src/hooks/use-builder.ts).
Success emits `status: done` as the terminal frame.

The route lives in [src/rest.rs](src/rest.rs) alongside `run_agent` and
reuses `RestState`. No new state, no new background tasks.

### L4 — Streaming UX without streaming nodes

Because L1 forces one-shot tool output, we keep the canvas "alive" by
emitting these `BuilderEvent`s on the SSE stream. Timings are
best-effort, not guarantees (review #17):

| Order | Event                                                |
|-------|------------------------------------------------------|
| 1     | `status: thinking` ("Asking Claude…")                |
| 2     | `status: writing` on first runner output (may take s)|
| 3     | `full-render: { tree }` decoded from the tool call   |
| 4     | `status: done`                                       |

The R1 patch buffer is unused on the live path; that's fine — it still
earns its keep on the fixture path. `bufferedPatches` simply stays at 0
in live mode.

### L5 — Fixture stays, behind a query param; `?demo=1` silent fallback

`PageBuilder.tsx` picks the adapter at mount. **Fix from review #12:**
parse the value, don't just `.has()`.

```ts
const params = new URLSearchParams(window.location.search);
const useFixture = params.get("fixture") === "1";
const demoMode   = params.get("demo") === "1";
const adapter = useMemo(() => {
  if (useFixture) return createFlowAgentBuilderFixture();
  return createHttpBuilderAdapter({
    url: "/api/builder/stream",
    onUnavailable: demoMode
      ? () => createFlowAgentBuilderFixture()  // silent stage fallback
      : undefined,
  });
}, [useFixture, demoMode]);
```

The existing Playwright suite appends `?fixture=1` to every nav so e2e
stays deterministic and offline. A separate opt-in `pnpm e2e:live` job
(post-demo) runs a single smoke against the live path.

`?demo=1` is the stage-safety lane (review #18): if the backend returns
503, the adapter swaps to the fixture **silently** instead of showing
the audience a "Use offline fixture" link.

### L6 — Provider unavailable returns 503 + Retry-After

`POST /api/builder/stream` returns **503 Service Unavailable** with
header `Retry-After: 0` and body
`{"error":"provider unavailable","hint":"..."}` if
`AiRuntime::list_providers()` reports the requested provider as
`available: false`. (Review #6: provider down is infrastructure, not a
malformed request → 503, not 422.)

The frontend renders the hint inline in the transcript in normal mode;
in `?demo=1` mode it silently swaps to the fixture per L5.

### L7 — Hard budget: no retry, hard timeout (review #11)

The route applies:

- `max_tokens = 8192` on the runner call (review #2 — 4096 truncates
  Grafana-style trees);
- a 30-second wall-clock cancellation (`tokio::time::timeout`);
- **no retry.** Retry-within-timeout interacts badly; a slow first
  attempt eats the second's budget. Demo lane is no-retry. Post-demo a
  separate retry-without-timeout wrapper can sit above this.

No agentic loop — the model gets one shot to emit the tool call; if it
refuses or text-replies, we emit a single `error` frame with the
model's text as the message.

### L8 — Validation gate before emitting `full-render`

After the tool call returns, the backend:

1. Deserialises into `serde_json::Value`.
2. Walks the tree and rejects any node whose `type` is not in the
   schema-generated `Kind` allow-list.
3. Rejects trees deeper than 12 levels or wider than 64 children at any
   node.

**Pre-deployment check (review #13):** run this validator over every
script in `createFlowAgentBuilderFixture()` as a `cargo test`. If any
fixture tree trips the limits, raise the limits or fix the fixture
**before** P1 lands; otherwise the validator silently breaks the
offline path too.

Failures emit a single `error` frame with a one-line reason. The
frontend shows the reason and leaves the previous tree intact.

### L9 — SSE buffering must be defeated (NEW, review #4)

Without this the demo goes dead the moment anything proxies the route.

**Response headers on `/api/builder/stream` (required):**

```
Content-Type:        text/event-stream
Cache-Control:       no-cache, no-transform
Connection:          keep-alive
X-Accel-Buffering:   no
```

**Server side:** the route is **excluded** from any
`CompressionLayer`/`gzip`/`br` middleware. Document the exclusion at
the `Router::route` call site in [src/rest.rs](src/rest.rs).

**Vite dev proxy:** ensure `server.proxy['/api']` does not buffer.
Smoke with `curl -N http://localhost:9742/api/builder/stream` and
watch for incremental frames.

### L10 — Client abort wired to the prompt box (NEW, review #15)

`useBuilder` already exposes `cancel()` (which aborts the adapter's
`AbortController`). `PageBuilder.tsx` **must** call it on:

- composer submit while `phase !== "idle"` (new prompt cancels old),
- route unmount,
- explicit Stop button (already in the transcript component).

Without this, fast re-typing produces two concurrent SSE streams racing
last-write-wins on the canvas.

## 4. Surfaces

### 4.1 New files (**full plan** — see §13 for demo-lane subset)

| Path                                                              | Purpose                                                            | Budget |
|-------------------------------------------------------------------|--------------------------------------------------------------------|--------|
| `crates/starter-ai-sdui-tool/Cargo.toml`                          | New micro-crate: the `emit_ui_tree` `ToolDef` + validator          | —      |
| `crates/starter-ai-sdui-tool/src/lib.rs`                          | `tool_def()`, `validate(&Value) -> Result<UiComponentTree, ...>`   | 250    |
| `crates/starter-ai-sdui-tool/schema.json`                         | Canonical schema (TS `Kind` generated from this — §L2)             | —      |
| `crates/starter-ai-sdui-tool/src/system_prompt.txt`               | Versioned prompt + 2 few-shots (review #3)                          | 200    |
| `scripts/gen-sdui-types.mjs`                                      | `schema.json` → TS via `json-schema-to-typescript`                  | 60     |
| `examples/flow-agent/src/builder_stream.rs`                       | Route handler + SSE producer; calls `Registry::get(Provider::...)` | 320    |
| `packages/starter-ui-ai-builder/src/adapters/http.ts`             | `createHttpBuilderAdapter({ url, onUnavailable? })`                | 200    |
| `packages/starter-ui-ai-builder/src/adapters/http.test.ts`        | Unit tests against an in-memory `ReadableStream`                   | 150    |
| `examples/flow-agent/tests/builder_stream_smoke.rs`               | Backend integration test against a stub runner                     | 200    |
| `examples/flow-agent/frontend/e2e/page-builder-live.spec.ts`      | One live-path smoke, skipped unless `LIVE_E2E=1`                   | 80     |

### 4.2 Edited files

| Path                                                              | What changes                                                       |
|-------------------------------------------------------------------|--------------------------------------------------------------------|
| `examples/flow-agent/src/rest.rs`                                 | Mount `/api/builder/stream`; add OpenApi entry; exclude from compression |
| `examples/flow-agent/src/lib.rs`                                  | `mod builder_stream;`                                              |
| `examples/flow-agent/Cargo.toml`                                  | Add `starter-ai-sdui-tool` dep                                     |
| `examples/flow-agent/frontend/src/pages/PageBuilder.tsx`          | Adapter selection per L5 (fixed `?fixture=1` check); abort wiring per L10 |
| `examples/flow-agent/frontend/vite.config.ts`                     | `/api` proxy no-transform per L9                                   |
| `packages/starter-ui-ai-builder/src/adapters/index.ts`            | Re-export `createHttpBuilderAdapter`                               |
| `packages/starter-ui-ai-builder/src/index.ts`                     | Public surface for the new adapter                                 |
| `examples/flow-agent/frontend/playwright.config.ts`               | Add `e2e:live` project, gated by env (post-demo)                   |
| `examples/flow-agent/Makefile`                                    | `make gen-sdui-types` target wired into `make build` (post-demo)   |
| `examples/flow-agent/PAGE-BUILDER.md`                             | One sentence linking here + "use `?fixture=1` for offline mode"    |

### 4.3 Untouched (explicitly)

- `crates/starter-ai/src/runners/claude.rs` — no changes; go through
  the existing `Registry`/`AiRunner` seam.
- `crates/starter-spi/src/ai/` — `ToolDef`/`ToolUse` already exist; no
  SPI changes.
- `packages/starter-sdui-react` — only the existing `Kind` export is
  consumed; no renderer changes.

## 5. The contract: `emit_ui_tree`

This block is **illustrative** (review #9). The canonical version is
`crates/starter-ai-sdui-tool/schema.json` (full plan) or the inline
constant in `builder_stream.rs` (demo lane).

Schema is **inlined** (no `$ref`/`$defs`) per review #16 to dodge
provider tool-schema quirks. The `Node` recursion is expressed by
inlining `children` as `array of object` and letting the validator do
the recursive shape check Rust-side.

```jsonc
{
  "name": "emit_ui_tree",
  "description": "Emit exactly one SDUI page tree that renders the user's request. Do not include prose; the page tree IS your reply.",
  "input_schema": {
    "type": "object",
    "required": ["root"],
    "properties": {
      "root": {
        "type": "object",
        "required": ["id", "type"],
        "properties": {
          "id":       { "type": "string", "minLength": 1, "maxLength": 64 },
          "type":     { "type": "string", "enum": ["page","row","col","grid","stack","tabs","card","text","heading","badge","kpi","kpi_grid","button","link","table","form","field","select","toggle","chart","sparkline","tree","timeline","markdown","code","wizard","drawer","rich_text","diff","ref_picker","date_range"] },
          "children": { "type": "array", "maxItems": 64 },
          "slots":    { "type": "object" }
        },
        "additionalProperties": true
      }
    }
  }
}
```

### System prompt (committed to `system_prompt.txt`)

Header paragraph + **two few-shot exchanges** (review #3) lifted
verbatim from `createFlowAgentBuilderFixture()` so there's zero drift
between the demo prompt and the offline reference. The few-shots are:

1. **Dashboard** — sales KPI grid + sparkline (from the `sales` fixture).
2. **Form** — onboarding two-field form + submit (from the `onboard`
   fixture).

Prompt header (paraphrased):

> You are a UI builder. The user describes a page; you call the
> `emit_ui_tree` tool exactly once with a complete SDUI tree that
> renders that page. Every node has a unique short `id` (≤ 8 chars to
> save tokens) and a `type` from the allowed enum. Prefer `stack` and
> `grid` for layout, `kpi_grid` for dashboards, `chart`/`sparkline`
> for time-series, `table` for tabular data, `heading` + `text` for
> prose. Never invent component types. Never wrap output in markdown
> or prose; the tool call is the entire reply.
>
> Two examples follow.

(The ≤ 8-char id hint addresses review #2 token pressure.)

### Prompt caching (review #14)

If the provider exposes prompt caching cheaply (Anthropic REST does
via `cache_control: { type: "ephemeral" }` on the system block + tool
schema), set it. If the CLI runner doesn't expose the header, skip it
for the demo — don't block on it.

## 6. SSE wire shape (exact)

Frames are SSE `data:` lines, one JSON object per line, separated by
`\n\n`. Names match the existing TS `BuilderEvent` union verbatim.

Success:

```
data: {"type":"status","phase":"thinking","message":"Asking Claude…"}

data: {"type":"status","phase":"writing"}

data: {"type":"full-render","tree":{"root":{"id":"r","type":"page","children":[…]}}}

data: {"type":"status","phase":"done"}

```

Error (one frame, no `status: error` per L3):

```
data: {"type":"status","phase":"thinking"}

data: {"type":"error","error":"provider returned text instead of tool call: \"I'd be happy to help …\""}

```

## 7. Phases (delivery order, **full plan**)

Each phase mergeable on its own; tree stays green.

### P0 — Verify CLI tool-use (§0)

Gate. Don't start P1 until pasted into a PR.

### P1 — Schema canon + Rust tool crate
- `crates/starter-ai-sdui-tool/schema.json` written by hand.
- `scripts/gen-sdui-types.mjs` regenerates the TS `Kind` from it.
- `tool_def()`, `validate(&Value)`. Unit tests: valid tree, unknown
  kind, too deep, too wide, missing id.
- **Pre-deploy gate (L8):** test that runs the validator over every
  fixture tree.
- CI: `make gen-sdui-types && git diff --exit-code` catches drift.

### P2 — Backend route
- `src/builder_stream.rs`: route handler, SSE producer with L9 headers,
  L7 timeout, L8 validator, L6 503-on-unavailable.
- `tests/builder_stream_smoke.rs`: stub `AiRunner` emits a canned
  `ToolUse`; asserts wire shape end-to-end. **No live Claude in CI.**

### P3 — Frontend adapter
- `adapters/http.ts`: `createHttpBuilderAdapter({ url, onUnavailable? })`.
  Hand-rolled SSE splitter (no new deps). Honors `AbortSignal`.
- Unit tests against in-memory `ReadableStream`: each `BuilderEvent`
  variant + abort mid-stream + 503 + error frame.

### P4 — Wire into PageBuilder + docs
- `PageBuilder.tsx`: L5 adapter selection (fixed param parsing), L10
  abort wiring.
- `vite.config.ts`: L9 proxy headers.
- `PAGE-BUILDER.md` cross-link.

### P5 — Live e2e + provider-unavailable UX
- `e2e/page-builder-live.spec.ts` gated by `LIVE_E2E=1`. One test:
  "iot dashboard" → `root.type === "page"` AND ≥ 1 `kpi_grid` node,
  within 30 s (tightened per review #20).
- Inline transcript surface for 503 in normal mode; silent swap in
  `?demo=1` mode.

## 8. Acceptance criteria

A reviewer verifies each by hand.

- [ ] §0 verification result is in the PR description.
- [ ] `make build` regenerates types and `git diff` is empty.
- [ ] `cargo test -p starter-ai-sdui-tool` passes (incl. fixture-tree
      cases per L8).
- [ ] `cargo test -p flow-agent --test builder_stream_smoke` passes
      with a stub runner.
- [ ] `pnpm --filter @nube/starter-ui-ai-builder test` covers each
      `BuilderEvent` variant + abort + 503 + error frame.
- [ ] `pnpm --filter flow-agent-frontend e2e` (offline, fixture) still
      passes — no regression to PR #18.
- [ ] `curl -N http://localhost:9742/api/builder/stream …` shows
      frames arriving incrementally (L9 smoke).
- [ ] With a provider available, `/pages/new` + "iot dashboard" + Send:
      - within 1 s the transcript shows "Asking Claude…",
      - within 30 s the canvas renders a non-empty tree,
      - `root.type === "page"`, ≥ 1 `kpi_grid` node, zero "Unknown
        component" placeholders.
- [ ] With no provider configured, normal mode shows the 503 hint
      inline within 200 ms; `?demo=1` silently swaps to fixture.
- [ ] Re-typing mid-stream cancels the prior request (L10 — observe a
      single in-flight SSE in DevTools).
- [ ] `pnpm e2e:live` (opt-in) passes against the real backend.

## 9. Non-goals (explicit)

- Multi-turn refinement ("now add a CPU chart"). First iteration is
  one-shot per prompt.
- Streaming individual nodes from the model.
- Model selection UI. Provider is fixed to whatever §0 picked.
- Persisting transcripts server-side.
- New auth (F4 holds).
- OpenAI/Anthropic-REST runners beyond the §0 fallback path.

## 10. Risk register

| Risk                                                              | Likelihood | Mitigation                                              |
|-------------------------------------------------------------------|-----------|----------------------------------------------------------|
| CLI runner doesn't emit structured `ToolUse`                      | **high**  | §0 gate; fall back to REST runner before any code        |
| Tool-call payload truncated at 4096 tokens                        | medium    | L7 bumps to 8192; prompt asks for short ids              |
| Claude refuses tool call and replies with prose                   | medium    | Detect, emit single `error` frame with prose text        |
| `Kind` union drifts                                               | low       | L2 codegen + CI `git diff --exit-code`                   |
| Live e2e flaky against the real model                             | high      | Opt-in `LIVE_E2E=1`; never blocks default CI             |
| 5–10 s tool-call wait feels dead                                  | medium    | L4 `status: thinking → writing` flip + spinner            |
| SSE buffered by proxy / compression                               | **high**  | L9 headers + compression exclusion + Vite proxy config   |
| Provider rejects `$ref`/`$defs` schema                            | low       | §5 inlines `Node`; smoke in P2                            |
| Mid-stream retype races canvas                                    | medium    | L10 abort wiring                                          |
| Validator silently breaks fixture                                 | low       | L8 pre-deploy test over fixture trees                    |

## 11. File-size budget summary

Full plan total (corrected from review #19): ~1,460 LOC new + ~180
edited. Demo lane (§13): ~480 LOC new + ~60 edited. Within the
400-line per-file workspace rule (largest single new file:
`builder_stream.rs` at 320).

## 12. Open questions (resolve before P1)

1. **Crate location** — keep `starter-ai-sdui-tool` separate (default),
   or fold behind a feature flag inside `starter-ai`? Separate lets a
   future MCP server depend on it without runners.
2. **System prompt review** — second pair of eyes on `system_prompt.txt`
   before P1?
3. **Generation policy** — nudge towards the existing
   `starter.ai-builder.dashboards` skill bundle, or stay generic?
   Default: generic; skill-aware generation is a follow-up.

---

## 13. DEMO LANE (ship tomorrow)

The full plan above is the post-demo target. For the **demo tomorrow**,
collapse to the minimum that proves the loop end-to-end.

### What's IN the demo lane

| Path                                                              | Notes                                                              | Budget |
|-------------------------------------------------------------------|--------------------------------------------------------------------|--------|
| `examples/flow-agent/src/builder_stream.rs`                       | Route + inline `tool_def()` + inline `KIND_ALLOW` + validator + SSE | 320    |
| `packages/starter-ui-ai-builder/src/adapters/http.ts`             | Adapter with abort + 503 → `onUnavailable` hook                    | 180    |
| `examples/flow-agent/frontend/src/pages/PageBuilder.tsx` (edit)   | L5 selection (fixed `?fixture=1`/`?demo=1`) + L10 abort wiring     | ~40    |
| `examples/flow-agent/src/rest.rs` (edit)                          | Mount route + compression exclusion                                | ~15    |
| `examples/flow-agent/frontend/vite.config.ts` (edit)              | Proxy no-transform per L9                                          | ~5     |

**Total: ~480 LOC new + ~60 edited.** One PR, one afternoon.

### What's OUT of the demo lane (deferred)

- The `starter-ai-sdui-tool` crate, `schema.json`, `gen-sdui-types.mjs`,
  CI drift gate.
- `builder_stream_smoke.rs`, `http.test.ts`, `page-builder-live.spec.ts`.
- L2 codegen (replaced by the cargo string-match drift test).
- Prompt caching unless the runner exposes it for free.

### Demo-lane hard rules (subset of §3, NO loosening)

- L1 tool-call (§0 verified first)
- L3 SSE shape (one `error` frame, not two)
- L4 `thinking → writing → full-render → done`
- L5 `?fixture=1` and `?demo=1` parsing (fixed)
- L6 503 + `Retry-After: 0`
- L7 8192 tokens, 30 s timeout, **no retry**
- L8 validator (inline `KIND_ALLOW`, depth 12, width 64)
- L9 anti-buffering headers + compression exclusion + Vite proxy
- L10 abort on resubmit / unmount / Stop

### Demo-lane acceptance (subset of §8)

- [ ] §0 verification posted.
- [ ] `curl -N` shows incremental frames.
- [ ] `pnpm e2e` (offline fixture) still green — no regression.
- [ ] On stage: `?demo=1` URL; "iot dashboard" produces a tree with
      `root.type === "page"` AND ≥ 1 `kpi_grid` within 30 s; if the
      backend trips, fixture takes over silently.
- [ ] Mid-stream retype cancels the prior request (single SSE in
      DevTools).

### Post-demo immediate follow-up PR

- Promote inline tool-def + `KIND_ALLOW` to `starter-ai-sdui-tool`
  crate (P1).
- Add tests (`builder_stream_smoke.rs`, `http.test.ts`).
- Codegen (`gen-sdui-types.mjs`) + CI drift gate.
- Live e2e job.
- `currentTree` pipe-through for `/pages/:id/edit`.
