# Workflow — rubix-flow-live-tick-demo

## Sequencing

16 stages across six phases. Order: A+B (upstream state seam + counter + doc rewrites, grouped per operator request) → C (rubix SSE + always-on mounter + verification + test) → D (bundled flow) → E (frontend live view + settings sidebar + e2e) → F (closing). Five REVIEW gates.

This job is **upstream-heavy** for one good reason: the per-node persistent state seam is the missing primitive every future stateful node needs. Counter is the first consumer; debounce, accumulator, rate-limit, last-value-buffer all reuse the trait.

## Per-phase discipline

### Phase A+B — upstream state seam + counter (grouped)

Three stages. A+B.1 lands the trait + impls + ctx + doc together because they're atomic (a partial landing breaks every existing NodeCtx call site). A+B.2 lands counter as the first consumer. A+B.3 rewrites the two stale upstream docs.

1. **The `NodeCtx.state: &'a dyn NodeStateStore` field is non-negotiable in A+B.1.** Every node kind that constructs or accepts a `NodeCtx` must update. Don't try to add it as an optional field via a builder — that complicates every consumer. One commit, one breaking change at the contract layer, all consumers updated in the same commit.
2. **The trait stays IO-free.** `NodeStateStore` itself has no `tokio`, no `sqlx`, no filesystem. Two impls handle the IO. This keeps the trait usable from contexts that can't depend on a runtime.
3. **The two impls run the same test matrix.** `starter-flow/tests/node_state_in_memory_test.rs` and `starter-store-sqlite/tests/node_state_sqlite_test.rs` are structurally identical — parameterise via a `Box<dyn NodeStateStore>` constructor closure if the matrix grows beyond ~6 cases.
4. **`node-state.md` is a peer to `hot-reload.md` + `settings.md`**, not a subordinate. R5 reconciliation goes here; future state-seam updates land here.
5. **The doc rewrites are not optional.** The two stale "Today (Phase 2-5)" sections actively mislead readers. Even if the operator never reads the upstream docs, every future Claude session reading them will be misled. Rewrite to present tense citing line numbers of the actual implementation.
6. **The counter is the simplest possible stateful node.** Resist scope creep — no min/max bounds, no overflow handling (document the i64 limit), no multi-counter modes. Six tests cover the surface.
7. **The `on_redeploy` hook is a 5-line addition to `NodeBehavior`** with a default no-op impl. Counter overrides. Document in `node-state.md` + the rewritten `hot-reload.md`.
8. `cargo test -p starter-flow-spi -p starter-flow -p starter-flow-nodes -p starter-store-sqlite` green per stage; `pnpm --filter @nube/starter-ui-flow typecheck` green after A+B.2.

### Phase C — rubix-side SSE + always-on mounter + integration test

Three stages. C.1 is verification-only (skipped-no-diff if the publish path is already in place). C.2 is the new code. C.3 is the integration test.

1. **C.1 is non-negotiable triage.** If `flow_ops.deploy` doesn't publish through `DefinitionManager`, hot-reload is broken regardless of how good the upstream code is. Verify by grep, commit a fix if needed, skip otherwise.
2. **The SSE registry pattern follows extensions.** A `FlowSubscriptionRegistry` mounted at boot, per-flow `tokio::sync::broadcast` channels. Don't reinvent — read `rubix/crates/rubix-agent/src/routes/` for the extensions SSE pattern and mirror it.
3. **`boot::flow_runtime` generalises `boot::scheduler`.** PR #32's scheduler wired one flow's cron; this generalises to "every deployed flow that has a `trigger.schedule` node". Refactor the existing scheduler module to consume the new one rather than duplicating the wiring.
4. **sqlite path is configurable** but defaults to `~/.rubix/node_state.db`. Don't share the Postgres pool — sqlite is the right substrate for high-frequency small writes (every counter increment).
5. **The integration test runs against real testcontainers PG + a tempdir sqlite.** Drives the full live-tick loop with sub-second cron to keep the test fast. Restart-persistence test drops the runner and reconstructs against the same sqlite file.

### Phase D — bundled tick-counter flow

One stage. Smallest phase.

1. **The YAML lives in `rubix-flows/flows/`** alongside the other bundled flows. `flows_seed` picks it up automatically; no extra wiring.
2. **No MessageKeys.** The counter's output is a slot value, not user-facing text. The log node's `message_template` is a tracing event.
3. **One sanity test confirms the YAML parses.** That's the bar; the live-tick test from C.3 covers behaviour.

### Phase E — frontend live view + settings sidebar

Four stages. The visible piece.

1. **E.1 includes `body_yaml` in `flow_ops.list`'s response.** The minimal backend touch to unblock the live-view route without inventing `flow_ops.get`. Don't fan out further — `flow_ops.get`/`update`/`delete`/`history` are a separate job.
2. **E.1 also lands `flow_ops.kinds`.** The settings sidebar needs per-kind JSON Schema; the endpoint sources from `NodeKindRegistry`. One cheap call cached by react-query.
3. **`useFlowEvents` mirrors `useExtensionEvents`.** Don't invent a new SSE-hook pattern — use the existing one.
4. **Settings sidebar handles primitives only.** `string`/`number`/`boolean`/`enum`. Complex schemas fall back to a JSON textarea. Document the limitation in the route file header.
5. **The Save button calls `flowDeploy`.** The engine's hot-reload picks up the change. No new write verb.
6. **Conflict toast on stale revision.** Surface optimistic-concurrency errors honestly.
7. **The Playwright spec is the operator-runnable proof.** Live tick + hot edit + restart persistence. If it passes, the demo works.

