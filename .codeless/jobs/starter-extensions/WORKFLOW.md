# Workflow — starter-extensions

How to drive this job. The shape is "extract a kernel that has been
re-implemented twice and finalise the cross-transport contribution
model (R13). Build it in four ordered kernel phases plus four
independent adapter phases — each independently mergeable — so
stopping after any one leaves a working product."

## Sequencing

- Stage 1 is **prose-only**. Resolve the six open questions in
  [SCOPE.md](./SCOPE.md), record under "Decisions", commit. No code.
- Stages 3–5 (Kernel Phase 1) land contracts + SDK + host + the
  first transport adapter (`ext-mcp`) together so the first useful
  unit of work is "a builtin extension is reachable from an MCP
  client." Landing `ext-mcp` here — instead of saving every adapter
  for the adapter phases — proves the R13 contribute-to-adapter
  pattern before any flavour or transport multiplies the surface.
- Kernel Phase 2 (stages 7–8) is the heaviest. Do `ext-supervisor`
  first with the `in_memory_transport()` test harness; only wire
  real process spawning once the state-machine tests pass. Then
  `ext-server` admin routes (admin + event-ring SSE live tail only —
  the REST contribution surface waits for Phase 5).
- Kernel Phase 3 (stage 10) and Kernel Phase 4 (stage 11) are
  independent of each other after Kernel Phase 2. Land Phase 3
  first because UI parity with rubix is the higher-risk fork.
  Phase 4 may be marked `[-]` if no consumer needs WASM day-one.
- Stage 12 is the **kernel-complete REVIEW**. The kernel surface is
  frozen here — adapters do not change the trait.
- Adapter Phases 5/6/7 (stages 13–15) can ship in any order after
  the kernel REVIEW. Each one is one new crate + one new
  `contributes.<x>` block; the runner picks the order that matches
  the consumer's immediate need (Phase 5 first if HTTP is the
  primary surface; Phase 6 first if the host is CLI-first; Phase 7
  any time after kernel).
- Phase 8 (`ext-grpc`) is not a stage — it is a reserved slot
  documented in SCOPE.md. The `same-source-streams-over-four-
  transports` smoke test stays `[skip]` until `starter-grpc` lands.
- Stage 16 (smoke tests) is the merge gate. No phase ships
  individually without its own subset passing; the full 10-test
  sweep gates the final merge.

## Per-stage discipline

- Before any code change in a phase:
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) that the
    stage touches. R1, R2, R3, R7, R13 are the load-bearing ones;
    if a change makes any of them harder to enforce, stop and write
    up the conflict.
  - For kernel stages, read
    [`DOCS/extensions/scope/SCOPE.md`](../../../DOCS/extensions/scope/SCOPE.md)
    §"What each crate / package owns" and
    §"Per-transport contribution mechanics" for the crate being
    added.
  - For adapter stages, re-read R13 + the manifest example in the
    source SCOPE and confirm every new `contributes.*` field
    deserialises under `deny_unknown_fields`.
  - For Phase 3, read the upstream
    `rubix-workspace/extension-ui-sdk` at the pinned SHA the
    decisions log records, and identify the rubix-specific hooks
    (`useNode`, `useSlot`, `useKinds`) that must come out of the
    fork.
- Touch only what the stage names. No drive-by refactors.
- Verify before commit:
  - **Rust**: `cargo check --workspace --all-features --all-targets`,
    then `cargo test -p <touched crate>`, then
    `cargo clippy --workspace --all-targets -- -D warnings`. Plus,
    for adapter stages, an isolation check: build the workspace
    with **only** the new adapter's feature enabled and confirm
    sibling adapter deps (e.g. `axum`, `clap`, `wasmtime`, `tonic`)
    are absent from `cargo tree`.
  - **TS**: `pnpm -F starter-ext-sdk-ts typecheck`,
    `pnpm -F starter-ext-ui typecheck`, plus
    `renderWithExtensionHost` host harness tests.
  - **Cross-cutting**: `cargo deny check` (manifest hygiene, no
    duplicated transitive deps that would break the
    "no React duplication" / WASM consumers).
