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
`ExtensionBehavior` trait, three packaging flavours (builtin / WASM
/ process), one YAML manifest, one supervisor, one Module Federation
UI runtime. A consumer's product adds two crates and one TS package
to get a load-bearing extension substrate; an extension author
writes one struct + one `block.yaml`.

## In scope

- The seven Rust crates in
  [`DOCS/extensions/scope/SCOPE.md`](../../../DOCS/extensions/scope/SCOPE.md#repo-layout)
  §"Repo layout":
  `starter-ext-spi`, `starter-ext-sdk`, `starter-ext-host`,
  `starter-ext-supervisor`, `starter-ext-wasm` (feature-gated,
  optional), `starter-ext-server`.
- The two TS packages: `starter-ext-ui` (host-side Module Federation
  runtime, fork of `rubix-workspace/extension-ui-sdk`'s `./mf` entry)
  and `starter-ext-sdk-ts` (what UI-extension authors import, fork
  of the same upstream's main entry).
- The `starter-jsonrpc-stdio` crate **in the `starter` workspace**
  (not here) — shared Content-Length-framed JSON-RPC 2.0 plumbing,
  consumed by both `starter-mcp` and `starter-ext-supervisor`. The
  dependency arrow stays inside `starter` because `starter-mcp` is
  the older consumer.
- Four working `examples/hello-*` extensions: one builtin, one
  process, one wasm, one UI panel. The first three share a single
  source with the flavour selected by a cargo feature (R1).
- Eight smoke tests from the source SCOPE §"Smoke tests" — each is
  a merge gate, not a nice-to-have.

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
- Migration of codeless's `plugin.toml` or rubix-agent's `block.yaml`
  consumers. Greenfield design sanity-checked against both, but the
  framework does not bend to accommodate either.

## Hard rules (load-bearing)

Every rule below is enforceable by a simple test or grep. Trip one
and the substrate collapses into "yet another plugin framework"
(R1–R12 in the source SCOPE):

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
  extension's tool/panel/kind ids must be its id or a dotted
  descendant. Reserved prefixes (`sys.*`, `starter.*`) cannot be
  claimed.
- **R5** — Stateless behaviours. `ExtensionBehavior` methods take
  `&self`. State lives in host-provided stores or the supervisor's
  per-extension state machine (read-only to the extension). This
  discipline is what keeps flavours interchangeable.
- **R6** — Capabilities declared (`requires:` macro-checked at
  extension compile time), granted (`capabilities:` in the
  operator's manifest), enforced where the runtime permits
  (WASM-hard, process-advisory at JSON-RPC boundary, builtin-doc).
  The `Ctx` newtype the SDK gives an extension only exposes methods
  for declared categories — calling `ctx.http()` without
  `http_out` requires fails to compile.
- **R7** — Static metadata, never runtime-templated. Tool
  descriptions, schemas, prompt templates, panel labels: files in
  the bundle, read at load time, cached. Anti-prompt-injection
  guarantee for the LLM-tool case codeless cares about.
- **R8** — Default-deny for WASM, advisory-deny at the wire
  boundary for process, explicit-grant for both.
- **R9** — One supervisor, one restart policy, no supervision
  groups in v0.1. Each process extension is its own subtree.
- **R10** — One IPC wire format: stdio JSON-RPC 2.0,
  Content-Length-framed.
- **R11** — UI extensions via Module Federation; shared singletons
  negotiated host-side. UI extensions cannot issue raw `fetch` —
  they consume `useHostClient()` from `@starter/ext-sdk-ts` which
  routes through `starter-client-ts`.
- **R12** — Comments explain why, never what. No session-progress
  chatter. TODOs carry a name or ticket.

## Constraints

- Cargo workspace at `starter-extensions/` root, `pnpm-workspace.yaml`
  for TS packages.
- Cargo features for `starter-ext-sdk` are mutually exclusive
  (`builtin | wasm | process`); enabling two produces a linker
  error by design (R1 enforcement).
- Stdio JSON-RPC framing lives in `starter-jsonrpc-stdio` in the
  **starter workspace**, not here. Both `starter-mcp` and the
  supervisor consume it.
- TS singleton majors enforced at host load time. Mismatch =
  load-time refusal with reason
  `singleton-mismatch: <pkg>@<expected> vs <actual>`, lifecycle
  state `Failed`, other extensions unaffected.
- WASM is feature-gated. Consumers without untrusted-extension
  requirements do not pay for `wasmtime` transitively.
- `starter-ext-server` admin routes (`enable` / `disable`) gated
  behind `Role::Admin` from `starter-spi`. No finer-grained scope
  until a consumer asks.

## Phasing

Mirrors the source SCOPE's §"Phasing" exactly. Each phase ships an
independently mergeable, working result:

- **Phase 1** — `ext-spi` + `ext-host` + `ext-sdk` (builtin only)
  with the `hello-builtin` example loading end-to-end.
- **Phase 2** — `ext-supervisor` + process flavour +
  `ext-server` admin routes with `hello-process` (same source as
  Phase 1, one cargo feature flipped). The heaviest phase.
- **Phase 3** — `ext-ui` + `ext-sdk-ts` forks from
  `rubix-workspace/extension-ui-sdk` with rubix-specific graph
  hooks stripped, plus `hello-ui` rendering a panel in a host slot.
  Independent of Phase 2 — builtin extensions can contribute UI
  without the supervisor.
- **Phase 4** — `ext-wasm` (WASI-p2 on wasmtime), default-deny
  capabilities, `hello-wasm` from the same source. Deferrable if
  no consumer needs it day-one.

Stage 12 (smoke tests) is the merge gate covering all phases.

## Deliverables

- Seven Rust crates per the §"Repo layout" in the source SCOPE.
- Two TS packages (`starter-ext-ui`, `starter-ext-sdk-ts`).
- Four `examples/hello-*` extensions exercising every flavour.
- The eight smoke tests from §"Smoke tests" in the source SCOPE,
  every one of them passing in CI before merge.
- Testing seams shipped per crate (§"Testing seams"):
  `TestRegistry`, `in_memory_transport()`, `MockCtx`, `ephemeral()`,
  `TestApp`, `renderWithExtensionHost`.

## Open questions (resolve in stage 1)

The four open questions in the source SCOPE §"Open questions",
copied with biases the runner records the resolved answer to under
"Decisions" below:

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

Record decisions in this file under "Decisions" before stage 3 (the
first code stage) begins.

## Decisions

(populated in stage 1)

## Cross-cutting checks the runner must keep honest

These are not Codeless's R1–R5 (which apply to codeless's UI
boundary, not this workspace). They are the equivalents for
`starter-extensions`:

- The dependency-arrow test: nothing in `starter-extensions` is
  consumed by `starter` itself. The consumer's binary depends on
  both workspaces; neither depends on the other in reverse.
- The "extension author has zero starter-workspace deps beyond
  `starter-ext-sdk` + `starter-ext-spi` + `starter-spi` public
  types" smoke test, every phase.
- The "LLM-facing description is byte-identical at load and call
  time" smoke test (R7 anti-prompt-injection guarantee), every
  phase that touches the manifest path.
- The mutually-exclusive feature guard on `starter-ext-sdk`
  (linker error on misconfiguration) — guard with a CI matrix that
  builds each example under each enabled feature.