### Phase F — closing

One stage. Three artifacts + PR.

1. **Design doc extension** describes the always-on pattern, links to the three upstream docs.
2. **Session note follows the goals-2-4-3 shape.**
3. **PR title:** `feat(flow): live-tick demo + NodeStateStore upstream + always-on flow runtime`.

## Anti-patterns specific to this job

- **Don't store node state in slot values.** Use the new `NodeStateStore` seam. State and data are different concerns.
- **Don't share the Postgres pool for node state.** sqlite is the right substrate for high-frequency per-node writes. PG holds revisions; sqlite holds live state.
- **Don't add `flow_ops.get` / `update` / `delete` / `history` in this job.** They're a separate CRUD job. This job uses `flow_ops.deploy` for the hot-edit and extends `flow_ops.list` minimally to carry the body.
- **Don't redesign the canvas.** `<FlowCanvas>` already supports `overlay` and `slotValues`. Pass through; don't fork.
- **Don't add WebSocket. SSE is enough.**
- **Don't ship a builtin "fire now" button.** Schedule is the trigger.
- **Don't hand-roll a JSON-Schema form library.** Primitives only (string/number/boolean/enum); textarea fallback for everything else. A future job pulls in `@rjsf/core` or similar if richer schemas become a real need.
- **Don't list paths with brace expansion in handovers.** Trips diff-verify.
- **Don't list a path under Done that the stage didn't modify.**
- **Don't `--no-verify`, don't `--force`.**

## REVIEW gate behaviour

Five gates: A+B↔C, C↔D, D↔E, E↔F, plus the final pre-PR gate inside F. Each commits and pushes the stage(s) that led to it; the gate itself commits nothing.

Each gate's handover must include:

- One-line title per commit.
- Per-stage `cargo test` and `pnpm test/typecheck/e2e` summary.
- For A+B: confirmation that every existing `NodeCtx` call site updated cleanly and `cargo test --workspace` is green.
- For C: one operator-runnable curl-based manual flow demonstrating the live tick.
- For D: one operator-runnable flow demonstrating the bundled flow lands in PG after `make restart`.
- For E: one operator-runnable browser flow (the five-step demo from SCOPE Phase E's gate).
- Any deviation from SCOPE.
- Open Questions evidence where the stage answered one.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
2. `docs` — update `handover.md` for the next stage and the active session doc.
3. `git` — stage the changes, commit with the message `stage N: <one-line title from template.yaml>`, push to `codeless/rubix-flow-live-tick-demo`.

REVIEW gate stages mark `git` as `skipped — gate-only`. C.1 marks `git` as `skipped — verification only` if no fix needed. Never `--force`, never `--no-verify`.

## Hard rules (repeated)

- One verb per file. Rust ≤ 400 lines hard, TS ≤ 200 lines hard.
- Code comments link `docs/design/<area>/README.md` only (rubix-side). Upstream `DOCS/flow/scope/*.md` may be cited from upstream code (existing convention; don't break it).
- No phasing markers in code.
- Upstream-first (R2). NodeStateStore + counter + doc rewrites land before rubix consumes.
- R5 stateless behaviours reconciled via NodeStateStore.
- R6 tests live with the code in the same commit.
- R10 reverse-DNS ids (`starter.flow.counter`).
- R12 observability spans on counter.invoke + flow_events.subscribe.
- R13 cancellation checks.
- No emojis. Comments explain *why*, not *what*.

## References

- `DOCS/flow/scope/SCOPE.md`, `hot-reload.md`, `settings.md` (latter two rewritten in this job).
- `crates/starter-flow/src/definition/{resolver,active,classifier,manager}.rs` — hot-reload implementation.
- `crates/starter-flow-spi/src/{node,event_dto,settings}.rs` — SPI surface.
- `crates/starter-flow-nodes/src/{trigger_schedule,log}.rs` — the demo's other two nodes.
- `crates/starter-store-sqlite/` — home of `node_state.rs`.
- `rubix/crates/rubix-tools/src/flow_ops/{deploy,list}.rs` — backend touch points.
- `rubix/crates/rubix-agent/src/boot/{scheduler,flows_seed}.rs` — existing wiring to generalise.
- `rubix/packages/rubix-client-react/src/hooks/{extensions,use-extension-events}.ts` — SSE hook pattern.
- `packages/starter-ui-flow/src/canvas/FlowCanvas.tsx` — canvas already supports `overlay` + `slotValues`.
- `rubix/docs/sessions/2026-05-25-handover-flow-crud-and-orientation.md` — current handover + codeless runbook.
- `rubix/SCOPE.md`, `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
