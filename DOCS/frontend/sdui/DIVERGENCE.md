# SDUI — Divergence from Rubix

Starter's SDUI is a port of Rubix's SDUI (see
[`rubix-workspace/rubix-agent/docs/design/frontend/SDUI.md`](file:///home/user/code/rubix-workspace/rubix-agent/docs/design/frontend/SDUI.md)).
The IR wire shape is intentionally kept compatible so the ported
renderer / builder / binding engine code largely works as-is.

This file lists **every intentional drift** from Rubix's SDUI. When
starter's IR diverges further, add a row here in the same PR — that
keeps reviewers honest and gives Rubix maintainers a single place to
look when they want to know how the two have moved apart.

[`DOCS/frontend/sdui/SCOPE.md`](./SCOPE.md) is the **normative**
reference for starter; Rubix's SDUI.md is the **origin**, not the
spec. If the two disagree on a wire field, starter's wins for
starter consumers.

## Drifts

### D1 — `form_errors` action response → `diagnostics`

| | Rubix | Starter |
|---|---|---|
| Response variant | `form_errors` | `diagnostics` |
| Shape | `{ errors: { field: message } }` (per-field error map) | `{ items: [{ severity, code, message, field? }] }` (flat list) |
| Severity | error only | `error` / `warning` / `info` |
| Back-compat | Rubix kept `form_errors` deserialising for one release | Starter rejects `form_errors` at the wire — has not shipped |

**Why.** The wider shape covers warnings and info, not just per-field
errors. The flat list with optional `field` covers both global and
inline-by-field cases.

**Migration.** None — starter hasn't shipped. New handlers emit
`diagnostics` only.

### D2 — Render target is `starter-ui-kit` (shadcn), not `@rubix/ui-core`

Rubix's React renderer lives in `rubix-ui-core/src/sdui/` and renders
against Rubix's UI primitives. Starter's
`@nube/starter-sdui-react` renders the same IR against
`@nube/starter-ui-kit`'s shadcn primitives.

**Why.** Starter is shadcn-only via `starter-ui-kit`; importing a
parallel UI primitive library would defeat the theme editor and the
unified styling story.

**Migration.** Component-by-component reimplementation. Same `node.type
→ React component` dispatch table; different render output.

### D3 — Three Rust crates, not one

Rubix ships its IR + binding engine + builder DSL across
`rubix-contracts/ui-ir`, `rubix-agent/crates/dashboard-*`, and
`extension-sdk/sdui-builder` — a split that reflects Rubix's
workspace history rather than a planned dependency surface.

Starter ships them as **three narrow crates** with an explicit
dependency contract:

- `starter-ui-ir` — types, schema, version stamp. No I/O.
- `starter-ui-bindings` — grammar, `EvalContext`, subscription
  planner, `EntityGraph` trait. Depends on `starter-ui-ir`.
- `starter-ui-builder` — typed Rust constructors. Depends on
  `starter-ui-ir` only (not on `starter-ui-bindings` — see
  [SCOPE.md § builder/bindings dependency contract](./SCOPE.md#surface--rust-builder-dsl)).

A consumer authoring pages-as-code from `main.rs` pulls `ir +
builder`; the binding engine ships on the server.

**Why.** A consumer that doesn't need the binding engine shouldn't
compile it. A CLI pretty-printer shouldn't compile the builder.

### D4 — HTTP routes in a separate crate, not in `starter-server`

Rubix's SDUI routes live in `rubix-agent/crates/dashboard-transport`
alongside other transport code. Starter ships them as a standalone
`starter-sdui-routes` crate that the consumer opts into via their
own `Cargo.toml`, never as a dep of `starter-server`.

**Why.** Cargo features on `starter-server` cannot prevent the
underlying crates from being built; the only honest opt-out is a
separate crate. See [SCOPE.md § Surface — Rust (HTTP
routes)](./SCOPE.md#surface--rust-http-routes-opt-in).

### D5 — `EntityGraph` trait, not Rubix's node graph

Rubix's binding engine resolves against Rubix's specific node-graph
implementation (`/`-rooted station, typed kinds, etc.). Starter
defines an `EntityGraph` trait that consumers implement against
whatever they have — a database, a service layer, an in-memory
fixture.

**Why.** Starter is the foundation; pinning to one entity model
would defeat the point.

**Migration.** Ports of Rubix's grammar verbatim; only the source
of children/slots is abstracted. The `$target/child.slot` syntax is
unchanged.

### D6 — `cost_cap`, `session_policy`, and other flow node config

Not an SDUI divergence per se, but ai-builder flows reference
config slots (`cost_cap`, `session_policy`, `slice`) that are
defined by [DOCS/flow](../../flow/scope/SCOPE.md) and
[DOCS/agent](../../agent/SCOPE.md), not by Rubix. Rubix has its own
flow story; starter's flow story is independent.

## Reserved (not drifts yet, but watch)

- **RSQL query grammar**: starter currently inherits Rubix's RSQL
  shape via `starter-ui-builder::rsql()`. If starter's query engine
  ([sdui S-D2](./SCOPE.md#s-d2--rsql-query-engine-ship-a-default-or-require-byo))
  diverges (different operators, different push-down semantics), add
  a row here.
- **Stream sentinel `reason` values**: starter inherits `done | error
  | timeout | gone` from Rubix. If a new reason is added (e.g.
  `auth_revoked`), add a row here.
- **`ir_version` numbering**: starter's `SUPPORTED_IR_VERSION` starts
  at whatever Rubix's was at port time. If starter ships an IR
  variant Rubix doesn't have, the version numbers fork — add a row
  here and note the fork point.

## When this document goes away

Once starter has been independent of Rubix for long enough that
"port from Rubix" is no longer how people think about SDUI — likely
within two consumer adoptions and one IR-version bump — this file
can be retired. At that point [SCOPE.md](./SCOPE.md) is the only
spec; Rubix is referenced (if at all) as historical lineage in the
"Why this exists" section.

The retirement criterion is: the drifts table has grown to the
point that "diff against Rubix" is no longer a useful framing. When
that's true, the file has served its purpose.
