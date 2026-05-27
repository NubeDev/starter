# Caller identity propagation

> **Tier:** design (canonical). Lifetime: durable. Describes the
> shipped row-1 capability of the extensions north-star scope. The
> plan / "we will" framing lives in
> [docs/scope/extensions-north-star/](../../scope/extensions-north-star/README.md);
> this page describes the surface as it is.

## What

Every JSON-RPC frame the host (or an adapter sitting above the
supervisor) sends to an extension carries a `CallerIdentity` in
its `_meta.caller` sidecar. The extension's per-flavour entry-point
glue extracts the identity from each inbound frame and surfaces it
through `ctx.caller()` so handlers can scope work to the requesting
tenant / user without re-deriving the principal from `params`.

## Why

The north-star proposal's "Rule 3" requires every tenant-scoped
capability handle to refuse a frame whose owning tenant the host
has not authenticated. The ergonomics that make Rule 3 enforceable
in practice — `ctx.warehouse_read().query(template, params)`
"just works" against the calling tenant — depend on a single
identity source that flows through every JSON-RPC frame. This is
that source.

## Wire shape

`CallerIdentity` is a struct in
[`starter-ext-spi::identity`](../../../../starter-extensions/crates/starter-ext-spi/src/identity.rs):

```rust
pub struct CallerIdentity {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub roles: Vec<String>,
    pub request_id: String,
}
```

It rides in an optional `_meta.caller` field on
[`JsonRpcRequest`](../../../../starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs)
and [`JsonRpcNotification`](../../../../starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs):

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/com.acme.chart.render",
  "params": { "rows": [/* … */] },
  "_meta": {
    "caller": {
      "tenant_id": "t-42",
      "user_id": "u-7",
      "roles": ["viewer"],
      "request_id": "req-9c1f"
    }
  }
}
```

A frame without `_meta.caller` represents a host-internal frame
(health pings, `init`, `shutdown`, periodic cron). The SDK reflects
this as `ctx.caller() == None`. Tenant-scoped capability handles
refuse to serve a frame whose caller is `None` or whose
`tenant_id` is `None`.

### Why a sidecar field, not nested in `params`

Nesting under `params._meta` (MCP's convention) couples identity
to the handler's parameter schema — every contributed tool would
have to make room for the reserved key. Lifting identity onto the
envelope keeps `params` exclusively the handler's contract, which
matches how the request id is treated. Adding the field is
backward-compatible: serde defaults the missing case to
`FrameMeta::default()`, and a serialised empty `FrameMeta` never
appears on the wire (the `skip_serializing_if` predicate elides it).

## Host stamping path

The supervisor exposes two ways to attach identity to an outbound
frame, both in
[`starter-ext-supervisor::supervisor`](../../../../starter-extensions/crates/starter-ext-supervisor/src/supervisor.rs):

- `SupervisorHandle::call_as(method, params, caller, timeout)` —
  the request/response dispatch entry adapters use for inbound
  REST/MCP/CLI calls.
- `SupervisorHandle::send_with_caller(envelope, caller)` — stamps
  a pre-built envelope (notifications, raw protocol frames).

Both route through the private `stamp_caller` helper which writes
`_meta.caller` on the envelope. The helper overwrites any
pre-existing `_meta.caller` so the supervisor (or the adapter
immediately above it) is the single source of truth — a child
process cannot launder a different identity by pre-stamping the
envelope it asked to be relayed.

Backward compatibility: the existing `SupervisorHandle::call` and
`SupervisorHandle::send` paths remain. A call through them emits a
frame without `_meta.caller`, which the SDK reflects as a system
frame.

## SDK surface

`ctx.caller()` is always present on every `requires!{}`-generated
`Ctx` newtype — like `events()` and `cancel()`, identity is a
kernel-level concern that does not need a capability gate. The
accessor returns `Option<&CallerIdentity>`; `None` is a
host-internal frame.

The per-flavour entry-point glue is what populates the field:

- **Process** ([`starter-ext-sdk/src/process.rs`](../../../../starter-extensions/crates/starter-ext-sdk/src/process.rs)):
  the dispatch loop extracts `_meta.caller` from each inbound
  frame, calls `CtxInner::with_caller(...)` to produce a per-call
  clone, and wraps it into the per-extension `Ctx` newtype before
  invoking the handler. The clone is cheap — every field in
  `CtxInner` is `Arc`-backed.
- **Builtin** — populated by the host's dispatch table directly
  when it constructs the per-call `CtxInner`. The mechanism lands
  with the host's dispatch-table refactor in row 2.
- **Wasm** — the wasm flavour passes the identity through the
  guest's import surface; lands alongside the first wasm capability
  call.

## Tests

Round-trip and stamping coverage lives next to the code:

- `starter-extensions/crates/starter-ext-spi/src/identity.rs` —
  `CallerIdentity` / `FrameMeta` round-trip and empty-field elision.
- `starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs` —
  envelope serialisation with and without `_meta`.
- `starter-extensions/crates/starter-ext-supervisor/src/supervisor.rs` —
  `stamp_caller` writes the meta block and overwrites a pre-existing
  one.
- `starter-extensions/crates/starter-ext-sdk/src/ctx.rs` —
  `CtxInner::with_caller` sets and clears the caller field.

## Open questions

Tracked in
[docs/scope/extensions-north-star/PROGRESS.md](../../scope/extensions-north-star/PROGRESS.md):

- Where the host resolves `CallerIdentity` from on the inbound
  side (REST `Authorization` header, MCP authn, CLI principal)
  lands per-adapter in subsequent rows and is not part of this
  one. Row 1 ships the wire shape and the propagation primitive;
  adapter wiring is additive.
