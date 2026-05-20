# Scope — flow-agent-example

> Source of truth:
> [`examples/flow-agent/SCOPE.md`](../../../examples/flow-agent/SCOPE.md).
> That file is the full design (hard rules F1–F7, surfaces, file
> budget, F6 visual acceptance checklist). This per-job brief is the
> trimmed version pointing at it. When this file disagrees with the
> source-of-truth SCOPE, that doc wins.

## Goal

Finish the `examples/flow-agent/` example: a flow editor + AI agent
chat product on top of the existing `starter-flow`, `starter-ai`,
`starter-ui-flow`, `starter-ui-chat`, `starter-ui-kit`, and
`starter-client-ts` libraries. The product authors flows visually,
runs them on the engine with a live SSE overlay, and chats with
agents that can invoke flows as tools. SSE + REST only, no auth,
no CLI.

Phase 1 already shipped in commit `83e48e8` (Cargo binary boots,
SQLite migrations, flows + agents CRUD, SPA scaffold typechecks).
This job picks up from Phase 2 of the example SCOPE and lands the
remaining six phases.

## Out of scope

- **Auth, multi-tenant, RBAC.** The example binds to 127.0.0.1 and
  trusts the OS-level boundary (SCOPE F4). No `starter-auth-*` wires
  in.
- **CLI.** No clap binary, no `flow-agent run …` subcommands. The
  product is the web UI (SCOPE F5).
- **Postgres.** SQLite only. The pattern transfers but a Postgres
  variant is a future example.
- **gRPC, WebSocket, MCP transport.** SSE + REST only (SCOPE F3).
- **Versioned flows, undo history beyond in-session.** Optimistic
  locking on `version` is enough; full version history is a
  follow-up.
- **Custom node-kind authoring UI.** Node kinds are registered in
  Rust and surfaced through `BUILTIN_NODE_KINDS` from
  `@nube/starter-ui-flow`.
- **Cloud deploy / Docker.** Localhost demo only.
- **i18n.** English only; `starter-i18n` integration is a separate
  example.
- **Persisted conversation history.** Conversations live in the
  react-query cache for MVP; a `conversations` table is a
  one-migration follow-up if needed.

## Deliverables

1. **Flow editor** (`FlowEditor.tsx`) wired to `<FlowCanvas>` with
   typed slots, palette insert, and `version`-based optimistic
   save/load.
2. **Real flow runs** — `POST /api/flows/{id}/fire` drives a
   `starter-flow` engine instance instead of the Phase 1 stub.
   Terminal status persists in the `runs` table.
3. **Live SSE overlay** — `GET /api/flows/{id}/events` already
   exists; the editor subscribes and renders the run state on the
   canvas in real time.
4. **Agent chat** — `POST /api/agents/{id}/run` (SSE) backed by
   `starter-ai::Registry::with_defaults()`; `<Chat />` from
   `@nube/starter-ui-chat` renders the stream client-side.
5. **Agent-as-tool bridge** — flows registered in the agent's tool
   list (`flow:<id>` entries) are callable as `AiTool`s; agent-fired
   runs emit on the same `EventHub.runs` channel.
6. **Sidebar primitive upstream** — `Sidebar`, `SidebarProvider`,
   `SidebarTree` (nested expand/collapse) added to
   `@nube/starter-ui-kit`. The example consumes it as a normal
   downstream would (per F2 — no patching downstream).
7. **Nested live sidebar in the example** — FlowsTree + AgentsTree,
   driven by react-query + the existing `GET /api/events` SSE
   stream, with cookie/localStorage persistence matching
   `SIDEBAR.md` §1–§3.
8. **F6 visual pass + README** — every box in the example SCOPE's
   F6 acceptance checklist ticked; `examples/flow-agent/README.md`
   documents the two-command boot.

## Constraints

- **R1** (file ≤ 400 lines), **R3** (transport never contains
  domain logic), **R6** (UI packages are zero-I/O) from the
  workspace [`SCOPE.md`](../../../SCOPE.md). The example's domain
  layer lives in `src/domain.rs` + `src/store.rs` + the new
  `src/flow_engine.rs` and `src/ai_runtime.rs`; REST + SSE
  handlers stay thin.
- **F2** (no patching of starter packages downstream) from the
  example SCOPE. If `@nube/starter-ui-kit` is missing a Sidebar
  primitive, **add it upstream in stage 6**, do not inline a copy
  in the example.
- **F3** — REST for CRUD, SSE for streams. No new transports.
- **F4** — no auth wiring. Every route is open.
- **F5** — no CLI.
- **F6** — visual checklist applies in stage 7. Do not paste any
  raw hex colour into a TSX file; use Tailwind tokens from
  `@nube/starter-ui-kit/styles.css`.
- **Per-file budget** in the example SCOPE "File-size budget"
  table holds. If a file approaches the limit, split by concept,
  not by `utils`.

## Open questions

1. **Trigger payload schema for the agent-as-tool bridge (stage 5).**
   Start permissive (`{type:"object",additionalProperties:true}`)
   or infer from the trigger node's config slot? Resolve at the
   stage 5 entry — prefer permissive unless inference is trivial
   from the existing `FlowTopology`.
2. **Provider selection in the chat endpoint (stage 4).** Agents
   declare a `provider` string. Map directly to
   `starter_spi::ai::Provider` enum, or keep it as a free-form
   string and fail with a clear error on unknown values? Resolve at
   stage 4 entry — prefer the enum mapping so the failure is
   compile-time-checked.
3. **Sidebar primitive scope (stage 6).** The full rubix sidebar
   is ~600 lines across several files. Ship a slimmer ≤ 400-line
   `sidebar.tsx` that covers the example's needs (provider, header,
   content, group, group-label, item, tree) and defer
   admin-specific bits. Confirm in handover before writing.

## References

- [`examples/flow-agent/SCOPE.md`](../../../examples/flow-agent/SCOPE.md) — full design.
- [`examples/notes/src/flow_demo.rs`](../../../examples/notes/src/flow_demo.rs) — Phase 5 template for `starter-flow` wiring.
- [`packages/starter-ui-flow/README.md`](../../../packages/starter-ui-flow/README.md) — `<FlowCanvas>`, `NodeKindRegistry`, `RunOverlay`.
- [`packages/starter-ui-chat/README.md`](../../../packages/starter-ui-chat/README.md) — `<Chat>`, `createSseAdapter`.
- SIDEBAR.md (rubix-agent docs) — nested sidebar persistence pattern, referenced by the example SCOPE F7.
- `rubix-ui-core/src/lib/sidebar/` — reference implementation of nested SidebarNavTree to crib structure from in stage 6.