- Commit only if green. One logical batch per commit; commit
  message stage-tagged: `stage N: <one-line title>`.

## REVIEW gates

Three:

- **After stage 1** — decisions sign-off before any code lands. Six
  open questions (four from the original SCOPE, two from the R13
  rewrite) have downstream effects (streaming-notification
  placement in `ext-spi` shapes every adapter; per-entry auth shape
  shows up in every contribution).
- **After stage 6** — Kernel Phase 1 end-to-end: `hello-builtin`
  loads and is reachable from an MCP client. Phase 2's supervisor
  + Phase 5's REST adapter both land on top of these contracts;
  confirming they are right is cheaper here than after either
  exists.
- **After stage 9** — Kernel Phase 2: the supervisor + admin routes
  are the heaviest single chunk. `extension-survives-host-restart`,
  `crash-loop-is-bounded`, and `capability-violation-rejected-
  logged-counted` smoke tests must pass.
- **After stage 12** — kernel complete. The trait, manifest, and
  SDK surfaces freeze here. Adapter phases (5/6/7) must not
  change them; if a real need surfaces during an adapter phase,
  stop and propose a kernel change explicitly, do not back-door it
  through an adapter.

Write a one-line summary into the handover at each gate. Do not
proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" section filled in for all six open questions; no code changed. |
| 3 | `starter-ext-spi` compiles; depends on `starter-spi` only (R2 grep test passes); `ExtensionId` rejects non-reverse-DNS strings in tests; streaming notification types present alongside `JsonRpcEnvelope`. |
| 4 | `#[derive(Extension)]` reads a fixture `block.yaml` at compile time and emits a dispatch table; a missing handler in the impl is a *compile error in the example*, not a runtime error; `requires!{}`-generated `Ctx` exposes a `Stream<Item = Event>` shape. |
| 5 | Two-phase commit lands all-or-nothing; `ext-mcp` registers `hello-builtin`'s `contributes.tools` entry in `starter-mcp::ToolRegistry`; an MCP client invokes the tool end-to-end; bad-manifest-is-isolated smoke test passes. |
| 7 | `Supervisor` drives `Discovered → Validated → Starting → Running → Stopping → Stopped` plus `Crashed → Failed` on cap-exceeded, using `in_memory_transport()` — no real process spawning yet; streaming notifications forwarded without interpretation. |
| 8 | `hello-process` (same source as `hello-builtin`, one cargo feature flipped) spawns under the supervisor; init handshake verifies manifest content hash; `/extensions/<id>/events` shows state transitions and supports SSE live tail. |
| 10 | Two `examples/hello-ui` panels load into a single host page; `two-extensions-no-React-duplication` smoke test passes; bundles served from `ext-server`'s `GET /extensions/<id>/ui/*`. |
| 11 | `hello-wasm` (same source as `hello-builtin`) instantiates under `ext-wasm`; default-deny holds — an extension without `http_out` cannot reach the network even with `wasi:http` linked. |
| 12 | All four kernel phases shipped; `one-source-three-flavours` smoke test passes; kernel surface frozen — no more changes to trait / manifest / SDK until the smoke-test stage. |
| 13 | `ext-server` REST adapter mounts `contributes.rest` and `POST /tools/<id>` for `contributes.tools`; auth applied by adapter; `streaming: sse` emits `text/event-stream` with heartbeat and retry header; `streaming-response-cancels-promptly` smoke test passes (client disconnect → `stream.cancel` within ~hundred ms). |
| 14 | `ext-cli` adapter registers a `contributes.cli` entry as a `clap::Command`; synchronous dispatch with timeout works for process flavour; `streaming: stdout` renders one event per line. |
| 15 | `ext-workers` tick scheduler fires `contributes.workers` at declared intervals + jitter; per-worker error backoff respected; `testing::tick_now(id)` runs a worker synchronously. |
| 16 | All ten smoke tests pass in CI (Phase 8 streaming test stays `[skip]` until `ext-grpc` lands): one-source-three-flavours, extension-survives-host-restart, crash-loop-is-bounded, capability-violation-rejected, two-extensions-no-React-dup, bad-manifest-isolated, byte-identical-description, streaming-response-cancels-promptly, same-source-streams-over-four-transports, extension-author-zero-extra-deps. |

