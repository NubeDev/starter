# `NodeStateStore` — per-node persistent state

> Stage A+B.1 of the rubix-flow-live-tick-demo job. Lands the SPI seam
> that lets a flow node carry a small amount of durable state across
> restarts without the engine growing a generic blob-on-the-side.

The canonical motivating case is the demo counter node: every 5 s
trigger increments a value; the value must survive a `rubix-agent`
restart so the operator's live `/flows/<id>` SSE stream keeps counting
from where it left off rather than resetting to zero.

## R5 reconciliation — why this is a chokepoint, not a generic KV

SCOPE rule **R5** — "every persistence surface routes through exactly
one chokepoint" — already protects the run/flow/session/skill stores.
Nothing in those three stores fits the counter use case: it is not
session state, it is not run state (a single counter spans many runs),
and it is not flow-definition data.

We resist the temptation to bolt a free-form `kv` table onto the
engine. Instead we add a *typed* seam scoped exactly to a node:
`(flow_id, node_id, key)`. The narrowness is the point:

- the engine can reason about a node's state when the flow revision
  changes (the `reset_on_redeploy` semantics below),
- the SPI surface stays small enough that two impls (in-mem + sqlite)
  fit on a screen, and
- every call site routes through `NodeCtx.state` — there is one place
  to grep when an audit needs to know "what state does this node
  persist?".

## Keying scheme

```
NodeStateKey {
    flow_id: FlowId,      // reverse-DNS, e.g. "rubix.flows.live_tick"
    node_id: NodeId,      // reverse-DNS, e.g. "rubix.nodes.counter"
    key:     String,      // opaque per-node, e.g. "count"
}
```

Rules:

- `flow_id` and `node_id` use the same reverse-DNS validation as every
  other flow identifier (see `crates/starter-flow-spi/src/node.rs ::
  validate_reverse_dns`).
- `key` is **opaque to the store**. Node bodies pick their own
  convention; the counter uses the single key `"count"`. Multi-field
  state should be either separate keys (cleaner) or a serialised
  struct under one key (terser).
- `key` is capped at 256 bytes (`NodeStateKey::MAX_KEY_BYTES`).
- The value payload is capped at 64 KiB
  (`NodeStateValue::MAX_VALUE_BYTES`). The cap is enforced at the
  trait boundary so both impls behave identically under load — a node
  that wants more than 64 KiB of state is misusing the seam and should
  reach for a `RunStore` checkpoint or a domain table instead.

## API

```rust
#[async_trait]
pub trait NodeStateStore: Send + Sync {
    async fn get(&self, key: &NodeStateKey) -> Result<Option<NodeStateValue>, NodeStateError>;
    async fn put(&self, key: &NodeStateKey, bytes: Vec<u8>) -> Result<u64, NodeStateError>;
    async fn cas(&self, key: &NodeStateKey, expected: u64, bytes: Vec<u8>) -> Result<u64, NodeStateError>;
    async fn delete(&self, key: &NodeStateKey) -> Result<(), NodeStateError>;
}
```

- `get` returns `Ok(None)` when no row exists.
- `put` is unconditional. First write of a key returns version `1`;
  every subsequent overwrite returns `previous + 1`.
- `cas` is optimistic. `expected = 0` means "no row exists yet";
  every other value of `expected` must match the current version
  exactly. On mismatch the store returns `NodeStateError::CasMismatch
  { expected, actual }` without mutating state. `actual` is `None` if
  the row is currently absent.
- `delete` removes the row if present; deleting an absent row is a
  no-op (returns `Ok(())`).

`NodeCtx` gains a borrow:

```rust
pub struct NodeCtx<'a> {
    pub run:    RunId,
    pub node:   &'a NodeId,
    pub cancel: &'a dyn Cancel,
    pub skill:  &'a SkillSelection,
    pub state:  &'a dyn NodeStateStore,   // ← added stage A+B.1
}
```

The borrow is the load-bearing API addition of the stage: every node
body now has a stable place to reach for per-instance state without
the engine handing out a free-form `dyn Any`.

## CAS semantics

