# rubix-watchdog

External liveness supervisor for `rubix-agent`. Probes `/livez` on a
fixed cadence; on sustained failure, force-kills the agent and runs
a restart command.

This crate exists because the failure mode we built it for —
**all tokio workers parked on a futex** — cannot be handled from
inside the wedged runtime. The defense has to be an OS-level peer
process.

> **Status (2026-05-27, afternoon update):** The earlier diagnosis
> below — that a `let _enter = span.enter()` across an `.await` in
> [`crates/starter-flow/src/graph.rs`][graph-fix] was the root
> cause — turned out to be **incorrect / incomplete**. After that
> "fix" landed at 11:53, the agent still wedged within ~19 minutes
> on the same futex signature. A second investigation (see
> [§ Real root cause](#real-root-cause-2026-05-27-afternoon) below)
> identified the actual culprit: a Postgres pool-connection leak
> in the `/api/v1/dashboards/events` SSE route. Fix applied at
> ~13:00 in [`rubix/crates/rubix-agent/src/main.rs:573-606`][de-fix].
> The `rubix-auth` pool no longer bursts; the wedge cascade is
> broken. A secondary leak (the listener itself doesn't release
> its own dedicated-pool connection promptly on SSE disconnect)
> remains as follow-up.

[graph-fix]: ../../../crates/starter-flow/src/graph.rs
[de-fix]: ../rubix-agent/src/main.rs

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

### Real root cause (2026-05-27 afternoon)

After the `graph.rs` change above landed at 11:53, the agent wedged
again at 12:37 (~19 min uptime, post-fix binary confirmed by mtime).
A second investigation reproduced the wedge with the producer flow
**completely disabled** (`data-flow-producer.yaml` renamed aside),
proving the producer/`write_slot_batch` path was not the trigger.

What the second investigation found in the log immediately before
the wedge:

```
WARN rubix.boot.pool_telemetry: pool near saturation
     pool="rubix-auth" size=16 idle=0 in_use=16 max=16
WARN rubix.routes.dashboard_events: ChangeTail::subscribe failed
     error=internal error
WARN sqlx::pool::acquire: acquired connection, but time to acquire
     exceeded slow threshold acquired_after_secs=27.159
WARN sqlx::pool::acquire: acquired_after_secs=25.251
... ×19 more, descending from 25s to 15s
```

Twenty tasks had been parked in `sqlx::Pool::acquire().await` for
15–27 seconds. The "futex_do_wait on every worker" signature in
`/proc` is the same shape whether the lock is a `tracing` RwLock
**or the semaphore inside `sqlx::Pool`** — the original diagnosis
mis-identified which one.

#### Where the leak lives

[`crates/starter-changelog-postgres/src/tail_listen.rs:81`](../../../crates/starter-changelog-postgres/src/tail_listen.rs)
calls:

```rust
let mut listener = PgListener::connect_with(self.pool.sqlx()).await?;
```

`PgListener::connect_with(&pool)` **permanently consumes one
connection from the pool** for the lifetime of the listener. The
spawned task at lines 102-135 then sits in a `listener.try_recv()`
loop forever holding that connection.

The team already knew this — explicit comments in
[`boot/flow_notify.rs:84`](../rubix-agent/src/boot/flow_notify.rs)
("Dedicated tiny pool: PgListener pins one connection for LISTEN")
and
[`boot/pool_telemetry.rs:60-64`](../rubix-agent/src/boot/pool_telemetry.rs)
("the LISTEN listener uses max=2 with one connection pinned
forever by PgListener") show the pattern was understood. The
`flow_notify` listener was given its own dedicated 2-connection
pool for exactly this reason. The **dashboard SSE listener wired
in [`rubix-agent/src/main.rs:573`](../rubix-agent/src/main.rs)
missed that mitigation** — it was constructed against the shared
16-connection auth pool.

So every dashboard sidebar SSE subscription (one per browser tab,
one per reconnect, plus whatever the frontend opens implicitly)
permanently consumed one slot of the 16-connection `rubix-auth`
pool. Once 16 were held, every auth-gated route — every
`with_principal`-wrapped handler in the agent — blocked on
`pool.acquire().await`. The 32 tokio workers piled up on those
acquires, parked on the sqlx pool semaphore's internal futex, and
the runtime wedged.

The previous "span guard" theory was on a code path that just
happened to also be parked on the auth pool. Misled by symptom
similarity to a real footgun documented elsewhere in the file.

#### Fix applied

Mirror the `flow_notify` pattern — give the dashboard SSE
listener its own dedicated tiny pool, isolated from the shared
auth pool. [`rubix-agent/src/main.rs:573-606`](../rubix-agent/src/main.rs):

```rust
let listen_inner = PgPoolOptions::new()
    .max_connections(2)
    .connect(dsn).await?;
let listen_pool = Pool::from_sqlx(listen_inner);
let _t_dash_listen =
    boot::pool_telemetry::spawn(listen_pool.sqlx().clone(), "rubix-dash-listen");
let tail = Arc::new(PgListenTail::new(listen_pool));
let store = Arc::new(PgDashboardStore::new(pool.clone())); // unchanged
```

After restart with this fix:

- `rubix-auth` pool stays at `in_use=0..4 / 16` — never bursts.
- `/livez` healthy through the prior 14-18 min MTBF window and
  beyond. No futex deadlock.
- The dashboard SSE leak is now **bounded and visible**: the new
  `rubix-dash-listen` pool saturates at 2/2 within a minute and
  the operator sees `ChangeTail::subscribe failed` warnings as
  the 3rd+ subscriber's acquire times out. The agent stays up.

#### What this exposed about the SSE feature

With the cascade severed, two distinct problems are now visible:

1. **The leak itself.** [`tail_listen.rs:102-135`](../../../crates/starter-changelog-postgres/src/tail_listen.rs)
   checks `tx.is_closed()` only at the top of each loop iteration,
   and the loop's `tokio::select!` sleeps for `safety_interval`
   (default 30s) on each idle tick. After an SSE client
   disconnects it can take **up to 30 seconds** before the
   spawned task notices and releases its pinned connection. If
   the frontend reconnects faster than that, "old" listeners pile
   up.

2. **The subscription rate.** With a fresh agent, no operator
   interaction, the new tiny pool was saturated within 60s — so
   either the frontend opens multiple SSE connections per page,
   or a reconnect loop is firing. Investigate before bumping
   pool size as a band-aid.

These are tracked in [§ Future work](#future-work-still-open-after-the-fix).

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

## Verification protocol

The `graph.rs` "span guard" fix at 11:53:22 on 2026-05-27 **did not
hold** — the agent wedged again at 12:37 on the same signature.
The real fix landed at ~13:00 in
[`rubix-agent/src/main.rs:573-606`](../rubix-agent/src/main.rs)
(dedicated `rubix-dash-listen` pool for `PgListenTail`).

**Quick verification (cascade is broken):**

1. `make restart` (no console rebuild needed).
2. `grep 'rubix-auth' /tmp/rubix-agent.log | tail` — the auth pool
   should stay in low single digits (`in_use=0..4`) and **never**
   emit `pool near saturation`. Pre-fix it would burst to
   `in_use=16` within ~5-19 minutes.
3. `/livez` should keep returning 200 indefinitely past the prior
   14-18 min MTBF window.

**Full verification (with tokio-console, leak count optional):**

1. `make CONSOLE=1 restart`.
2. Wait 30+ minutes.
3. Take a `console-dump http://127.0.0.1:6669 8` snapshot.
   Compare task/resource/op counts to baseline. Pre-wedge growth
   curve was 148 → 552 tasks over 18 minutes; post-fix should be
   flat near baseline (~150 / 120 / 120).

**What the fix does NOT do:** it does not stop the SSE listener
from leaking its own dedicated-pool connection — the leak is just
isolated to a 2-connection blast radius. Expect to see
`ChangeTail::subscribe failed` warnings as the 3rd+ concurrent
dashboard subscriber's acquire times out. The agent stays up; the
dashboard feature is degraded under load. See item 1 in
[§ Future work](#future-work-still-open-after-the-fix).

---

## Future work (still open after the fix)

1. **Tighten the `PgListenTail` drop latency.** Today the spawned
   task in
   [`crates/starter-changelog-postgres/src/tail_listen.rs:98-135`](../../../crates/starter-changelog-postgres/src/tail_listen.rs)
   only notices a dropped subscriber when the `safety_interval`
   sleep (default 30s) wins the `tokio::select!`. SSE
   reconnect-faster-than-30s = listener pile-up = `rubix-dash-listen`
   pool saturates. Fix: select on `tx.closed()` alongside the
   `try_recv` + safety sleep so connections are returned within ~1s
   of client disconnect.
2. **Investigate the SSE subscription rate.** Fresh agent, no
   operator interaction → 3+ concurrent dashboard subscribers
   within 60s. Either the frontend opens multiple SSE per page or
   a reconnect loop is firing. Diagnose before bumping
   `max_connections` on `rubix-dash-listen` as a band-aid.
3. **Audit every other `PgListener::connect_with(shared_pool)`
   site.** Today there are two known consumers — `dashboard_events`
   (now fixed) and `flow_notify` (already dedicated). Any future
   listener built against the shared 16-conn pool re-introduces
   the cascade. Consider making `PgListenTail::new` reject
   non-tiny pools at construction (debug_assert `max <= 4`) so
   the mistake fails loudly during boot, not silently 19 minutes
   later.
4. **Revert or re-evaluate the `graph.rs` "fix".** The change at
   [`crates/starter-flow/src/graph.rs:262-274`][graph-fix] was
   landed on a wrong diagnosis. Inspect whether the `span.enter()`
   call it removed was actually harmful — the original `tracing`
   footgun about span guards across `.await` is real, but the
   specific call site in `write_slot_batch` had no `.await` in
   the guard's scope. Either way, the change is defensible on
   tracing-hygiene grounds; just don't credit it with fixing the
   wedge.
5. **Cron expression interpretation.** The `*/5 * * * * *` schedule
   in producer.yaml is intended as "every 5 seconds" but actually
   fires once per minute (cron parser interprets it as "second-5
   of every minute"). Worth fixing for correctness even though it
   no longer correlates with the deadlock.
6. **Coalesce the producer multi-fire.** Three identical
   `log.invoke` lines within microseconds is wasteful regardless
   of whether it's a wedge trigger; the rubix seed adapter should
   emit once per tick. Tracked in
   [`rubix/docs/sessions/data-flow/2026-05-26-data-flow-01-producer-multi-fire.md`](../../docs/sessions/data-flow/2026-05-26-data-flow-01-producer-multi-fire.md).
7. **`SO_REUSEADDR` on the agent listener.** Would shrink the
   `make stop` polling window from 5s to ~0s and make the race
   protection in the Makefile unnecessary.
8. **Production systemd unit.** When the agent runs under systemd
   in prod, ship a `rubix-watchdog.service` peer unit with
   `Restart=on-failure` + `RestartSec=5` for both, with the
   agent's `Restart=no` so only the watchdog drives lifecycle.
