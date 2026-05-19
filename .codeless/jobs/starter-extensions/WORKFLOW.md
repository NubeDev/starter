# Workflow — starter-extensions

How to drive this job. The shape is "extract a kernel that has been
re-implemented twice. Build it in four phases — each independently
mergeable — so stopping after any one leaves a working product."

## Sequencing

- Stage 1 is **prose-only**. Resolve the four open questions in
  [SCOPE.md](./SCOPE.md), record under "Decisions", commit. No code.
- Stages 3–5 (Phase 1) land the contracts + SDK + host together so
  the first useful unit of work is "a builtin extension loads."
  Land each crate as its own commit so a failure is isolated.
- Phase 2 (stages 7–8) is the heaviest. Do `ext-supervisor` first
  with the `in_memory_transport()` test harness; only wire
  real process spawning once the state-machine tests pass. Then
  `ext-server` and `hello-process` on top.
- Phase 3 (stage 10) and Phase 4 (stage 11) are independent of each
  other after Phase 2. Land in either order; the WORKFLOW assumes
  Phase 3 first because UI parity with rubix is the higher-risk
  fork. Phase 4 may be marked `[-]` if no consumer needs WASM
  day-one — `hello-wasm` does not block merge.
- Stage 12 (smoke tests) is the merge gate. No phase ships
  individually without its own subset of those tests passing; the
  full eight-test sweep gates the final merge.

## Per-stage discipline

- Before any code change in a phase:
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) that the
    stage touches. R1, R2, R3, R7 are the load-bearing ones; if a
    change makes them harder to enforce, stop and write up the
    conflict.
  - For Phase 1 stages, read
    [`DOCS/extensions/scope/SCOPE.md`](../../../DOCS/extensions/scope/SCOPE.md)
    §"What each crate / package owns" for the crate being added;
    the trait surface lives there.
  - For Phase 3, read the upstream
    `rubix-workspace/extension-ui-sdk` at the pinned SHA the
    decisions log records, and identify the rubix-specific hooks
    (`useNode`, `useSlot`, `useKinds`) that must come out of the
    fork.
- Touch only what the stage names. No drive-by refactors. If a real
  bug shows up in starter-spi or starter-server, leave a one-line
  note in the handover and keep going.
- Verify before commit:
  - **Rust**: `cargo check --workspace --all-features --all-targets`,
    then `cargo test -p <touched crate>`, then
    `cargo clippy --workspace --all-targets -- -D warnings`. Each
    of the eight smoke tests is a binary in the workspace; a phase
    is not done until its subset passes.
  - **TS**: `pnpm -F starter-ext-sdk-ts typecheck`,
    `pnpm -F starter-ext-ui typecheck`, plus the
    `renderWithExtensionHost` host harness tests.
  - **Cross-cutting**: `cargo deny check` (manifest hygiene, no
    duplicated transitive deps that would break the
    "no React duplication" / WASM consumers).
- Commit only if green. One logical batch per commit; commit
  message stage-tagged: `stage N: <one-line title>`.

## REVIEW gates

Three:

- **After stage 1** — decisions sign-off before any code lands. The
  four open questions have downstream effects (on-disk convention
  reaches `starter-config`; enable/disable persistence touches DB
  migrations); locking them down first is cheap.
- **After stage 6** — Phase 1 end-to-end: `hello-builtin` loads, the
  "bad manifest is isolated" smoke test passes. Phase 2's
  supervisor lands on top of these contracts; confirming they are
  right is cheaper here than after the supervisor exists.
- **After stage 9** — Phase 2: the supervisor + axum routes are the
  heaviest single chunk in the workspace. The "extension survives
  host restart", "crash loop is bounded", and "capability violation
  is rejected, logged, counted" smoke tests must pass. UI work
  (Phase 3) lands on top of this surface; gating it here protects
  the UI from chasing supervisor bugs.

Write a one-line summary into the handover at each gate. Do not
proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" section filled in for the four open questions; no code changed. |
| 3 | `starter-ext-spi` compiles; depends on `starter-spi` only (R2 grep test passes); `ExtensionId` rejects non-reverse-DNS strings in tests. |
| 4 | `#[derive(Extension)]` reads a fixture `block.yaml` at compile time and emits a dispatch table; a missing handler in the impl is a *compile error in the example*, not a runtime error. |
| 5 | Two-phase commit lands all-or-nothing; the "bad manifest is isolated" smoke test passes against a fixture with one good + one broken extension. |
| 7 | `Supervisor` drives `Discovered → Validated → Starting → Running → Stopping → Stopped` plus `Crashed → Failed` on cap-exceeded, using `in_memory_transport()` — no real process spawning yet. |
| 8 | `hello-process` (same source as `hello-builtin`, one cargo feature flipped) spawns under the supervisor; init handshake verifies manifest content hash; `/extensions/<id>/events` shows state transitions. |
| 10 | Two `examples/hello-ui` panels load into a single host page; the "two extensions, no React duplication" smoke test passes. |
| 11 | `hello-wasm` (same source as `hello-builtin`) instantiates under `starter-ext-wasm`; default-deny holds — an extension without `http_out` cannot reach the network even with `wasi:http` linked. |
| 12 | All eight smoke tests in CI: one-source-three-flavours, extension-survives-host-restart, crash-loop-is-bounded, capability-violation-rejected, two-extensions-no-React-dup, bad-manifest-isolated, byte-identical-description, extension-author-zero-extra-deps. |

## Anti-patterns

- Adding a `host_call(method, params)` escape hatch to `Ctx` "just
  for now". The whole point of the typed-capability `Ctx` (R6) is
  that the type system rejects calls the extension did not declare.
  Untyped escape hatches are deferred forever, not "now".
- Putting any runtime logic in `starter-ext-spi`. R2 — contracts
  crate, depends only on `starter-spi`, zero I/O. The first PR that
  pulls in `tokio` or `axum` is wrong.
- Forking the JSON-RPC framing crate into `starter-extensions`. The
  source SCOPE places `starter-jsonrpc-stdio` in the `starter`
  workspace; both `starter-mcp` and `starter-ext-supervisor` import
  it. Duplicate framing means drift.
- Adding supervisor groups (one-for-all, rest-for-one) "to keep the
  door open." R9 — no groups in v0.1, manifest reserves
  `supervision.group`. Adding groups is additive in the manifest
  later; adding them now bloats the state machine by an order of
  magnitude.
- Templating extension descriptions or schemas at runtime. R7 —
  static files in the bundle, byte-identical at load and call time.
  This is the LLM-tool anti-prompt-injection guarantee; lose it and
  the substrate is unsuitable for codeless's use case.
- Letting the proc-macro emit dispatch from an in-source attribute
  list ("`#[tool(name="weather.current")]`"). R3 — manifest is the
  one source. The macro reads the bundle's `block.yaml` at the
  extension's compile time; the impl matches.
- Lifting `rubix-workspace/rubix-ui-core` into the fork. Source
  SCOPE §"UI package source" is explicit: fork the SDK + the
  federation runtime, lift the UI kit into `starter-ui-kit` in the
  *starter* workspace, never lift `rubix-ui-core` (too coupled to
  the graph model).
- A "registration phase" where the running extension informs the
  host of its tools. R3 again — dispatch is manifest-driven, the
  derive macro generates the table, there is no runtime
  registration.
- Mixing the WASM-host's per-call fuel/memory/deadline caps into
  the manifest. The source SCOPE places them host-side
  (configured by the consumer), not in the bundle. Bundles travel;
  caps stay with the host.

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
