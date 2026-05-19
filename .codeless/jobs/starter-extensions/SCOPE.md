# Scope — starter-extensions

> Source of truth: [`DOCS/extensions/scope/SCOPE.md`](../../../DOCS/extensions/scope/SCOPE.md)
> in the starter repo. This file is the per-job brief the runner
> reads before every stage; it is intentionally short. When this
> file disagrees with the source-of-truth SCOPE, that doc wins —
> open an issue and update this file.

## Goal

Build `starter-extensions` as a **sibling workspace** to `starter`,
implementing the extension substrate the third time across the
author's projects (codeless → rubix-agent → starter). One
`ExtensionBehavior` trait, three packaging flavours (builtin / WASM /
process), one YAML manifest, one supervisor, one Module Federation UI
runtime, and **one contribution model that reaches every transport
the host exposes (MCP, REST/SSE, CLI, gRPC, periodic workers, UI)
through small feature-gated adapter crates (R13)**. A consumer's
product opts into the adapters it needs; an extension author writes
one struct + one `block.yaml` and reaches every surface the host has
turned on.

## In scope

- The seven Rust crates **kernel + transport adapters** in
  [`DOCS/extensions/scope/SCOPE.md`](../../../DOCS/extensions/scope/SCOPE.md#repo-layout)
  §"Repo layout":
  - kernel: `starter-ext-spi`, `starter-ext-sdk`, `starter-ext-host`,
    `starter-ext-supervisor`, `starter-ext-wasm` (feature-gated,
    optional)
  - transport adapters (each feature-gated, each optional, each
    additive — adding a new transport does NOT change the trait):
    `starter-ext-server` (REST + admin + UI bundle serving),
    `starter-ext-mcp`, `starter-ext-cli`, `starter-ext-workers`
- A **reserved slot** for `starter-ext-grpc` — documented in the
  workspace's `Cargo.toml` and the manifest schema, but no code
  ships until `starter-grpc` lands in the parent workspace.
- The two TS packages: `starter-ext-ui` (host-side Module Federation
  runtime, fork of `rubix-workspace/extension-ui-sdk`'s `./mf` entry)
  and `starter-ext-sdk-ts` (what UI-extension authors import, fork
  of the same upstream's main entry).
- The `starter-jsonrpc-stdio` crate **in the `starter` workspace**
  (not here) — shared Content-Length-framed JSON-RPC 2.0 plumbing,
  consumed by both `starter-mcp` and `starter-ext-supervisor`.
- Manifest support for every `contributes.*` block defined in the
  source SCOPE: `tools`, `cli`, `rest` (unary + `streaming: sse` +
  `streaming: ndjson`), `grpc` (reserved), `workers`, `ui`. Per-entry
  auth declarations (`require_role` / `require_scope`) honoured by
  the adapter, never by the extension.
- The streaming convention layered over the stdio JSON-RPC channel
  (R10): `stream.event` / `stream.end` / `stream.error` /
  `stream.cancel` notifications tagged with `stream_id`. Same
  convention reused by every adapter; **no new wire format**.
- Five `examples/hello-*` extensions: builtin, process, wasm, ui,
  and a new `hello-streaming` exercising the same `Stream<Item =
  Event>` source across SSE / CLI / MCP transports.
- Ten smoke tests from the source SCOPE §"Smoke tests" — each is a
  merge gate, not a nice-to-have, including the two streaming tests
  added with R13.

## Out of scope

- A marketplace, signed bundles, discovery service, deployment
  rings — distribution is orthogonal.
- Hot-reload (Erlang `code_change`-style). v0.1 enable/disable is a
  clean stop + restart.
- Supervisor groups (one-for-all, rest-for-one). Every extension is
  its own restart unit in v0.1. The manifest reserves the slot.
- cgroups / rlimits for process extensions. WASM has fuel + memory +
  deadline caps; process extensions are assumed trusted enough that
  OS-level isolation is a v0.2 feature behind an explicit threat
  model.
- Multi-tenant identity / signing / attestation.
- A UI design system. `starter-ext-ui` is a federation runtime;
  primitives stay in `starter-ui-kit` (separate crate in the
  `starter` workspace).
- A prompt / agent framework. Extensions contribute `Tool` impls;
  prompt templates and agent loops are consumer concerns built on
  `starter-ai`.
- A job queue / fan-out scheduler. `starter-ext-workers` is periodic
  invocation with backoff — not a queue.
- A second wire format for high-throughput streaming. Streaming is
  a notification convention over the **one** stdio JSON-RPC channel
  from R10. Side-channels are additive when justified by a real
  consumer.
- Authorisation logic inside extensions. Adapters apply auth from
  the manifest before invoking the extension.
- Migration of codeless's `plugin.toml` or rubix-agent's `block.yaml`
  consumers. Greenfield design sanity-checked against both, but the
  framework does not bend to accommodate either.

## Hard rules (load-bearing)

Every rule below is enforceable by a simple test or grep. Trip one
and the substrate collapses into "yet another plugin framework"
(R1–R13 in the source SCOPE):

- **R1** — One trait, three flavours, one source. The same
  `ExtensionBehavior` impl compiles to builtin, wasm or process via
  mutually-exclusive cargo features. A linker error means more than
  one is enabled. No flavour-specific trait methods.
- **R2** — `starter-ext-spi` is the contracts crate and **depends
  only on `starter-spi`**. Every other crate in this workspace
  depends on `starter-ext-spi`. Zero runtime logic, zero I/O.
- **R3** — The manifest is the only source of truth for what an
  extension provides. `block.yaml` parsed with
  `deny_unknown_fields`; descriptions and schemas are static files,
  never templated at runtime. `#[derive(Extension)]` reads
  `block.yaml` at the extension's compile time — missing or extra
  handlers are compile errors in the extension, not runtime errors
  in the host. Process flavour verifies manifest content-hash in
  the init handshake.
- **R4** — Reverse-DNS ids; namespace ownership enforced. An
  extension's tool/panel/kind/cli/rest/worker ids must be its id or
  a dotted descendant. Reserved prefixes (`sys.*`, `starter.*`)
  cannot be claimed.
- **R5** — Stateless behaviours. `ExtensionBehavior` methods take
  `&self`. State lives in host-provided stores or the supervisor's
  per-extension state machine (read-only to the extension).
- **R6** — Capabilities declared (`requires:` macro-checked at
  extension compile time), granted (`capabilities:` in the
  operator's manifest), enforced where the runtime permits
  (WASM-hard, process-advisory at JSON-RPC boundary, builtin-doc).
  The `Ctx` newtype the SDK gives an extension only exposes methods
  for declared categories — calling `ctx.http()` without `http_out`
  in `requires` fails to compile.
- **R7** — Static metadata, never runtime-templated. Tool
  descriptions, schemas, prompt templates, panel labels: files in
  the bundle, read at load time, cached. Anti-prompt-injection
  guarantee for the LLM-tool case codeless cares about.
- **R8** — Default-deny for WASM, advisory-deny at the wire
  boundary for process, explicit-grant for both.
- **R9** — One supervisor, one restart policy, no supervision
  groups in v0.1. Each process extension is its own subtree.
- **R10** — One IPC wire format: stdio JSON-RPC 2.0,
  Content-Length-framed. Streaming is a notification convention
  layered on top, **not a new wire format**.
- **R11** — UI extensions via Module Federation; shared singletons
  negotiated host-side. UI extensions cannot issue raw `fetch` —
  they consume `useHostClient()` from `@starter/ext-sdk-ts` which
  routes through `starter-client-ts`.
- **R12** — Comments explain why, never what. No session-progress
  chatter. TODOs carry a name or ticket.
- **R13** — Extensions contribute to every transport through one
  seam. An extension is not "an MCP tool" or "a REST handler" — it
  is a contributor that the host's transport adapters surface as
  any of: MCP tool, REST route, REST `/tools/<id>` endpoint, CLI
  subcommand, gRPC service, periodic worker, UI panel. **One
  contribution can surface in multiple transports** (e.g.
  `contributes.tools` reaches both MCP via `starter-ext-mcp` and
  REST via `starter-ext-server`'s `/tools/<id>`). Adding a new
  transport later is a new `starter-ext-<transport>` adapter crate
  + a new `contributes.<x>` manifest block. **The trait does not
  change.** Extensions written today keep working when a future
  adapter ships, as long as they do not opt in. Adapters apply auth
  (`require_role` / `require_scope`); extensions never check auth
  themselves.

## Constraints

- Cargo workspace at `starter-extensions/` root, `pnpm-workspace.yaml`
  for TS packages.
- Cargo features for `starter-ext-sdk` are mutually exclusive
  (`builtin | wasm | process`); enabling two produces a linker
  error by design (R1 enforcement).
- Cargo features for each transport adapter are independent and
  additive. A CLI-only consumer must not pull `axum` transitively;
  an MCP-stdio-only consumer must not pull `clap`. Default features
  in `starter-ext-host` are minimal; adapters are opt-in.
- Stdio JSON-RPC framing lives in `starter-jsonrpc-stdio` in the
  **starter workspace**, not here. Both `starter-mcp` and the
  supervisor consume it.
- The streaming notification shape (`stream.event` / `stream.end` /
  `stream.error` / `stream.cancel` tagged with `stream_id`) lives
  in `starter-ext-spi` alongside `JsonRpcEnvelope` so every adapter
  reuses one shape.
- Auth declarations (`require_role`, `require_scope`) on a
  contribution entry are honoured by the adapter that surfaces
  that entry, never by the extension. Extensions receive a
  verified `Principal` from `starter-spi` but do not check it.
- TS singleton majors enforced at host load time. Mismatch =
  load-time refusal with reason `singleton-mismatch: <pkg>@<expected>
  vs <actual>`; lifecycle state `Failed`; other extensions
  unaffected.
- WASM is feature-gated. Consumers without untrusted-extension
  requirements do not pay for `wasmtime` transitively.
- `starter-ext-server`'s admin routes (`enable` / `disable`) are
  gated behind `Role::Admin` from `starter-spi`. No finer-grained
  scope until a real consumer asks.

## Phasing

Mirrors the source SCOPE's §"Phasing" exactly: **four kernel phases
must ship in order; four adapter phases ship independently once
their kernel dependency is satisfied.**

### Kernel phases (ordered)

- **Phase 1** — `ext-spi` + `ext-sdk` (builtin only) + `ext-host` +
  **the first transport adapter `ext-mcp`** + `hello-builtin` reachable
  end-to-end over MCP. The first phase validates the
  contribute-to-adapter pattern in one shot.
- **Phase 2** — `ext-supervisor` + process flavour +
  `ext-server` admin routes (admin + event-ring SSE live tail only;
  REST contribution surface waits for Phase 5) +
  `hello-process` (same source as Phase 1, one cargo feature
  flipped). The heaviest phase.
- **Phase 3** — `ext-sdk-ts` + `ext-ui` forks from
  `rubix-workspace/extension-ui-sdk` with rubix-specific graph hooks
  stripped, plus `hello-ui` rendering a panel in a host slot. UI
  bundle serving from `ext-server`'s `GET /extensions/<id>/ui/*`.
- **Phase 4** — `ext-wasm` (WASI-p2 on wasmtime), default-deny
  capabilities, `hello-wasm` from the same source. Deferrable if no
  consumer needs it day-one.

### Adapter phases (independent, additive)

Each adapter is one new crate + one new `contributes.<x>` block in
the manifest schema. **None of them changes the trait** — that is
the property R13 buys.

- **Phase 5** — `ext-server` REST contribution surface
  (`contributes.rest` as merged `axum::Router` fragments;
  `contributes.tools` as `POST /tools/<id>` — one tool, two
  transports), per-entry auth, schema validation,
  `streaming: sse` and `streaming: ndjson` rendering, prompt
  cancellation on client disconnect.
- **Phase 6** — `ext-cli` adapter — `contributes.cli` registered as
  `clap::Command` impls in `starter-cli::CommandRegistry`,
  synchronous JSON-RPC dispatch with configurable timeout,
  `streaming: stdout` rendering. `examples/hello-cli`.
- **Phase 7** — `ext-workers` adapter — tick scheduler with jitter +
  error backoff per `contributes.workers` entry. Not a job queue —
  no fan-out, no shared queue. Worker state surfaced on
  `GET /extensions/<id>`.
- **Phase 8 (reserved)** — `ext-grpc` adapter — lands when
  `starter-grpc` lands. `contributes.grpc` becomes tonic `Service`
  impls; the `.proto` file is the schema contract. The same-source-
  streams-over-four-transports smoke test stays `[skip]` until this
  phase ships.

Stage 14 (smoke tests) is the merge gate across all eight phases.

## Deliverables

- Seven Rust crates (kernel + four adapter crates) per the
  §"Repo layout" in the source SCOPE; one reserved `ext-grpc` slot.
- Two TS packages (`starter-ext-ui`, `starter-ext-sdk-ts`).
- Five `examples/hello-*` extensions (`-builtin`, `-process`, `-wasm`,
  `-ui`, `-streaming`); the first three share one source with the
  flavour selected by a cargo feature.
- Ten smoke tests from §"Smoke tests" in the source SCOPE — every
  one of them passing in CI before merge (Phase 8 streaming test
  stays `[skip]` until `ext-grpc` lands).
- Testing seams shipped per crate:
  `TestRegistry`, `in_memory_transport()`, `MockCtx`, `ephemeral()`,
  `TestApp`, **`tool_pair()`**, **`cli::dispatch()`**,
  **`workers::tick_now()`**, `renderWithExtensionHost`.

## Open questions (resolve in stage 1)

The four open questions from the source SCOPE §"Open questions",
plus two follow-ups raised by the R13 rewrite. Biases recorded; the
runner writes the resolved answer with a revisit trigger under
"Decisions" below before stage 3 (the first code stage) begins.

1. **JSON-RPC wire schema versioning.** Bias: defer until v0.2 ships
   its first new host method; add a `host_capabilities` field to the
   init handshake then.
2. **Extension bundle on-disk convention.** Bias:
   `$XDG_DATA_HOME/<binary>/extensions/<id>/`, consumer-overridable
   via `starter-config`. Lives in `starter-config`'s defaults
   section, not the extensions workspace.
3. **Admin-endpoint capability set.** Bias: ship behind `Role::Admin`
   in v0.1; defer `Scope::ExtensionManage` until a real consumer
   asks.
4. **`enable`/`disable` persistence model.** Bias: a DB row keyed by
   extension id (small `extensions_state(id PRIMARY KEY, enabled,
   updated_at)` table), not a sidecar `.state.yaml`. Single source
   of truth and survives bundle redeploys.
5. **Streaming notification placement.** Bias: live in
   `starter-ext-spi` alongside `JsonRpcEnvelope` so every adapter
   reuses one shape. Alternative — adapter-local — was rejected
   because it would invite drift across adapters.
6. **Per-entry auth shape.** Bias: `require_role` (matches `Role`
   from `starter-spi`) + `require_scope` (free-form scope strings
   the host's `Authenticator` resolves), enforced by the adapter
   before invocation. Extensions receive a verified `Principal`
   and a typed `Scope` set but do not perform the check.

## Decisions

(populated in stage 1)

## Cross-cutting checks the runner must keep honest

- The **dependency-arrow test**: nothing in `starter-extensions` is
  consumed by `starter` itself. The consumer's binary depends on
  both workspaces; neither depends on the other in reverse.
- The **adapter-isolation test**: enabling only the `ext-cli`
  adapter must not pull `axum`, `wasmtime`, or `tonic` into the
  build graph. Same for the inverse (REST-only consumer pulls no
  `clap`). Guard with a `cargo tree`-based test.
- The **"extension author has zero starter-workspace deps beyond
  `starter-ext-sdk` + `starter-ext-spi` + `starter-spi` public
  types"** smoke test, every phase.
- The **"LLM-facing description is byte-identical at load and call
  time"** smoke test (R7 anti-prompt-injection guarantee), every
  phase that touches the manifest path.
- The **"streaming response cancels promptly"** smoke test —
  client disconnect (SSE close, CLI SIGINT, MCP cancel) must surface
  as `stream.cancel` to the child within a few hundred
  milliseconds; the extension's `Ctx` cancellation token observes
  it.
- The **"same source streams over four transports"** smoke test —
  one `Stream<Item = Event>` handler reachable via SSE on REST,
  line-delimited stdout on CLI, `notifications/progress` on MCP,
  and (Phase 8) gRPC server-streaming. Reshaping for any transport
  must not require extension code changes.
- The **mutually-exclusive feature guard** on `starter-ext-sdk`
  (linker error on misconfiguration) — guard with a CI matrix that
  builds each `hello-*` example under each enabled feature.
