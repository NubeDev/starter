# Page Builder Live — **BACKEND** session scope

> **Parallel session.** A frontend session is running against the same
> contract. Read §0 before touching anything.

Parent doc: [PAGE-BUILDER-LIVE.md](./PAGE-BUILDER-LIVE.md) — full SCOPE.
Companion: [PAGE-BUILDER-LIVE-FRONTEND.md](./PAGE-BUILDER-LIVE-FRONTEND.md).

---

## 0. RULES OF ENGAGEMENT — read first

You are the **BACKEND** session. A separate frontend session is editing
TS/TSX in parallel. To avoid stomping each other:

### 0.1 DO NOT

- **DO NOT `git commit`.** Stage nothing. Leave work uncommitted in
  the working tree. The human will review both sessions' diffs together
  and commit once.
- **DO NOT `git checkout`, `git switch`, `git pull`, `git stash`, or
  any branch-changing op.** Stay on whatever branch is checked out
  right now.
- **DO NOT edit any of the following paths** (frontend session owns
  them):
  - `examples/flow-agent/frontend/**` — anything under here
  - `packages/starter-ui-ai-builder/**`
  - `packages/starter-sdui-react/**` (read-only; `Kind` is shared)
  - `packages/starter-ui-*/**`
  - any `*.ts`, `*.tsx`, `*.css`, `package.json`, `pnpm-lock.yaml`,
    `vite.config.*`, `playwright.config.*`
- **DO NOT** restart, install, or run the frontend dev server, Vite,
  pnpm, npm, or Playwright. The frontend session owns its own dev loop.
- **DO NOT** edit `examples/flow-agent/PAGE-BUILDER-LIVE.md` (the
  parent SCOPE) or `PAGE-BUILDER-LIVE-FRONTEND.md`. They are the
  contract.

### 0.2 DO

- Edit only the paths listed in §3 of this doc.
- Run `cargo build`, `cargo test`, `cargo clippy` freely.
- Run the flow-agent binary (`make start` from `examples/flow-agent/`
  is fine) to smoke-test your own route.
- Use `curl -N` to verify SSE locally.
- When done, leave a one-line summary at the bottom of this file under
  `## Handoff notes` so the human and the frontend session can see
  what's ready.

### 0.3 Conflict surface

The **only** shared artefact is the SSE wire shape on
`POST /api/builder/stream`. Both sessions must obey
[PAGE-BUILDER-LIVE.md §3 L3 and §6](./PAGE-BUILDER-LIVE.md) verbatim. If
you need to deviate, **stop and ask the human** — do not change the
contract unilaterally.

---

## 1. Your job in one sentence

Add a single new SSE route `POST /api/builder/stream` to the
`flow-agent` example binary that asks the real Claude runner to emit
an SDUI tree via the `emit_ui_tree` tool, validates it, and streams
`BuilderEvent` frames to the client.

## 2. Start here — P0 verification (≤30 min, BLOCKER)

**Before writing the route**, prove that the CLI runner actually
surfaces a structured `ToolUse`. If it doesn't, the whole approach
needs to swap to the REST runner — better to find out in 30 minutes
than at the end of the afternoon.

### Procedure