## Anti-patterns

- Adding a `host_call(method, params)` escape hatch to `Ctx` "just
  for now". R6 — the type system rejects calls the extension did
  not declare; untyped escape hatches are deferred forever, not
  "now".
- Putting any runtime logic in `starter-ext-spi`. R2 — contracts
  crate, depends only on `starter-spi`, zero I/O. The first PR that
  pulls in `tokio` or `axum` or `clap` is wrong.
- Forking the JSON-RPC framing crate into `starter-extensions`. The
  source SCOPE places `starter-jsonrpc-stdio` in the `starter`
  workspace; both `starter-mcp` and `starter-ext-supervisor` import
  it. Duplicate framing means drift.
- A second wire format for streaming. R10 — streaming is layered as
  a notification convention (`stream.event` / `stream.end` /
  `stream.error` / `stream.cancel` tagged with `stream_id`) over
  the **one** stdio channel. If a phase needs a side-channel, write
  a design note first; do not introduce one inside an adapter.
- An adapter performing auth differently from its siblings.
  Authorisation lives on the manifest entry (`require_role` /
  `require_scope`); every adapter applies it the same way before
  invoking the extension. Extensions never check auth themselves;
  a contribution that needs to "check identity" is a contribution
  that needs to declare what it requires.
- A per-transport trait method ("`on_rest_call`, `on_cli_dispatch`,
  `on_mcp_tool_call`"). R1 + R13 — the trait is flavour-agnostic
  and transport-agnostic. Adapters synthesise the transport view;
  the extension implements *one* method per logical capability,
  regardless of which surface reaches it.
- Adding `contributes.<x>` parsing inside an adapter crate. The
  manifest schema is in `starter-ext-spi`; adapters consume it.
  Splitting the schema across crates makes `deny_unknown_fields`
  useless because a typo would parse fine without the adapter loaded.
- Letting an extension write its own streaming events directly to
  a transport object (axum response, stdout, MCP frame). The
  extension returns a `Stream<Item = Event>`; the adapter renders
  it. Anything else couples the extension to a transport.
- Adding supervisor groups (one-for-all, rest-for-one) "to keep the
  door open". R9 — no groups in v0.1; the manifest reserves
  `supervision.group`. Adding groups is additive in the manifest
  later; adding them now bloats the state machine.
- Templating extension descriptions or schemas at runtime. R7 —
  static files in the bundle, byte-identical at load and call time.
  Anti-prompt-injection guarantee.
- Letting the proc-macro emit dispatch from an in-source attribute
  list ("`#[tool(name="weather.current")]`"). R3 — manifest is the
  one source. The macro reads the bundle's `block.yaml` at the
  extension's compile time; the impl matches.
- Lifting `rubix-workspace/rubix-ui-core` into the fork. Source
  SCOPE §"UI package source" is explicit: fork the SDK + the
  federation runtime, lift the UI kit into `starter-ui-kit` in the
  *starter* workspace, never lift `rubix-ui-core`.
- An adapter's default features pulling in another adapter's deps.
  R5 of the parent SCOPE — defaults minimal, opt-in everything
  else. The `cargo tree` isolation check in §"Per-stage discipline"
  guards this; do not silence it.
- Mixing the WASM-host's per-call fuel/memory/deadline caps into
  the manifest. The source SCOPE places them host-side (configured
  by the consumer), not in the bundle.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
   On failure: stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, in the same worktree, so the fresh agent that opens
   the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push to the
   job's branch (`codeless/starter-extensions`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
