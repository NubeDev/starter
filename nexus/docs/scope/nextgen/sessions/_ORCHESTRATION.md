# Nexus Next-Gen — Orchestration Loop (the driver)

> This is the script the **loop** follows on every wake. It is NOT a workstream.
> The loop is the parent session; each workstream runs as a fresh **subagent** spawned by the loop.
> Everything lands on branch **`nexus-gaps`**, sequentially. No worktrees, no parallel writers.
>
> **NOTE — another AI session may be running concurrently.** Don't stress about diffs you didn't
> write. If a file you need to commit also has someone else's unrelated changes, commit only the
> hunks your WS touched (`git add -p`), and never revert/clobber changes you didn't make.

## Why sequential on one branch
Parallel agents on one branch overwrite each other. Sequential on one branch means each session
**commits before the next starts**, so a later session finds its dependencies (e.g. WS-03's macro
engine) already sitting in the working tree — dependencies resolve for free, no merging. The cost is
wall-clock; the win is reliability and zero collisions. This is the explicit user choice.

---

## LOOP ALGORITHM (run this every wake)

1. **Read [STATUS.md](./STATUS.md).** Identify the queue and each WS's status.
2. **Is a WS currently 🔵 in-progress?**
   - If a subagent is still running for it → do nothing, reschedule, exit. (Don't double-spawn.)
   - If marked 🔵 but no subagent is running (it returned) → run the **DONE GATE** on it (step 4).
3. **No WS in progress?** Pick the **first** WS in queue order whose status is ⬜ pending.
   - If none pending → check for ⛔ blocked rows whose blocker the human has since resolved
     (TODOs.md entry struck through / removed): reset those to ⬜ and pick the first.
   - If everything is ✅ or ⛔ and nothing is unblockable → **the run is complete.** Write a final
     loop-log line, summarize, and STOP the loop (do not reschedule).
4. **DONE GATE** (before marking any WS ✅ — this is how we trust a session finished):
   - `cargo test` (workspace) is **green**.
   - `cd nexus/ui && pnpm typecheck && pnpm build` is **green** (and `pnpm test` if the WS touched UI logic).
   - `openapi.json` regenerated + committed if DTOs changed (`cargo run --bin openapi > openapi.json`).
   - The session wrote a **`Done`** status line in its own `sessions/WS-xx.md` with a finish timestamp.
   - Working tree changes are **committed** on `nexus-backend` with a `WS-xx:` prefixed message.
   - If all pass → mark the row ✅, fill Finished + Commit columns, append a loop-log line.
   - If the build/tests are **red** and the session didn't flag a blocker → the session is NOT done.
     Spawn a fresh subagent to *fix the build for that WS only* (same charter). Do not advance.
5. **Spawn the next session** (step 3's pick): set its row to 🔵, fill Started, append a loop-log
   line, then launch the subagent with the **AGENT CHARTER** below (substituting the WS number).
6. **Reschedule** the next wake (~5 min) and exit. The loop re-enters at step 1.

> The loop itself never writes feature code. It only: reads STATUS, runs the gate, spawns one
> subagent, updates STATUS, reschedules. All feature work happens inside subagents.

---

## AGENT CHARTER (paste into every spawned subagent, substitute <WS-xx>)

```
You are implementing <WS-xx> for the Nexus dashboarding platform, as one autonomous session in an
unattended overnight build. You run to completion and return — you cannot ask the human anything.

READ FIRST, IN ORDER:
1. rubix/HOW-TO-CODE.md + rubix/FILE-LAYOUT.md     (the coding standard — governs every file)
2. nexus/docs/scope/nextgen/GAP_ANALYSIS.md        (why this matters)
3. nexus/docs/scope/nextgen/00_ROADMAP.md          (§0 re-verify, §4 your owned files, §5 your
                                                    migration block, §6 shared contracts, §8 DoD)
4. nexus/docs/scope/nextgen/<WS-xx>_*.md           (your spec — source of truth for scope)
5. nexus/docs/scope/nextgen/sessions/STATUS.md     (what's already done — your deps are committed)

CODING STANDARD (read these two FIRST — they govern every file you write):
- rubix/HOW-TO-CODE.md  and  rubix/FILE-LAYOUT.md
  The load-bearing rules from them:
  - ONE RESPONSIBILITY PER FILE. ≤400 lines hard (PR-blocking), ~100 typical. Split at 300.
    Verb-per-file folders (create.rs/update.rs/…), not noun-file-does-everything.
  - NO `utils.rs`/`helpers.rs`/`common.rs`/`misc.rs` — name the concept. `mod.rs` is a barrel only.
  - Transport handlers are THIN (≤20 lines): extract → call ONE domain fn → map DTO → return.
    No SQL, no business predicates, no loops/filters on domain data in a handler. Each transport
    file's opening doc-comment carries the `LAYER: transport (REST).` banner (HOW-TO-CODE §6).
  - Comments explain WHY not WHAT. Doc-comment every public item. NO progress markers
    (`// STAGE-1`, `// FIXED:`, `// Phase 0`), NO emoji. Bare TODOs forbidden — use `// TODO(loop):`.
  - Code comments may reference `docs/design/` only — never scope/session docs or HOW-TO-CODE.md.

HARD RULES (this is an unattended run — violating these poisons every later session):
- BRANCH: work on `nexus-gaps`. Do NOT create branches or worktrees. Do NOT switch branches.
  Another AI session may be editing the same branch — commit only YOUR hunks (`git add -p` the
  files your WS owns), never `git add -A`, never revert changes you didn't make.
- NO QUESTIONS: you cannot prompt the human. If you hit a genuine ambiguity or need work a
  not-yet-run session owns, you DO NOT guess and DO NOT hack/stub. Instead:
    (a) append a dated entry to nexus/docs/scope/nextgen/sessions/TODOs.md in the documented format,
    (b) set your row in STATUS.md to ⛔ blocked with a one-line reason,
    (c) commit whatever compiles cleanly so far, then STOP and return a summary.
- NO HACKS: no `unwrap()` on fallible paths to "make it compile", no `todo!()`/`unimplemented!()`
  left in shipped paths, no commented-out tests, no `#[ignore]` to dodge a failure, no stubbed
  functions that pretend to work. If you can't do it properly, it's a TODO entry, not a fake.
- STAY IN YOUR LANE: edit only files your WS owns (ROADMAP §4). Touch a 🔶 shared file only as a
  tiny append. If a shared contract you depend on is missing, that's a TODOs.md blocker — do not
  redefine it.
- DTO-FIRST: nexus-spi DTO → register in openapi.rs → `cargo run --bin openapi > openapi.json`
  → `cd nexus/ui && pnpm codegen`. Never hand-edit generated client types.
- Use migration number from ROADMAP §5 for your WS block.
- Ship mirrored tests. Keep `cargo test` and `cd nexus/ui && pnpm typecheck && pnpm test && pnpm build`
  GREEN before you call yourself done. A red build means you are not done.

SESSION LOG (mandatory): create/maintain nexus/docs/scope/nextgen/sessions/<WS-xx>.md with:
  - a `Status:` line (In-progress / Blocked / Done) and a `Started:` + `Finished:` UTC timestamp,
  - the task breakdown you executed and what each commit did,
  - any assumptions, any deviations, any follow-ups.

FIRST ACTION (ROADMAP §0, mandatory): re-grep every file:line your WS's "Current state" section
cites; if a claim drifted, fix the WS doc + bump its `Verified:` line BEFORE coding. Then confirm
your shared-contract deps exist in the tree. Then implement, commit (message prefixed `<WS-xx>:`),
and update STATUS.md + your session doc. When done, ensure the build/tests are green and return a
concise summary of what landed and what (if anything) you logged to TODOs.md.
```

---

## HEADLESS CRON MODE (the 100%-unattended path)

The loop survives a closed editor / sleeping session only when fired by the OS, not from a chat
window. The cron job runs **one wake per firing** with `claude -p` and exits — it is NOT the
in-session `/loop`. Each firing executes the LOOP ALGORITHM above exactly once.

**Concurrency lock (MANDATORY — prevents two firings double-spawning a WS):**
Before doing anything, the firing must acquire an exclusive lock and skip if it can't:
```
exec 9>nexus/docs/scope/nextgen/sessions/.loop.lock
flock -n 9 || { echo "$(date -u +%FT%TZ) another firing holds the lock — skip"; exit 0; }
```
A firing that holds the lock runs ONE wake (gate the returned WS, or spawn the next pending WS) and
exits, releasing the lock. A WS subagent can run longer than 5 min; that's fine — subsequent firings
see the row is 🔵 with work still committing and either (a) the subagent already returned → run the
gate, or (b) detect no new commits + no completion in WS-xx.md for a while → treat as still-running
and skip. **Never spawn a second WS while one is 🔵 and its WS-xx.md has no Blocked/Done line.**

**Determining "subagent still running" without live process state:** headless firings can't see a
previous firing's subagent. Use durable signals only: the WS-xx.md `Status:` line and `git log`.
- Row 🔵 + WS-xx.md Status `In-progress` + commits advancing across firings → still working, skip.
- Row 🔵 + WS-xx.md Status `Done`/`Blocked` → run the gate / honor the block, then advance.
- Row 🔵 + WS-xx.md Status `In-progress` + NO new commits for ≥3 firings (~15 min) → assume the
  subagent died; re-spawn the SAME WS fresh (it resumes from committed state — work is idempotent
  because each WS reads STATUS + git to see what's already landed).

**Heartbeat-based death detection (the precise signal):** `loop-tick.sh` writes
`sessions/.loop.heartbeat` (`<utc> wake-start`) before the long claude call and `<utc> wake-complete`
after. A recovering firing that holds the lock checks: if a WS row is 🔵, its WS-xx.md is still
`In-progress`, the heartbeat reads `wake-start`, AND that timestamp is >20 min old → the prior wake
died mid-run (machine slept / claude crashed). Re-spawn that SAME WS. If the heartbeat is recent,
a wake is genuinely in flight — but it would also hold the lock, so a fresh firing wouldn't get here
anyway. This file is the tie-breaker for the case where the lock was force-released by a kill.

**The installer:** `sessions/install-cron.sh` writes the crontab line. To stop the run, the human
runs `crontab -r` (or removes the line) — leave a `STOP` sentinel check too: if a file
`sessions/.loop.STOP` exists, every firing exits immediately without spawning. That's the kill switch.

## Notes for the loop driver
- **One subagent at a time.** Never spawn a second WS while one is 🔵 with a live subagent.
- **Timestamps:** the runtime has no clock inside scripts; when you (the loop) write timestamps,
  use `date -u` via Bash to get the real UTC time.
- **Crash recovery:** if the loop is restarted, step 1 reconstructs all state from STATUS.md +
  the per-session docs + `git log` — there is no hidden state. Safe to resume any time.
- **Definition of "all done":** every queue row is ✅, OR the remaining rows are ⛔ blocked and
  their TODOs are unresolved. Then STOP and report.
