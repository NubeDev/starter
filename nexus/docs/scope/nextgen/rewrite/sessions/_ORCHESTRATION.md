# Nexus Rewrite — Orchestration Loop (the driver)

> This is the script the **loop** follows on every wake. It is NOT a workstream.
> The loop is the parent session; each workstream runs as a fresh **subagent** spawned by the loop.
> Everything lands on branch **`nexus-rewrite`**, sequentially. No worktrees, no parallel writers.
>
> **NOTE — another AI session may be running concurrently.** Don't stress about diffs you didn't
> write. If a file you need to commit also has someone else's unrelated changes, commit only the
> hunks your RW touched (`git add -p`), and never revert/clobber changes you didn't make.

## Why sequential on one branch
Parallel agents on one branch overwrite each other. Sequential on one branch means each session
**commits before the next starts**, so a later session finds its dependencies (e.g. RW-01's core
traits) already sitting in the working tree — dependencies resolve for free, no merging. The cost
is wall-clock; the win is reliability and zero collisions. This is the explicit user choice,
proven by the 2026-06-09 nextgen run (13/13 landed).

---

## LOOP ALGORITHM (run this every wake)

1. **Read [STATUS.md](./STATUS.md).** Identify the queue and each RW's status.
2. **Is an RW currently 🔵 in-progress?**
   - If a subagent is still running for it → do nothing, reschedule, exit. (Don't double-spawn.)
   - If marked 🔵 but no subagent is running (it returned) → run the **DONE GATE** on it (step 4).
3. **No RW in progress?** Pick the **first** RW in queue order whose status is ⬜ pending.
   - If none pending → check for ⛔ blocked rows whose blocker the human has since resolved
     (TODOs.md entry struck through / `✅ RESOLVED`): reset those to ⬜ and pick the first.
   - If everything is ✅ or ⛔ and nothing is unblockable → **the run is complete.** Write a final
     loop-log line, summarize, and STOP the loop (remove the cron via install-cron.sh remove).
4. **DONE GATE** (before marking any RW ✅ — this is how we trust a session finished):
   - `cargo test --workspace` is **green** (run in `nexus/backend`).
   - If the RW touched UI/DTOs: `openapi.json` regenerated + committed and
     `cd nexus/ui && pnpm typecheck && pnpm build` green (`pnpm test` if UI logic changed).
   - The session wrote a **Done** status line in its own `sessions/RW-xx.md` with a finish timestamp
     (grep format-agnostic: `[Ss]tatus.*Done` — bold markdown variants count).
   - Working tree changes are **committed** on `nexus-rewrite` with an `RW-xx:` prefixed message.
   - The commit is **PUSHED**: `git push origin nexus-rewrite` (human requirement 2026-06-10 —
     every finished RW must be on the remote, on the CURRENT branch, never a new branch).
     If the session forgot to push, the gate pushes before marking ✅. If push fails
     (auth/network), still mark ✅ locally but log the failed push to TODOs.md loudly.
   - If all pass → mark the row ✅, fill Finished + Commit columns, append a loop-log line.
   - If the build/tests are **red** and the session didn't flag a blocker → the session is NOT done.
     Spawn a fresh subagent to *fix the build for that RW only* (same charter). Do not advance.
5. **Spawn the next session** (step 3's pick): set its row to 🔵, fill Started, append a loop-log
   line, then launch the subagent with the **AGENT CHARTER** below (substituting the RW number).
6. **Reschedule** the next wake (~5 min) and exit. The loop re-enters at step 1.

> The loop itself never writes feature code. It only: reads STATUS, runs the gate, spawns one
> subagent, updates STATUS, reschedules. All feature work happens inside subagents.

---

## AGENT CHARTER (paste into every spawned subagent, substitute <RW-xx>)

```
You are implementing <RW-xx> of the Nexus engine rewrite (ArkFlow removal + data engine), as one
autonomous session in an unattended build. You run to completion and return — you cannot ask the
human anything.

READ FIRST, IN ORDER:
1. rubix/HOW-TO-CODE.md + rubix/FILE-LAYOUT.md          (the coding standard — governs every file)
2. nexus/docs/scope/nextgen/rewrite/00_REWRITE_ROADMAP.md  (§2 architecture, §4 your owned files,
                                                            §5 migrations, §6 shared contracts,
                                                            §7 DoD, §8 hard constraints)
3. nexus/docs/scope/nextgen/rewrite/<RW-xx>_*.md        (your spec — source of truth for scope)
4. nexus/docs/scope/nextgen/rewrite/sessions/STATUS.md  (what's already done — your deps are committed)

CODING STANDARD (the load-bearing rules):
- ONE RESPONSIBILITY PER FILE. ≤400 lines hard (PR-blocking), ~100 typical. Split at 300.
  Verb-per-file folders, not noun-file-does-everything.
- NO `utils.rs`/`helpers.rs`/`common.rs`/`misc.rs` — name the concept. `mod.rs` is a barrel only.
- Transport handlers are THIN (≤20 lines): extract → call ONE domain fn → map DTO → return.
  Transport files carry the `LAYER: transport (REST).` doc banner.
- Comments explain WHY not WHAT. Doc-comment every public item. NO progress markers, NO emoji.
  Bare TODOs forbidden — use `// TODO(loop):`.

HARD RULES (this is an unattended run — violating these poisons every later session):
- BRANCH: work on `nexus-rewrite` (the CURRENT branch). Do NOT create branches or worktrees.
  Do NOT switch branches. Commit only YOUR hunks (`git add -p` the files your RW owns), never
  `git add -A`, never revert changes you didn't make.
- PUSH (MANDATORY): after your final commit, `git push origin nexus-rewrite`. A finished RW
  that exists only locally is NOT done. Never push to any other branch, never force-push.
  If the push is rejected because the remote moved, `git pull --rebase origin nexus-rewrite`
  (your commits are own-lane hunks, so rebase is safe) and push again.
- NO QUESTIONS: if you hit genuine ambiguity or need work a not-yet-run session owns, DO NOT guess
  and DO NOT hack/stub. Instead: (a) append a dated entry to rewrite/sessions/TODOs.md,
  (b) set your row in STATUS.md to ⛔ blocked with a one-line reason, (c) commit whatever compiles
  cleanly so far, then STOP and return a summary.
- NO HACKS: no `unwrap()` on fallible paths to "make it compile", no `todo!()`/`unimplemented!()`
  in shipped paths, no commented-out tests, no `#[ignore]` to dodge a failure (the RW-08 soak
  opt-in is the one sanctioned, documented exception), no stubbed functions that pretend to work.
- STAY IN YOUR LANE: edit only files your RW owns (ROADMAP §4). Touch a 🔶 shared file only as a
  tiny append. If a shared contract you depend on is missing, that's a TODOs.md blocker.
- ROADMAP §8 CONSTRAINTS ARE ABSOLUTE: no second database; public runner APIs frozen; no
  duckdb/pyo3/librdkafka; new heavy deps feature-gated OFF; don't copy ArkFlow code verbatim.
- DTO-FIRST: nexus-spi DTO → register in openapi.rs → regenerate openapi.json →
  `cd nexus/ui && pnpm codegen`. Never hand-edit generated client types.
- Ship mirrored tests. Keep `cargo test --workspace` (and UI gates if touched) GREEN before you
  call yourself done. A red build means you are not done.

SESSION LOG (mandatory): create/maintain nexus/docs/scope/nextgen/rewrite/sessions/<RW-xx>.md with:
  - a `Status:` line (In-progress / Blocked / Done) and `Started:` + `Finished:` UTC timestamps,
  - the task breakdown you executed and what each commit did,
  - any assumptions, deviations, follow-ups.

FIRST ACTION (mandatory): re-grep every file:line your RW spec's "Current state" section cites;
if a claim drifted, fix the spec doc + add/bump its `Verified:` line BEFORE coding. Then confirm
your dependency RWs' contracts exist in the tree. Then implement, commit (message prefixed
`<RW-xx>:`), update STATUS.md + your session doc, ensure green, and return a concise summary of
what landed and anything logged to TODOs.md.
```

---

## HEADLESS CRON MODE (the 100%-unattended path)

The loop survives a closed editor / sleeping session only when fired by the OS. The cron job runs
**one wake per firing** with `claude -p` and exits. Each firing executes the LOOP ALGORITHM above
exactly once.

**Concurrency lock (MANDATORY):** `loop-tick.sh` takes `flock -n` on `sessions/.loop.lock` and
skips if held. flock is the sole mutex; the kernel frees it when the holder dies. **NEVER `rm` the
lock** (lesson recorded 2026-06-09 in the nextgen supervisor log).

**Heartbeat:** `loop-tick.sh` writes `.loop.heartbeat` (`<utc> wake-start pid=<N>` /
`wake-complete pid=<N>`). Liveness = `kill -0 <pid>`, never timestamp guessing.

**Determining "subagent still running" without live process state:** durable signals only —
the RW-xx.md `Status:` line and `git log`.
- Row 🔵 + RW-xx.md `In-progress` + commits/file-mtimes advancing → still working, skip.
- Row 🔵 + RW-xx.md `Done`/`Blocked` → run the gate / honor the block, then advance.
- Row 🔵 + `In-progress` + NO progress for ≥3 firings (~15 min) + heartbeat PID dead → the wake
  died; re-spawn the SAME RW fresh (work is idempotent — each RW reads STATUS + git first).

**Kill switch:** `touch sessions/.loop.STOP` → every firing exits immediately. A STOP whose content
names a guarded PID auto-clears when that PID exits (supervisor handles it); a bare STOP is a human
kill switch — never auto-removed.

## Notes for the loop driver
- **One subagent at a time.** Never spawn a second RW while one is 🔵 with a live subagent.
- **Timestamps:** use `date -u` via Bash when writing timestamps.
- **Crash recovery:** all state reconstructs from STATUS.md + RW-xx.md + `git log`. Safe to resume.
- **All done:** every row ✅, or remaining rows ⛔ with unresolved TODOs → write the final report,
  run `./install-cron.sh remove`, STOP.