The counter use case looks like this:

```rust
loop {
    let current = ctx.state.get(&key).await?;
    let (expected, next) = match current {
        None => (0, 1u64),
        Some(v) => (v.version, u64::from_le_bytes(v.bytes.try_into()?) + 1),
    };
    match ctx.state.cas(&key, expected, next.to_le_bytes().to_vec()).await {
        Ok(_) => break,
        Err(NodeStateError::CasMismatch { .. }) => continue, // racer beat us
        Err(other) => return Err(other.into()),
    }
}
```

The bounded retry is the operator's safety net: if two propagator
ticks ever race the same node's state (they shouldn't — one node
fires at a time per run — but the cluster path will eventually have
multiple processes attached to the same SQLite file), the slow tick
notices and re-reads instead of overwriting.

## Two-impl pattern

Following the established pattern from `FlowStore` / `RunStore` /
`SessionStore`: one in-process impl in `starter-flow`, one SQLite impl
in `starter-store-sqlite`, both behind exactly the same trait. The
parameterised test matrix (`get-missing` / `get-after-put` /
`put-overwrites` / `cas-success` / `cas-mismatch` /
`delete-then-get-missing`) lives in `tests/node_state_in_memory_test.rs`
and `tests/node_state_sqlite_test.rs`; the two files are deliberately
copy-paste twins so a divergence is loud.

- `starter_flow::state::in_memory::InMemoryNodeStateStore` —
  `Arc<RwLock<HashMap>>`. Default the in-process engine wires when
  no SQLite-backed store is attached.
- `starter_store_sqlite::flow::node_state::SqliteNodeStateStore` —
  one `node_state` table keyed by `(flow_id, node_id, key)` with a
  `version INTEGER` column for CAS. Mutating calls run inside
  `BEGIN IMMEDIATE` so a concurrent writer cannot interleave the
  version read with the bump.

The migration sits at
`migrations/flow/0003_node_state.sql` — additive, forward-only, no
backfill (the table is empty on first boot).

## `reset_on_redeploy` semantics

The store does not interpret flow revisions. When a kind opts into
`reset_on_redeploy` (settings schema flag, lands with the counter node
in R2), the **engine** is responsible for calling
`delete(&NodeStateKey { flow_id, node_id, key })` for every key the
old node held before the new revision sees its first tick. The store
just stores; the engine just deletes.

A kind that does not opt in (the default) keeps its state across
revision changes — that is what makes the demo counter survive a hot
edit of the cron string or the step size.

## Size caps

- `key.key` ≤ `NodeStateKey::MAX_KEY_BYTES` (256 bytes).
- `value.bytes` ≤ `NodeStateValue::MAX_VALUE_BYTES` (64 KiB).

Both caps are enforced at the SPI boundary inside `NodeStateKey::new`
and `NodeStateValue::new` (and re-checked inside the in-memory and
sqlite `put` / `cas` paths so a caller that bypassed the constructors
still fails loudly). Exceeding either cap returns
`NodeStateError::{KeyTooLarge,ValueTooLarge}` *without* touching the
store.

The caps are chosen to fit the counter / cron-state / last-emitted-
hash use cases the seam exists for; values larger than that belong in
a `RunStore` checkpoint blob or a bespoke domain table.

## `NoopNodeStateStore`

For tests of node bodies that do not exercise state, the spi crate
ships a static `NOOP_NODE_STATE_STORE`. `get` returns `Ok(None)`;
every mutating call returns `NodeStateError::Backend("Noop... called
— wire a real NodeStateStore into NodeCtx")` so a missing wiring
fails loudly rather than silently swallowing writes.

## Out of scope (here)

- Listing or scanning all keys under a `(flow_id, node_id)` pair.
  No node body currently needs it; if a future kind does, add a
  `list_prefix` method and update both impls + the matrix at the
  same time.
- Cross-flow shared state. Out of scope by design — share state
  through a domain table reached by an extension, not by punning on
  `flow_id`.
- TTL / expiry. The store is a key/value box; if a kind wants TTL,
  it stores `(value, expires_at)` and checks on read.
