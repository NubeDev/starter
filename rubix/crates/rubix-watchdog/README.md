# rubix-watchdog

External liveness supervisor for `rubix-agent`. Probes `/livez` on a
fixed cadence; on sustained failure, force-kills the agent and runs
a restart command.

This crate exists because the failure mode we built it for —
**all tokio workers parked on a futex** — cannot be handled from
inside the wedged runtime. The defense has to be an OS-level peer
process.

---

## The freeze that motivated this

### Symptom

Roughly once per dev session, the agent stops responding. Process
is still alive (`pgrep` finds it, `ss` shows port 8088 still bound),
but every HTTP request times out and the log file simply stops
growing. `make restart` recovers; nothing else does.

### What we know from `/proc` at freeze time

- 29 threads total: 1 main + 28 `tokio-rt-worker` (= `nproc`).
- **Every single thread** in `State: S` parked on `futex_do_wait`.
- No `epoll_wait` anywhere — the wedge is pure CPU lock contention,
  not I/O.
- Process RSS stays flat (~50 MB); no memory leak symptom.
- `kill -USR1 <pid>` writes nothing to the log — the tokio signal
  stream can't be polled when no worker is making forward progress.

### Class of bug

This is the classic **lock-across-await deadlock** family. Some task
holds a synchronisation primitive (`tokio::sync::Mutex`,
`tracing-subscriber`'s span machinery, a `broadcast` channel slot)
across an `.await`, that `.await` ends up waiting on the same
primitive transitively, and the dependency cycle drains every worker
until the runtime has nobody left to schedule.

The exact site is **still unknown** at the time of writing. The
two strongest signals point at it:

1. Just before each freeze, the log shows the same `run_id`
   emitting `log.invoke` lines 3× within microseconds — the
   "producer multi-fire" pattern the comment in
   [`crates/starter-flow/src/graph.rs::write_slot_batch`] calls out.
   The duplicate emissions per tick spike concurrent span enter/exit
   on the same `run_id`, which puts pressure on whatever lock is
   actually deadlocking.

2. The auth pool (`rubix-auth`) is the only pool whose `in_use`
   count grows across the freeze window — `0 → 2 → 4` over a few
   seconds, then silence. Suggests requests on the auth-gated path
   are acquiring conns and not releasing.

These are correlations, not causation. Finding the actual await
site requires per-task introspection — see the `tokio-console`
section below.

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

---

## Behaviour

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

Three new targets, two new variants of an existing one.

### `make restart` — what changed

`make stop` now polls until port 8088 is actually free before
returning (5s ceiling, 250ms granularity). Without this, `make
restart` raced: `start` ran while the dying agent's listener was
still bound, the `agent` target saw the port "in use" and skipped
launch, and no new agent came up. The watchdog has hit this exact
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
`127.0.0.1:6669`. In another terminal:

```bash
tokio-console
```

Gives you per-task names, last poll durations, busy ratios,
held-lock holders, and the source location of every outstanding
`.await`. This is the diagnostic surface that will name the
deadlock site.

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

### Debugging session

```bash
make restart-console        # rebuild + restart with tokio-console
make watchdog-bg            # optional — auto-recover during the session
tokio-console               # in another terminal
# trigger the freeze (load tests, replay traffic, etc.)
# tokio-console will show which task parked and on what lock
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

## Companion diagnostics in `rubix-agent`

The watchdog is one of four pieces wired during the freeze
investigation. They are independent and complementary:

| Piece | Location | What it tells you |
|---|---|---|
| `runtime_canary` | [`rubix-agent/src/boot/runtime_canary.rs`] | `/livez` 200 vs 503 distinguishes "runtime wedged" from "HTTP layer wedged." Emits a heartbeat every 60s — its absence in the log is unambiguous evidence the canary task itself parked. |
| `runtime_metrics` (SIGUSR1) | [`rubix-agent/src/boot/runtime_metrics.rs`] | `kill -USR1 <pid>` dumps `num_workers` + `num_alive_tasks`. Rising `num_alive_tasks` across dumps = task leak; flat while frozen = workers parked. Will not respond when the runtime is too wedged to poll signals — that absence is itself a signal. |
| `task_watchdog` | [`rubix-agent/src/boot/task_watchdog.rs`] | Wraps long-lived background `JoinHandle<()>`s. If any (scheduler, canary, metrics, flow_notify) ever exits — panic, cancel, or clean return — emits a loud ERROR line so the supervisor *and* the operator notice. |
| `rubix-watchdog` | this crate | OS-level peer process that auto-recovers when the in-process diagnostics can no longer write. |

The first three answer **what** happened. This crate makes sure
the agent is **running again** so you can get back to work.

---

## Future work

1. **Find the actual deadlock site.** Use `make restart-console`
   and `tokio-console` next time the freeze recurs. Once we know
   which `.await` is parked on which lock, fix the offending
   site and the watchdog becomes pure belt-and-braces.
2. **Coalesce the producer multi-fire.** Three identical
   `log.invoke` lines within microseconds is wasteful regardless
   of whether it's the wedge trigger; the rubix seed adapter
   should emit once per tick.
3. **`SO_REUSEADDR` on the agent listener.** Would shrink the
   `make stop` polling window from 5s to ~0s and make the race
   protection in the Makefile unnecessary.
4. **Production systemd unit.** When the agent runs under systemd
   in prod, ship a `rubix-watchdog.service` peer unit with
   `Restart=on-failure` + `RestartSec=5` for both, with the
   agent's `Restart=no` so only the watchdog drives lifecycle.
