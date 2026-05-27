# Freeze analysis — 2026-05-27 04:49:09 UTC

## Diagnosis: span-guard-across-await deadlock in `write_slot_batch`

Identical class to the previous span-clone wedge, in a code path the previous fix missed.

### Evidence

- 552 tasks, 1300 resources, 6060 async ops at wedge time (vs 142/120/120 at boot — 4× task growth, 11× resource growth, 50× outstanding-await growth)
- 70+ outstanding `tokio::sync::RwLock` async ops piling up on a single resource (`58546795155816469`)
- 39 `RwLock::write` + 31 `RwLock::read` waiters across the dump
- All 29 tokio workers parked on futex; only the console_subscriber thread alive (on its dedicated runtime)

### Root cause: `crates/starter-flow/src/graph.rs:262-274`

```rust
let mut nodes = self.nodes.write().await;    // line 262 - awaits, may migrate worker
for (slot, value, opts) in &writes {
    let span = tracing::info_span!("write_slot", ...);   // line 264
    let _enter = span.enter();                            // line 274  ← BUG
    // ... processing ...
}
drop(nodes);
```

This is **exactly** the pattern the comment at lines 175-181 of the same file warns against:

> // `Instrument` (not `span.enter()`) — the body awaits an
> // RwLock acquire. Holding a span guard across `.await`
> // corrupts the thread-local span stack when the future
> // migrates between tokio workers and later panics
> // `tracing-subscriber` on an unrelated emit. This is
> // the hottest write path in the propagator; getting it
> // wrong wedges the runtime.

The single-write path (`write_slot`, line 184) was fixed by wrapping the body in `async move { ... }.instrument(span).await`. **The batch-write path (`write_slot_batch`) was missed** and still uses `let _enter = span.enter();` after a `self.nodes.write().await`.

### Why it takes 14-18 minutes to manifest

- Producer + tick-counter crons fire every minute (the YAML `*/5 * * * * *` is misparsed as "second-5 of every minute")
- Each fire triggers `write_slot_batch` with multiple writes (`synth.output` → `ingest.input` AND `ingest.in`)
- Each batch leaves residual span-stack corruption on the worker that handled it
- After ~14-18 minutes of accumulated TLS span-stack damage, a worker hits the corruption boundary and `span.enter()` deadlocks inside `tracing-subscriber`'s internal `RwLock`
- Other workers that also have corrupted TLS stacks then deadlock on the next `span.enter()`
- Cascade: all 28 workers parked on futex within microseconds

### Fix

Replace lines 262-318 with a closure-based instrumentation pattern matching `write_slot`:

```rust
let span_for_loop = tracing::info_span!("write_slot_batch", ...);
async move {
    let mut nodes = self.nodes.write().await;
    for (slot, value, opts) in &writes {
        let span = tracing::info_span!(parent: &span_for_loop, "write_slot", ...);
        async {
            // ... processing body ...
        }.instrument(span).await;
    }
    drop(nodes);
    // ... emit ...
}.instrument(span_for_loop).await
```

OR simplest fix: just remove the `let _enter = span.enter();` at line 274, since the loop body has no `.await` and the spans are only used for `span.record(...)`. The span is created but never entered — fields recorded via `span.record()` still work without entering. Verify with tests that span event hierarchy is preserved.

### Companion finding (separate issue)

The producer YAML at `rubix/crates/rubix-flows/flows/data-flow-producer.yaml` has **duplicate links** (line 62-63: both `synth.output -> ingest.input` AND `synth.output -> ingest.in`) AND **duplicate cron declarations** (flow-level `cron_expr` at line 20 + trigger-node `cron_expr` at line 26). These compound the write_slot_batch load. Cleaning these up reduces frequency without fixing the root deadlock.

### Verification next session

After applying the `Instrument` fix:
1. `make CONSOLE=1 restart`
2. Let it run 30+ min (2× current MTBF) without wedge
3. Pre/post snapshot comparison: task/resource/op counts should be flat (within 10%), not growing
