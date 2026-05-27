# rubix-watchdog

External liveness supervisor for `rubix-agent`. Probes `/livez` on a
fixed cadence; on sustained failure, force-kills the agent and runs
a restart command.

This crate exists because the failure mode we built it for —
**all tokio workers parked on a futex** — cannot be handled from
inside the wedged runtime. The defense has to be an OS-level peer
process.

> **Status (2026-05-27):** The root cause of the recurring wedge was
> diagnosed via tokio-console and fixed in
> [`crates/starter-flow/src/graph.rs:262-274`][graph-fix]. See
> [§ Root cause](#root-cause-diagnosed-2026-05-27) below. The
> watchdog remains useful as a defense against future runtime
> wedges of unrelated origin — that's its job — but it's no longer
> compensating for a known bug.

[graph-fix]: ../../../crates/starter-flow/src/graph.rs

---

## The freeze that motivated this

### Symptom

Roughly once per dev session, the agent stopped responding. Process
was still alive (`pgrep` found it, `ss` showed port 8088 still
bound), but every HTTP request timed out and the log file simply
stopped growing. `make restart` recovered; nothing else did.

### `/proc`-level signature at freeze time

- 29 threads total: 1 main + 28 `tokio-rt-worker` (= `nproc`).
- **Every single thread** in `State: S` parked on `futex_do_wait`.
- No `epoll_wait` anywhere — the wedge is pure CPU lock contention,
  not I/O.
- Process RSS stays flat (~50 MB); no memory leak symptom.
- `kill -USR1 <pid>` writes nothing to the log — the tokio signal
  stream can't be polled when no worker is making forward progress.
- Voluntary context switches: **0 of 29 threads** advance in a 5s
  sample window. Total runtime deadlock.

### Root cause (diagnosed 2026-05-27)

A `let _enter = span.enter();` was held across a `for` loop body
that ran after `self.nodes.write().await` in
[`graph.rs::write_slot_batch`][graph-fix]. The comment at lines
175-181 of the same file (left by the prior fix for the
single-write path) explicitly warned this would wedge the runtime:

> `Instrument` (not `span.enter()`) — the body awaits an RwLock
> acquire. Holding a span guard across `.await` corrupts the
> thread-local span stack when the future migrates between tokio
> workers and later panics `tracing-subscriber` on an unrelated
> emit. This is the hottest write path in the propagator;
> getting it wrong wedges the runtime.

The prior fix landed in the single-write path (`write_slot`) but
missed the batch-write path (`write_slot_batch`). Both run hot —
batch is hit on every producer flow tick.

#### How it manifests

1. Producer + tick-counter crons fire on a `*/5 * * * * *`
   schedule (intended every 5s; actually fires once per minute due
   to a separate cron-parsing issue — see § Compounding factors).
2. Each fire triggers `write_slot_batch` with multiple writes
   (`synth.output` → `ingest.input` AND `ingest.in`).
3. Each batch leaves residual span-stack corruption on the worker
   that handled it.
4. After ~14-18 minutes of accumulated TLS span-stack damage, a
   worker hits the corruption boundary and `span.enter()`
   deadlocks inside `tracing-subscriber`'s internal `RwLock`.
5. Other workers that also have corrupted TLS stacks then
   deadlock on their next `span.enter()`.
6. Cascade: all 28 workers parked on futex within microseconds.

#### Direct evidence from tokio-console

Captured at freeze time:
- **552 alive tasks** (vs **142** at boot — 4× growth)
- **1300 tokio resources** (vs **120** at boot — 11× growth)
- **6060 outstanding async ops** (vs **120** at boot — 50× growth)
- **70+ tokio RwLock waiters** stacked on a single resource id
  (`58546795155816469`) — 39 `RwLock::write` + 31 `RwLock::read`
- Single task (id=1, axum accept loop) polling `Sleep::new_timeout`
  **1,684 times** — the timeout cycle that never gets to do work
  because the RwLock can't be acquired.

Full evidence dump: [`rubix/docs/freeze-evidence/wedge-20260527T044909Z/`](../../docs/freeze-evidence/wedge-20260527T044909Z/)
with [`ANALYSIS.md`](../../docs/freeze-evidence/wedge-20260527T044909Z/ANALYSIS.md)
containing the full trace, 5 pre-wedge snapshots, post-wedge dump,
and thread states.

#### Fix

```diff
         let mut nodes = self.nodes.write().await;
         for (slot, value, opts) in &writes {
+            // Span created but NOT entered. The single-write path
+            // (`write_slot`) had to switch from `span.enter()` to
+            // `.instrument(span).await` because holding a span
+            // guard across `.await` corrupts the thread-local span
+            // stack — see the comment at lines 175-181 above.
             let span = tracing::info_span!(
                 "write_slot",
                 ...
             );
-            let _enter = span.enter();
             let entry = nodes.entry(slot.node.clone()).or_default();
```

The loop body has no `.await`, so `.instrument()` isn't required.
The minimal change is to simply not enter the span at all —
`span.record(...)` below works without entering.

100 tests still pass (incl. all `write_slot_batch_*` tests).

### Why no `acquire_timeout` saves us

`sqlx`'s `acquire_timeout` defaults to 30s — but that timeout is a
tokio timer. When the runtime is wedged, **no tokio timer can
fire**, including the one that would surface "acquire timed out" as
a recoverable error. The same argument applies to:

- `tokio::time::timeout(...)`
- `tower_http::TimeoutLayer`
- `tokio::signal::unix::Signal::recv()` (the SIGUSR1 dump handler)

Any defense that depends on the tokio runtime making forward
progress is, by construction, also wedged. Hence: external
watchdog.

### Compounding factors (not the root, but worth fixing)

The producer YAML at
[`rubix/crates/rubix-flows/flows/data-flow-producer.yaml`](../../crates/rubix-flows/flows/data-flow-producer.yaml)
has two anti-patterns that multiply the `write_slot_batch` rate:

1. **Duplicate links** (line 62-63):
   ```yaml
   - { from: "synth.output",  to: "ingest.input" }
   - { from: "synth.output",  to: "ingest.in" }
   ```
   Both target the same downstream node. The same data flows
   twice on every producer tick.

2. **Duplicate cron declarations**: flow-level `cron_expr` at
   line 20 AND trigger-node `cron_expr` at line 26. Two
   schedulers compete on the same flow.

Cleaning these up reduces load but **wouldn't have fixed the
deadlock** — load just shifted the time-to-wedge.

---

## Diagnostic tooling built during the hunt

These live outside the crate but are part of the same defensive
posture. Keep them around — they will be useful for the next
freeze of a different origin.

### Inside `rubix-agent`

| Piece | Location | What it tells you |
|---|---|---|
| `runtime_canary` | [`rubix-agent/src/boot/runtime_canary.rs`](../rubix-agent/src/boot/runtime_canary.rs) | `/livez` 200 vs 503 distinguishes "runtime wedged" from "HTTP layer wedged." Emits a heartbeat every 60s — its absence in the log is unambiguous evidence the canary task itself parked. |
| `runtime_metrics` (SIGUSR1) | [`rubix-agent/src/boot/runtime_metrics.rs`](../rubix-agent/src/boot/runtime_metrics.rs) | `kill -USR1 <pid>` dumps `num_workers` + `num_alive_tasks`. Rising `num_alive_tasks` across dumps = task leak; flat while frozen = workers parked. Will not respond when the runtime is too wedged to poll signals — that absence is itself a signal. |
| `task_watchdog` | [`rubix-agent/src/boot/task_watchdog.rs`](../rubix-agent/src/boot/task_watchdog.rs) | Wraps long-lived background `JoinHandle<()>`s. If any (scheduler, canary, metrics, flow_notify) ever exits — panic, cancel, or clean return — emits a loud ERROR line so the supervisor *and* the operator notice. |
| `rubix-watchdog` | this crate | OS-level peer process that auto-recovers when the in-process diagnostics can no longer write. |

### tokio-console wiring

A Cargo feature `tokio-console` on
[`starter-observability`](../../../crates/starter-observability/) +
[`rubix-agent`](../rubix-agent/Cargo.toml) enables a
`console_subscriber` layer that exposes the runtime on
`127.0.0.1:6669` for live introspection of every task, resource,
and outstanding `.await`.

Toggle with `make CONSOLE=1 restart` (or `make restart-console`).
See [`crates/starter-observability/src/tracing/init.rs`](../../../crates/starter-observability/src/tracing/init.rs)
for the wiring — **note the per-layer filter** (commit message
explains): a global `EnvFilter` at `info` level would silently
suppress all tokio task events because the layer needs
`tokio=trace` and `runtime=trace`. The fix uses
`.with_filter(env_filter)` on the fmt layer only, leaving the
console layer unfiltered.

**You also need the `tokio-console` CLI** (separate install):
```bash
cargo install --locked tokio-console
# then in a new terminal:
tokio-console http://127.0.0.1:6669
```

### `/tmp/console-dump` — non-interactive gRPC dumper

A small Rust binary at `/tmp/console-dump/` that connects to the
console_subscriber gRPC server, subscribes for N seconds, and
prints a textual summary of tasks + resources + async ops sorted
by busy time. Useful for headless / scripted captures where the
tokio-console TUI can't run (no TTY, automated wakeup loop, etc.).

Sources are in `/tmp/console-dump/src/main.rs`. If lost, the key
proto interface is `console_api::instrument::InstrumentClient`
with `watch_updates(InstrumentRequest {})`.

### `/tmp/check-freeze.sh` — wedge detector + continuous snapshotter

The script that drove the autonomous diagnosis loop. Probes
`/livez`; on healthy, takes a console snapshot and rotates 30
files in `/tmp/snapshot-rotating/`; on wedge detection,
captures everything (console dump, threads, log tail, probes,
the last 5 pre-wedge snapshots) to
`rubix/docs/freeze-evidence/wedge-<TS>/`.

The **pre-wedge snapshots are the gold** — the post-wedge dump
often catches the runtime mid-collapse and is partial. The
snapshot from ~60s before the wedge has the full task graph.

---

## Behaviour (the watchdog itself)

Once per `RUBIX_WATCHDOG_PROBE_INTERVAL_SECS` (default 10s), the
watchdog `GET`s `RUBIX_WATCHDOG_URL` (default
`http://127.0.0.1:8088/livez`) with a 3s HTTP timeout. Any
non-200 / timeout / connection-refused counts as one failure.

On `RUBIX_WATCHDOG_FAILURE_THRESHOLD` (default 3) consecutive
failures (≈30s wedged), the watchdog escalates:

1. **SIGUSR1** to the agent. The runtime-metrics handler may manage
   to dump one line before death — useful forensics. If the agent
   recovers between SIGUSR1 and the next probe, the failure counter
   resets and no kill happens.
2. Wait 2s.
3. Re-probe `/livez` once. If the agent answered, abort the kill —
   cheap insurance against an over-eager restart.
4. **SIGABRT** to the agent. Produces a core dump (when
   `ulimit -c unlimited`) for post-mortem `gdb -c core` work.
   Preferred over SIGKILL because the dump is the only way to
   recover the futex chain after the fact.
5. Wait 5s for the process to die (poll `/proc/<pid>`).
6. **SIGKILL** if still alive.
7. Run `RUBIX_WATCHDOG_RESTART_CMD` via `sh -c`. The command is
   expected to daemonise (e.g. `nohup ... &`) — the watchdog does
   not track the new agent, it just resumes probing.

---

## Configuration

Every knob is an env var; defaults are sane for the dev setup.

| Env var | Default | Meaning |
|---|---|---|
| `RUBIX_WATCHDOG_URL` | `http://127.0.0.1:8088/livez` | Probe target |
| `RUBIX_WATCHDOG_PROBE_INTERVAL_SECS` | `10` | Cadence between probes |
| `RUBIX_WATCHDOG_HTTP_TIMEOUT_SECS` | `3` | Per-probe HTTP timeout |
| `RUBIX_WATCHDOG_FAILURE_THRESHOLD` | `3` | Consecutive failures before kill |
| `RUBIX_WATCHDOG_AGENT_PID_FILE` | _(unset)_ | If set, file containing the agent PID |
| `RUBIX_WATCHDOG_AGENT_PROCESS_NAME` | `rubix-agent` | If pid-file unset, `pgrep -f $name$` |
| `RUBIX_WATCHDOG_RESTART_CMD` | _(required)_ | Shell command to spawn after the kill |

The watchdog logs once per minute as a heartbeat
(`target=rubix_watchdog "watchdog heartbeat"`). Its absence from
the log is itself evidence — if the watchdog process dies, you
should know.

---

## How the `rubix/Makefile` wires this together

### `make restart` — the everyday command

`make stop` polls until port 8088 is actually free before
returning (5s ceiling, 250ms granularity). Without this, `make
restart` raced: `start` ran while the dying agent's listener was
still bound, the `agent` target saw the port "in use" and skipped
launch, and no new agent came up. The watchdog had hit this exact
race — the kill succeeded, the restart command exited 0, but no
agent was running afterwards.

If the port is still bound after 5s, `stop` prints a warning to
stderr but doesn't fail. The user sees the misbehavior instead of
it being hidden.

### `make restart-console` — tokio-console mode

Equivalent to `make CONSOLE=1 restart`. Sets
`RUSTFLAGS="--cfg tokio_unstable"` and adds `--features
tokio-console` to the cargo invocation. The resulting binary wires
a `console_subscriber` layer that exposes the tokio runtime on
`127.0.0.1:6669`.

Trade-off: the `tokio_unstable` cfg invalidates the cargo cache.
Toggling between `restart` and `restart-console` triggers a full
workspace rebuild. Keep one mode for the whole debug session.

### `make watchdog` — foreground supervisor

```bash
make watchdog
```

Runs the watchdog in the foreground. Restart command is preset to
`cd <rubix> && make restart`. Ctrl+C stops the watchdog without
touching the agent.

**Sharp edge:** if you Ctrl+C *while a restart is in flight*, the
terminal SIGINT propagates to the still-building `cargo run` child
and the new agent never comes up. Either wait for the restart to
finish, or use the daemonised variant.

### `make watchdog-bg` / `make watchdog-stop` — daemonised

```bash
make watchdog-bg     # nohup'd, logs to /tmp/rubix-watchdog.log
make watchdog-stop   # pkill the watchdog; agent left running
```

Use this for any session longer than a few minutes. The watchdog
becomes terminal-independent and the Ctrl+C race above does not
apply.

### Typical day

```bash
make start           # agent + frontend + docker compose
make watchdog-bg     # supervisor on, logs to /tmp/rubix-watchdog.log

# ... develop ...
# If the agent wedges, the watchdog auto-restarts it within ~30s.
# tail -f /tmp/rubix-watchdog.log to watch.

make watchdog-stop   # end of day
make stop
```

### Debugging session (the workflow that caught the bug)

```bash
make restart-console        # rebuild + restart with tokio-console
make watchdog-bg            # optional — auto-recover during the session

# In a separate terminal, either interactive:
tokio-console http://127.0.0.1:6669

# Or scripted continuous snapshotting:
/tmp/check-freeze.sh        # one-shot
# (this script is what the Claude wakeup loop drove every 4 min
#  to keep rotating pre-wedge snapshots in
#  /tmp/snapshot-rotating/, then capture the freeze when it hit)
```

---

## What this crate deliberately does NOT do

- **Parse the agent's log.** Logs may be rotated, compressed, or
  absent in a container. The probe URL is the contract.
- **Retry the restart command on failure.** If `make restart` is
  broken, that's an operator problem, not something the watchdog
  should paper over by spinning.
- **Adapt its cadence.** Constant cadence keeps behaviour boring
  and predictable; clever back-off hides bugs.
- **Run multiple agents or compete with systemd.** If you run it
  under systemd, configure systemd `Restart=no` on the agent
  service and let the watchdog own lifecycle.
- **Tail-recover from a partial-restart failure.** If the kill
  worked and the restart command failed silently (e.g. port not
  freed in time, see above), the watchdog will detect the next
  failed probe and try again on its next cycle. We don't try to be
  clever about it.

---

## Verification protocol for the graph.rs fix

The fix went in at 11:53:22 on 2026-05-27. Prior MTBF was
**14-18 minutes**. Verification is straightforward:

1. `make CONSOLE=1 restart`
2. Wait 30+ minutes (≥2× prior MTBF).
3. If still healthy → fix confirmed.
4. Take a `/tmp/console-dump/target/release/console-dump
   http://127.0.0.1:6669 8` snapshot. Compare task/resource/op
   counts to the baseline at boot (~140 / 120 / 120). If they're
   flat (within 10%), the leak is gone.

The pre-fix progression was:
- t+ 4min:  148 tasks / 120 res / 120 ops
- t+ 7min:  152 / 120 / 120
- t+10min:  158 / 120 / 120
- t+12min:  162 / 120 / 120
- t+14min:  176 / 120 / 120
- t+18min (wedge): 552 / 1300 / 6060

If the post-fix curve stays at ~150 / 120 / 120 indefinitely, the
diagnosis was correct and the fix complete.

---

## Future work (still open after the fix)

1. **Cron expression interpretation.** The `*/5 * * * * *` schedule
   in producer.yaml is intended as "every 5 seconds" but actually
   fires once per minute (cron parser interprets it as "second-5
   of every minute"). Worth fixing for correctness even though it
   no longer correlates with the deadlock.
2. **Coalesce the producer multi-fire.** Three identical
   `log.invoke` lines within microseconds is wasteful regardless
   of whether it's a wedge trigger; the rubix seed adapter should
   emit once per tick. Tracked in
   [`rubix/docs/sessions/data-flow/2026-05-26-data-flow-01-producer-multi-fire.md`](../../docs/sessions/data-flow/2026-05-26-data-flow-01-producer-multi-fire.md).
3. **`SO_REUSEADDR` on the agent listener.** Would shrink the
   `make stop` polling window from 5s to ~0s and make the race
   protection in the Makefile unnecessary.
4. **Production systemd unit.** When the agent runs under systemd
   in prod, ship a `rubix-watchdog.service` peer unit with
   `Restart=on-failure` + `RestartSec=5` for both, with the
   agent's `Restart=no` so only the watchdog drives lifecycle.
5. **Audit the rest of `starter-flow` for the same anti-pattern.**
   The fix only addressed `write_slot_batch`. Run
   `grep -rn 'let _enter\|let _g.*enter()' crates/starter-flow*`
   and verify every site either (a) has no `.await` in scope OR
   (b) uses `.instrument(span).await` instead. The single-write
   and batch-write fixes show this footgun is easy to introduce.