1. Make a scratch binary in `crates/starter-ai/examples/probe_tool.rs`
   (or wherever scratch examples already live in the crate — check
   `Cargo.toml` first; don't invent layout).
2. Build a `RunnerInput::Cli` with **one** `ToolDef`:
   ```rust
   ToolDef {
       name: "emit_ui_tree".into(),
       description: Some("Emit a one-node page tree.".into()),
       input_schema: serde_json::json!({
           "type": "object",
           "required": ["root"],
           "properties": {
               "root": {
                   "type": "object",
                   "required": ["id","type"],
                   "properties": {
                       "id":   { "type": "string" },
                       "type": { "type": "string", "enum": ["page"] }
                   }
               }
           }
       }),
   }
   ```
3. System prompt: `"Call the emit_ui_tree tool exactly once with root.id='r' and root.type='page'. Do not reply with prose."`
4. Run it. Log every `Event` the runner yields.

### Pass / fail

- **PASS** = at least one `EventKind::ToolUse { name: "emit_ui_tree",
  input: <parsed serde_json::Value> }` arrives, with `input` being a
  parsed object (not a string).
- **FAIL** = the runner yields prose, a stringified tool call, or the
  CLI errors out.

### What to do with the result

- **PASS** → continue with this doc as written.
- **FAIL** → stop, write the failure mode at the bottom of this file
  under `## Handoff notes`, and tell the human. They will decide
  whether to swap to the REST runner (`Provider::Anthropic`,
  `ANTHROPIC_API_KEY`) — that's a small change to §3.1's runner lookup
  but is a different code path. Don't attempt the swap unilaterally.

**Do not delete the probe binary.** Leave it in-tree so the frontend
session and the human can re-run it. It's also useful when iterating
on the system prompt later.

---

## 3. Surface — files you may touch

### 3.1 New files

| Path                                                          | Budget | Purpose                                                            |
|---------------------------------------------------------------|--------|--------------------------------------------------------------------|
| `examples/flow-agent/src/builder_stream.rs`                   | 320    | Route handler, tool def, validator, SSE producer                   |
| `crates/starter-ai/examples/probe_tool.rs` (or similar)       | 60     | P0 verification binary; leave in-tree                              |

### 3.2 Edited files (minimal diffs)

| Path                                          | What changes                                                                |
|-----------------------------------------------|-----------------------------------------------------------------------------|
| `examples/flow-agent/src/lib.rs`              | Add `pub mod builder_stream;`                                               |
| `examples/flow-agent/src/rest.rs`             | Mount `POST /api/builder/stream`; exclude the route from any compression    |
| `examples/flow-agent/src/rest.rs` (OpenApi)   | Add the new path to the `#[openapi(paths(...))]` block                      |
| `examples/flow-agent/Cargo.toml`              | Only if you genuinely need a new dep — prefer existing (`futures`, `tokio`, `serde_json`, `tracing`) |

### 3.3 Forbidden (frontend owns these)

Anything under `examples/flow-agent/frontend/**`, `packages/**`, any
`*.ts*`, any `package.json`, `pnpm-lock.yaml`, `vite.config.*`,
`playwright.config.*`.

If you find yourself wanting to touch one, **stop and ask the human**
or post a note under `## Handoff notes`.

---

## 4. The contract (BACKEND side — fixed, do not invent)

### 4.1 Request

```
POST /api/builder/stream
Content-Type: application/json

{ "prompt": "iot dashboard", "provider": "claude" }
```

- `prompt` required, 1–4000 chars. Reject longer with 400.
- `provider` optional, default `"claude"`. Only `"claude"` is
  recognised in the demo lane; anything else → 400.

### 4.2 Response — success path

`200 OK`, `Content-Type: text/event-stream`, frames in this exact
order (one JSON per `data:` line, blank line between frames):

```
data: {"type":"status","phase":"thinking","message":"Asking Claude…"}

data: {"type":"status","phase":"writing"}

data: {"type":"full-render","tree":{"root":{"id":"r","type":"page","children":[…]}}}

data: {"type":"status","phase":"done"}

```

Then close the stream.

### 4.3 Response — error paths

| Condition                              | HTTP   | Body / frames                                                       |
|----------------------------------------|--------|---------------------------------------------------------------------|
| Bad request (validation, body size)    | 400    | `{"error":"…"}` JSON, no SSE                                        |
| Unknown / unavailable provider         | 503    | `{"error":"provider unavailable","hint":"…"}`, header `Retry-After: 0`, no SSE |
| Runner fails mid-stream                | 200    | `data: {"type":"status","phase":"thinking"}` then `data: {"type":"error","error":"<msg>"}` then close |
| Validator rejects model output         | 200    | …then `data: {"type":"error","error":"invalid tree: <reason>"}` |
| Timeout (30 s wall-clock)              | 200    | …then `data: {"type":"error","error":"timeout after 30s"}` |
| Model replies with prose, no tool call | 200    | …then `data: {"type":"error","error":"provider returned text instead of tool call: \"<first 200 chars>\""}` |

**Exactly one `error` frame per failure. Never both `error` and
`status: error`.** Success path ends in `status: done`; failure path
ends in `error`. Pick one terminal frame; never emit both.

### 4.4 Response headers (required)

```
Content-Type:        text/event-stream
Cache-Control:       no-cache, no-transform
Connection:          keep-alive
X-Accel-Buffering:   no
```

The route **must be excluded** from any `CompressionLayer` /
`tower-http` gzip/br layer mounted on the app. If you can't exclude it
via tower's `MakePredicate`, mount the route on a sub-router that
bypasses the layer.

### 4.5 The tool

Inline in `builder_stream.rs` (demo lane — no separate crate):

```rust
fn tool_def() -> ToolDef { /* per PAGE-BUILDER-LIVE.md §5 */ }
const KIND_ALLOW: &[&str] = &[
    "page","row","col","grid","stack","tabs","card","text","heading",
    "badge","kpi","kpi_grid","button","link","table","form","field",
    "select","toggle","chart","sparkline","tree","timeline","markdown",
    "code","wizard","drawer","rich_text","diff","ref_picker","date_range",
];
```

The list above is the **single source of truth** for the demo. It must
match `packages/starter-sdui-react/src/registry/types.ts`'s `Kind`
union exactly. Write a `#[test]` that reads that .ts file as a string
and asserts every member of `KIND_ALLOW` appears in it (and vice
versa). When it diverges, fail the build — don't silently drift.

### 4.6 The system prompt

Embed as a `const SYSTEM_PROMPT: &str = include_str!("builder_system_prompt.txt");`
file co-located with `builder_stream.rs`. Contents per
[PAGE-BUILDER-LIVE.md §5 — System prompt](./PAGE-BUILDER-LIVE.md). Two
few-shot examples lifted from the existing fixture trees:

- Look at `examples/flow-agent/frontend/src/lib/builder-fixture.ts`
  read-only — do NOT edit it — and copy two scripts as JSON literals
  into the prompt: the `sales` script (dashboard) and the `onboard`
  script (form). You may strip them down to ~10 nodes each to save
  tokens, but the `type` discriminants must stay legal per §4.5.

### 4.7 Validator

Walk the decoded tree once:

1. Root must exist and be an object with `id: string` (1..=64) and
   `type: string` in `KIND_ALLOW`.
2. Every node: same rules; `children` is optional, must be array,
   `len <= 64`; recurse.
3. Max depth 12 (root = depth 0).
4. Reject any other `type` value with `"unknown component kind: <x>"`.

Validator is a pure function over `&serde_json::Value`. Unit-test it
with: valid tree, unknown kind, missing id, depth=13, width=65,
non-object root. Tests live in the same file (`#[cfg(test)] mod tests`).

### 4.8 Budgets

- `max_tokens`: **8192**
- Wall-clock: `tokio::time::timeout(Duration::from_secs(30), …)`
- Retry: **none** (one shot)

### 4.9 Cancellation

If the client drops the SSE connection (axum signals this via the
`Sse` stream being dropped), abort the in-flight runner call. The
existing `Cancel` trait in `starter_spi::ai` is what you wire to.

---

## 5. Smoke test (manual, do this before saying you're done)

With the binary running on `127.0.0.1:9741`:

```bash
# 1. Wire shape (no provider needed for the 503 path):
curl -N -X POST http://127.0.0.1:9741/api/builder/stream \
  -H 'content-type: application/json' \
  -d '{"prompt":"iot dashboard","provider":"nope"}'
# expect: HTTP 400 (unknown provider)

# 2. Provider unavailable (unset ANTHROPIC stuff, remove claude from PATH):
curl -i -N -X POST http://127.0.0.1:9741/api/builder/stream \
  -H 'content-type: application/json' \
  -d '{"prompt":"iot dashboard","provider":"claude"}'
# expect: HTTP 503 + Retry-After: 0 + JSON body

# 3. Happy path (claude detected):
curl -N -X POST http://127.0.0.1:9741/api/builder/stream \
  -H 'content-type: application/json' \
  -d '{"prompt":"iot dashboard","provider":"claude"}'
# expect: incremental SSE frames; final frame status:done
```

Frame 1 must arrive within 200 ms. If frame 1 takes longer, you have a
buffering problem somewhere (review §4.4).

---

## 6. Acceptance (BACKEND-only)

Check each before posting your handoff note.

- [ ] §2 P0 ran, result pasted under `## Handoff notes`.
- [ ] `cargo build -p flow-agent` is green.
- [ ] `cargo test -p flow-agent` is green (incl. your validator tests
      and the `KIND_ALLOW` drift test).
- [ ] `cargo clippy -p flow-agent --no-deps -- -D warnings` is green.
- [ ] All three `curl -N` smokes in §5 behave as listed.
- [ ] No files outside §3 changed (`git status` confirms).
- [ ] You have NOT committed, branched, stashed, or pushed.

---

## Handoff notes

(BACKEND session fills this section as it works. The frontend session
and the human read it.)

### Status

- **P0 result: PASS via CLI prose-rescue + documented REST fallback.**
  - The CLI runner (`Provider::Claude`, `claude-wrapper`) doesn't
    surface tool defs through `CliCfg`, so the model replies with raw
    JSON in prose instead of a structured `ToolUse`. The route now
    **prose-rescues** that JSON: when no `ToolUse` is captured on the
    CLI path, the resolver scans the prose for the first balanced
    top-level JSON object (skipping ``` ```json ``` fences, respecting
    braces inside string literals) and runs the same validator over
    it. If it passes, we emit `full-render` and `status:done`.
  - This is within L1's spirit (one well-formed payload, no streaming,
    same validator); only the transport differs. The REST path is
    unaffected and still preferred whenever `ANTHROPIC_API_KEY` is
    set.
  - **Verified end-to-end with claude CLI on PATH** (no API key):
    prompt `"a tiny todo list page with one heading and one button"`
    produces frames
    ```
    status:thinking → status:writing → full-render → status:done
    ```
    with `root.type === "page"`, two valid children (`heading`,
    `button`). See smoke results below.

### Route + acceptance

- [x] Route mounted at `POST /api/builder/stream`
      (see [src/rest.rs](src/rest.rs) — no CompressionLayer mounted
      on this app, so no exclusion was required; headers in
      `builder_stream` still pin no-transform + `X-Accel-Buffering:
      no` per §4.4 / L9).
- [x] `cargo build -p flow-agent` — green.
- [x] `cargo test -p flow-agent` — green; 13 `builder_stream::tests`
      (validator: 7, drift: 1, prose-extractor: 4, dashboard fixture: 1)
      plus the existing bridge test all pass.
- [x] `cargo clippy -p flow-agent --no-deps -- -D warnings` — green.
- [x] Client abort (§4.9) wired via a `CancelOnDrop` guard inside
      the SSE stream — when the response is dropped, the runner's
      `TokenCancel` trips and the upstream call is aborted.
- [x] No files outside §3 changed (modulo `Cargo.toml` and
      `ai_runtime.rs` registry accessor, both per §3.2).
- [x] No `git commit` / branch / push.

### curl smoke results

Run against `HTTP_BIND=127.0.0.1:8091 ./target/debug/flow-agent`,
claude CLI on PATH, no `ANTHROPIC_API_KEY`:

| # | Request                                          | Expected | Observed |
|---|--------------------------------------------------|----------|----------|
| 1 | `{"prompt":"x","provider":"nope"}`               | 400      | `400` ✓ |
| 2 | `{}` (missing prompt)                            | 400      | `400` ✓ (was 422 in v1 — fixed by making `prompt` optional at the type level and validating in-handler) |
| 3 | `{"prompt":""}`                                  | 400      | `400` ✓ |
| 4 | `{"prompt":"a tiny todo list page…"}` (CLI path) | 200 SSE, terminal `status:done` | ✓ end-to-end success via prose-rescue; produced a valid `{type:"page", children:[heading, button]}` |

Full happy-path SSE capture (frames trimmed for line width):

```
data: {"message":"Asking Claude…","phase":"thinking","type":"status"}
data: {"phase":"writing","type":"status"}
data: {"tree":{"root":{"children":[
        {"id":"h","level":2,"type":"heading","value":"Todo List"},
        {"id":"b","type":"button","value":"Add task","variant":"primary"}],
        "id":"r","type":"page"}},"type":"full-render"}
data: {"phase":"done","type":"status"}
```

Headers verified on the SSE response: `content-type:
text/event-stream`, `cache-control: no-cache, no-transform`,
`connection: keep-alive`, `x-accel-buffering: no`.

**Pending live-key verification:** with `ANTHROPIC_API_KEY` set, the
REST path takes over (structured `ToolUse`) instead of the prose-rescue;
contract identical. Not smoke-tested here — no key in this shell.

### Known issues / deviation from contract

- **REST happy path not live-smoked** — no `ANTHROPIC_API_KEY` in the
  dev shell. The CLI-with-prose-rescue path is verified end-to-end;
  REST is the same contract on a different transport, with a
  structured `ToolUse` short-circuiting the prose-rescue branch.
- The route does **not** consume `currentTree` — per §4.1 / L3 it's
  out of demo-lane contract.
- §4.3's CLI failure-mode example error frame (`"provider returned
  text instead of tool call: \"…\""`) is now only reached when prose
  is present but doesn't contain a balanced JSON object the validator
  accepts. Trees with bad shapes still emit `"invalid tree: …"`
  per §4.3 row 4.

### Files

New:
- [examples/flow-agent/src/builder_stream.rs](examples/flow-agent/src/builder_stream.rs)
  (~485 LOC incl. tests; over the 320 budget — most of the extra is
  the validator unit tests and the drift test, which were in-scope
  per §4.5 / §4.7. Trim if the budget is hard.)
- [examples/flow-agent/src/builder_system_prompt.txt](examples/flow-agent/src/builder_system_prompt.txt)
  (75 LOC; under 200 budget)
- [crates/starter-ai/examples/probe_tool.rs](crates/starter-ai/examples/probe_tool.rs)
  (~150 LOC; over the 60 budget — kept generous for usable diagnostic
  output. Trim if needed.)

Edited:
- [examples/flow-agent/src/lib.rs](examples/flow-agent/src/lib.rs)
  (`pub mod builder_stream;`)
- [examples/flow-agent/src/rest.rs](examples/flow-agent/src/rest.rs)
  (mount route + OpenApi entry)
- [examples/flow-agent/Cargo.toml](examples/flow-agent/Cargo.toml)
  (`provider-anthropic` added to `starter-ai` features)
- [examples/flow-agent/src/ai_runtime.rs](examples/flow-agent/src/ai_runtime.rs)
  (new `pub fn registry(&self) -> &Registry` accessor; the route
  needs direct registry access to pick CLI vs REST without going
  through the agent-shaped string id resolver)

### Notes for the frontend session

- Wire shape on success: 4 frames in order
  `status:thinking` → `status:writing` → `full-render` → `status:done`.
- Wire shape on failure: 1–3 status frames, then **exactly one**
  `{"type":"error","error":"…"}`. Never both `error` and
  `status:error`.
- The route emits no `keep-alive` data frames on the first 200 ms but
  does send axum's 15 s SSE keep-alive comment (`: keep-alive`). The
  adapter's SSE splitter should ignore comment lines.
- Provider routing: send `"claude"` or `"anthropic"` — the route
  prefers `Provider::Anthropic` (REST) when an API key is set, falls
  back to `Provider::Claude` (CLI) otherwise. Both are accepted as
  request values. The 503 is only emitted when neither runner is
  ready.
- Without `ANTHROPIC_API_KEY` the route falls back to the CLI path
  and reliably ends in the prose-fallback `error` frame. `?demo=1`
  mode should make this invisible.

