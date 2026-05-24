# Session handover — 2026-05-24: codeless orchestration & current rubix state

> **Tier:** session note. Lifetime: days. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source code must
> never reference this file.** Supersedes
> [2026-05-23-next-steps-7.md](./2026-05-23-next-steps-7.md) for
> the "what's running" picture; that file is preserved for the
> Phase 2b U1/U2/U3 narrative which is still accurate as history.

This handover captures two threads the next session needs:

1. **Where rubix is** — the thin slice landed end-to-end, the
   six-step smoke test ran, four bugs were found and fixed,
   one codeless job is paused mid-flight on a rebased branch.
2. **How to drive codeless** — submit, overlay, start, restart,
   clean up. The mechanics are not in `HOW-TO-CODE.md`; this is
   the operator runbook.

---

## 1. Read first, in this exact order

Non-negotiable. Read these fully before touching anything:

1. [NEW-SESSION.md](../../NEW-SESSION.md) — non-negotiables,
   doc-tier rules, layer separation, smoke test.
2. [HOW-TO-CODE.md](../../HOW-TO-CODE.md) — contributor entry
   point. Decision tree, crate map.
3. [FILE-LAYOUT.md](../../FILE-LAYOUT.md) — Rule Zero in long
   form. ≤400 lines per file; verb-per-file.
4. [SCOPE.md](../../SCOPE.md) — the thirteen rules + phases +
   non-goals.
5. [docs/scope/THIN-SLICE.md](../scope/THIN-SLICE.md) — the
   active roadmap. Five PRs end-to-end. PRs 1–5 all landed.
6. **This file.**
7. [2026-05-24-smoke-test-pr28.md](./2026-05-24-smoke-test-pr28.md)
   — the bug report that drove PR #29.
8. The Phase 2b history: [2026-05-23-next-steps-7.md](./2026-05-23-next-steps-7.md).
9. The two design docs covering what's wired today:
   - [docs/design/agent/](../design/agent/README.md) — runtime
     wiring picture.
   - [docs/design/starter-changes/](../design/starter-changes/README.md)
     — upstream PR ledger (Phase 2a, 2b, 2c all marked landed).

After step 9, you can answer the NEW-SESSION §1 self-test.

---

## 2. State of play

### What's on master right now

Master HEAD as of this handover: `698c025` (PR #29 merged).

| Item | Status |
|---|---|
| PR 1 — disk tool standalone | ✅ landed |
| PR 2 part 1 + part 2 (Path B) — auth + authz + audit + Postgres | ✅ landed |
| PR 3 — MCP exposure | ✅ landed in PR #27 |
| PR 4 — ClickHouse history + insights rule + alert | ✅ landed in PR #27 |
| PR 5 — REST + CLI parity | ✅ landed in PR #27 |
| Demo wiring — Block A (binary), Block B (runtime deps), Block C (Claude Desktop stdio) | ✅ landed in PR #28 |
| Smoke-test bug fixes — B1 (mani path), B2 (password + DSN), B3 (CH TTL casts), B4 (ChClient gating) | ✅ landed in PR #29 |
| U1 (`starter-mcp` Accept-Language) + U2 (`InMemoryTransport`) + U3 (`FlowRegistry::resolve`) | ✅ landed pre-#27 |
| Block A of the **agent-runtime** job (rubix-flows YAML loader) | 🟡 **in worktree only**, see §3 |

### What's running but NOT yet merged

**One codeless job is paused on a rebased branch, ready to restart.**

- **Job ID:** `01KSBTCGYB1GVVKX4GG3E3XHAC`
- **Name:** `rubix-agent-runtime`
- **Branch:** `codeless/rubix-agent-runtime`
- **Status:** `stopped` (cost $0, ended after stage 0's REVIEW
  gate hit the diff-verify pre-check trap — a known runtime
  bug, not a code issue; see §4).
- **Worktree:** `/home/user/.codeless/worktrees/job-01KSBTCGYB1GVVKX4GG3E3XHAC`
- **What landed:** commit `542b918` ("stage 1 (block A, rubix
  YAML loader) — parse bundled flows into FlowBody"). The
  worktree HAS the YAML loader; it does NOT have Block B
  (upstream `starter-ai-agent` + `starter-flow-node-loop`) or
  Block C (wire AiRunner + register node kind in rubix-agent).
- **Was rebased onto master:** yes, cleanly. The branch is now
  `542b918` ← `698c025` (current master) — no merge conflicts.
- **Was force-pushed:** yes, via `--force-with-lease` to
  `origin/codeless/rubix-agent-runtime`.

To resume the job, see §6 "Restarting a paused job."

### Test counts (last run on the worktree)

- `starter-i18n --features preferences`: 52 passed
- `starter-spi --all-features`: 67 + 7 + 6 passed
- `rubix-agent`: builds green; integration tests (`mcp_disk_test`,
  `mcp_stdio_test`, `rest_disk_test`, `cli_disk_test`,
  `changelog_middleware_test`, `authz_gate_test`,
  `bootstrap_user_test`) all green pre-rebase. Re-run after the
  agent-runtime job lands its three blocks.

### Smoke test status

Last run before PR #29 fixed B1–B4 — see
[2026-05-24-smoke-test-pr28.md](./2026-05-24-smoke-test-pr28.md).
**Not re-run since the fixes merged.** Top of the next session's
work, if no new codeless work is started, is to verify the smoke
runs clean post-#29.

```bash
mani run dev-deps-down
docker volume rm rubix-dev-postgres-data rubix-dev-clickhouse-data 2>/dev/null
mani run demo
# then steps 2–6 from THIN-SLICE.md §"Success criterion"
```

---

## 3. The agent-runtime job in detail

This is the in-flight piece. The thin slice landed (PRs 1–5 +
demo wiring + smoke fixes) but the MCP demo is partially
**fake** — `boot/mcp.rs` registers a hand-rolled flow with a
fake `com.rubix.diag-render` `NodeBehavior` that returns
hardcoded Spanish strings. There is no real LLM reasoning loop
behind the MCP surface today.

The agent-runtime job closes that gap:

| Block | What it lands | Where |
|---|---|---|
| **A** | `rubix-flows/src/load.rs` parses all six bundled YAMLs into real `FlowBody`s; `boot/mcp.rs` registers all six via `FlowRegistry::register`; hand-rolled flow + fake `diag-render` deleted | rubix — **already on the worktree branch as commit `542b918`** |
| **B** | New upstream crates `starter-ai-agent` (runner-agnostic `AgentLoop` primitive) + `starter-flow-node-loop` (thin `NodeBehavior` wrapper); `LONG-TERM.md` for deferred concerns (sessions, cost-cap, cancel, streaming, skills) | starter upstream — not yet in worktree |
| **C** | `rubix-agent/src/boot/ai.rs` returns `Arc<dyn AiRunner>` (default `ClaudeRunner`); `boot/mcp.rs` registers `AiAgentNode` so bundled flows fire real loops | rubix — not yet in worktree |

The full spec is at
[`.codeless/jobs/rubix-agent-runtime/`](/home/user/code/rust/starter/.codeless/jobs/rubix-agent-runtime/) —
three files (`SCOPE.md`, `template.yaml`, `WORKFLOW.md`)
totalling ~840 lines. SCOPE.md has a **30-line pre-drafted Rust
sketch** for the `AgentLoop` + `AiAgentNode` APIs so codeless
implements against a fixed contract.

### Why Block A landed but the job stopped

Codeless ran stage 0 (Block A work), committed it as `90165bf`
(now `542b918` after rebase), and then the runtime marked stage 1
(REVIEW block A) as `failed` with `failure_class:
pre-check-failed`. **Cost was $0** — the stage barely ran.

This is the same diff-verify pre-check trap that hit the
rubix-thin-slice-v2 job: the `Done`-doc handover used shell
brace-expansion in path lists (e.g. `routes/{mod.rs,tools.rs}`),
which the pre-check can't match against literal paths.

The upstream bug is filed at
[`/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`](/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md).

**Resolution:** restart the job (see §6). On restart, codeless
should re-run the review checklist (cheap, no code change), pass
it this time if the handover paths are listed individually, and
proceed to Block B.

---

## 4. Known codeless gotchas (operator-side)

These are pitfalls I hit and burned time on. The next operator
should not repeat them.

| # | Gotcha | Workaround |
|---|---|---|
| G1 | **Diff-verify pre-check rejects brace-expanded paths** in `Done`-doc handovers. Marks otherwise-correct REVIEW gates as `failed`. | Tell the agent in `WORKFLOW.md` to list every path individually. No `{a,b}.sql`, no globs, no leading `./`. Both v2 and agent-runtime job specs already include this rule. |
| G2 | **Pre-creating `.codeless/jobs/<name>/`** on disk causes the next `submit_job` to 409. | Never pre-create. Write spec files to `/tmp/` and submit via RPC; the server seeds the directory and you overlay SCOPE/WORKFLOW via `write_job_file`. |
| G3 | **Job rows can't be cleanly deleted** (`delete_job` returns FK violation). The worktree + branch can be removed by hand; the row stays. | Don't worry about the orphan rows; they're harmless once `status=stopped` and `ended_at` is set. Job names can be reused. |
| G4 | **`workspace_mode: in-repo` will commit straight onto your source tree's branch.** Hard rule 1 in ADDING-JOB.md. | Always pass `"workspace_mode": "worktree"` and a `"branch": "codeless/<job-name>"`. |
| G5 | **`mani` runs `cmd` from the *project* directory, not the repo root.** Path `rubix/docker/X` inside a task on `projects.rubix` becomes `rubix/rubix/docker/X` and breaks. | Make paths relative to the project root. Caught as bug B1 in the smoke test. |
| G6 | **The agent burns context re-discovering the same upstream APIs.** | Pre-draft API sketches in SCOPE.md (as done for `AgentLoop`); name the exemplar file path + line numbers. |
| G7 | **A `Done`-doc listing a path the agent "ran" but didn't modify** also trips the diff-verify. | Tell the agent in WORKFLOW.md that paths in `Done` mean "files I modified," not "files I touched / scripts I executed." |

---

## 5. Adding a new codeless job — the canonical flow

The mechanics are documented upstream at
[`/home/user/code/rust/codeless-workspace/codeless/setup/ADDING-JOB.md`](/home/user/code/rust/codeless-workspace/codeless/setup/ADDING-JOB.md).
This section is the rubix-specific operator runbook — what's
worked twice now, what to copy.

### Step 1 — Write the three spec files to `/tmp/`

Do NOT write them into `.codeless/jobs/<name>/` directly. The
server creates that directory.

```
/tmp/job-SCOPE.md       — the per-job brief; points at THIN-SLICE.md
                          as authoritative; lists "what's already
                          landed" + "what's already answered" + the
                          three blocks (in scope) + non-goals
                          (carve-outs) + acceptance criteria per
                          block + the BLOCKED escape hatch.
/tmp/job-template.yaml  — name + goal + stages[]. Each work stage
                          is ONE long sentence packed with concrete
                          deliverables. REVIEW stages alternate.
/tmp/job-WORKFLOW.md    — sequencing diagram, per-stage discipline,
                          anti-patterns, the closing-trio block
                          (mandatory; copy from ADDING-JOB.md Step 3),
                          references list.
```

**Exemplars to copy religiously:**

- The three job spec files at
  [`.codeless/jobs/rubix-demo-wiring/`](/home/user/code/rust/starter/.codeless/jobs/rubix-demo-wiring/)
  (this one landed cleanly as PR #28 — known-good shape).
- The three at
  [`.codeless/jobs/rubix-agent-runtime/`](/home/user/code/rust/starter/.codeless/jobs/rubix-agent-runtime/)
  (currently mid-flight; demonstrates the pre-drafted API sketch
  pattern in SCOPE.md).
- The starter convention at
  [`.codeless/jobs/flow-nodes/`](/home/user/code/rust/starter/.codeless/jobs/flow-nodes/)
  (the canonical reference).

### Step 2 — Resolve `repo_id` and submit as draft

The server resolves workspace name → repo_id. There's one
workspace attached (`starter`).

```bash
REPO_ID="01KS7BBHGPQNC1EDPD8E440204"  # the starter workspace

python3 <<'PY' >/tmp/submit.json
import json
print(json.dumps({
  "repo_id":           "01KS7BBHGPQNC1EDPD8E440204",
  "prompt":            None,
  "template_yaml":     open("/tmp/job-template.yaml").read(),
  "runner":            "claude",
  "branch":            "codeless/<job-name>",
  "workspace_mode":    "worktree",
  "cost_cap_cents":    5000,        # $50 for a 3-block job typical
  "wall_clock_cap_ms": 10800000,    # 3 hours typical
  "start_immediately": False        # ← ALWAYS false; you start manually
}))
PY

RESP=$(curl -s -X POST http://127.0.0.1:7777/rpc/submit_job \
  -H 'content-type: application/json' --data @/tmp/submit.json)

JOB_ID=$(echo "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
echo "JOB_ID=$JOB_ID"
```

The submit returns a `draft` job. The server has now created
`.codeless/jobs/<job-name>/` with placeholder SCOPE.md +
WORKFLOW.md + the template.yaml you sent. The branch
`codeless/<job-name>` will be allocated when the job starts.

### Step 3 — Overlay the real SCOPE + WORKFLOW

The placeholders need to be replaced with your real files via
`write_job_file`:

```bash
for f in SCOPE.md WORKFLOW.md; do
  python3 -c "
import json
print(json.dumps({
  'job_id':   '$JOB_ID',
  'filename': '$f',
  'content':  open('/tmp/job-$f').read()
}))" > /tmp/wf.json
  curl -s -X POST http://127.0.0.1:7777/rpc/write_job_file \
    -H 'content-type: application/json' --data @/tmp/wf.json
done
```

### Step 4 — Verify the draft

Before showing the user the UI link, sanity-check the state:

```bash
curl -s -X POST http://127.0.0.1:7777/rpc/get_job \
  -H 'content-type: application/json' -d "{\"job_id\":\"$JOB_ID\"}" \
  | python3 -m json.tool \
  | grep -E '"status"|"branch"|"workspace_mode"|"cost_cap_cents"|"wall_clock_cap_ms"|"started_at"'
```

Expect:
```
"status":         "draft"
"branch":         "codeless/<job-name>"
"workspace_mode": "worktree"
"cost_cap_cents": 5000
"started_at":     null
```

Then show the user:

```
UI link: http://localhost:1420/jobs/<JOB_ID>?workspace=01KS7BBHGPQNC1EDPD8E440204&tab=chat
```

### Step 5 — Start when the user says go

```bash
curl -s -X POST http://127.0.0.1:7777/rpc/start_job \
  -H 'content-type: application/json' \
  -d "{\"job_id\":\"$JOB_ID\"}"
```

**Default discipline: never call `start_job` without explicit
"start it" / "go" from the user.** This has bitten me; the user
always wanted draft-first review.

---

## 6. Restarting a paused / stopped job

The agent-runtime job at `01KSBTCGYB1GVVKX4GG3E3XHAC` is the
case in point. It's `stopped`, has a worktree at
`/home/user/.codeless/worktrees/job-01KSBTCGYB1GVVKX4GG3E3XHAC`,
and the branch was rebased onto master and force-pushed.

**Before restarting, rebase if master has moved since the job
last ran:**

```bash
cd /home/user/.codeless/worktrees/job-<JOB_ID>
git fetch origin master
git rebase origin/master      # resolve any conflicts here
git push --force-with-lease origin codeless/<job-name>
```

For the current agent-runtime job, the rebase already happened
cleanly (zero conflicts) and the push succeeded. The branch is
ready.

**Then restart via RPC:**

```bash
curl -s -X POST http://127.0.0.1:7777/rpc/start_job \
  -H 'content-type: application/json' \
  -d '{"job_id":"01KSBTCGYB1GVVKX4GG3E3XHAC"}'
```

Codeless picks up at the next un-passed stage (stage 1, the
REVIEW gate that failed). It re-runs the review checklist
against the rebased commit; if the handover paths are clean
this time (i.e. listed individually, no brace expansion), the
gate passes and codeless proceeds to Block B.

**Confirm with the user before restarting.** It's their cost
cap.

---

## 7. Cleanup — when a job is done or you want to discard

Three resources need attention; not all RPCs work cleanly.

### When a job merges successfully (e.g. PR #28)

- Leave the job row in the DB as `completed`.
- The worktree at `/home/user/.codeless/worktrees/job-<JOB_ID>`
  is harmless but can be removed with
  `git worktree remove <path>` from anywhere in the repo. Add
  `--force` if codeless runtime scratch (`runs/`) is untracked.
- The branch `codeless/<job-name>` can be deleted locally and
  on origin once the PR is merged.

### When you want to abandon a job

- `curl -X POST .../rpc/stop_job -d '{"job_id":"<JOB_ID>"}'` —
  returns `null` on success (real method despite the empty
  response).
- `delete_job` returns an FK error; the row stays. Don't worry.
- `git worktree remove --force <worktree-path>` to clean the
  worktree.
- `git branch -D codeless/<job-name>` + `git push origin
  --delete codeless/<job-name>` to remove branches.
- The name is now reusable for a new `submit_job`.

This is the path the rubix-thin-slice v1 job took — it ran $2.65
worth of redundant work, was abandoned, and v2 reused the same
shape under a different branch.

---

## 8. What the next session might pick up

Three plausible threads, ordered by my best read of priority:

### Thread 1 — Re-run the smoke test post-PR-#29

The B1–B4 fixes merged; nobody has re-run the six-step demo to
confirm all six PASS. Top of the list because it's small and
either confirms reality or surfaces a new bug.

**~30 min.** Re-uses the smoke prompt at the end of
[2026-05-24-smoke-test-pr28.md](./2026-05-24-smoke-test-pr28.md)
(or write a fresh one if cleaner).

### Thread 2 — Restart the agent-runtime job

The worktree is rebased and ready. Restart via `start_job` and
let codeless work through Block B (upstream `starter-ai-agent`
+ `starter-flow-node-loop`) and Block C (wire AiRunner in
rubix-agent). Cost cap is $50, wall-clock cap 4 hours.

**Operator-time: 5 min to restart + check.** Codeless does the
real work.

### Thread 3 — File the codeless diff-verify bug as a real upstream issue

The bug doc at
[`/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`](/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md)
exists but I never confirmed if codeless has a tracker / GitHub
repo to file it against. If yes, port the bug doc into a real
issue; if no, leave the local doc and link it from any future
codeless job spec.

**~15 min** + repo discovery.

### What NOT to start

- ❌ **New goal coverage** (the other 25 verb stubs in
  `rubix-tools`). Premature — the agent-runtime job needs to
  land first so each new tool has a real loop to be dispatched
  from. Pre-spec'd in
  [GAPS.md](../scope/GAPS.md) but explicitly post-thin-slice.
- ❌ **Extensions** (PR 5 deferred). Needs `starter-ext-flow`
  upstream first. Tracked.
- ❌ **OAuth, dashboards, flow-programmer tools, analytics
  reports, user-admin tools.** Per the thin-slice non-goals.

---

## 9. Hard rules to internalise (one more time)

From `HOW-TO-CODE.md`, `FILE-LAYOUT.md`, `SCOPE.md`, repeated
because every session forgets at least one:

- **One verb per file.** ≤400 lines hard, ~100 typical. No
  `utils.rs` / `helpers.rs` / `common.rs` / `misc.rs`.
- **Doc-tier rule.** Code comments link
  `docs/design/<area>/README.md` only. Never `SCOPE.md`,
  `HOW-TO-CODE.md`, `NEW-SESSION.md`, `FILE-LAYOUT.md`,
  `docs/scope/`, or `docs/sessions/`. `./rubix/scripts/lint-doc-refs.sh`
  enforces it.
- **No phasing markers in code.** No `// Phase 0`, `//
  STAGE-1 done`, `// FIXED:`. The lint doesn't catch these;
  review must.
- **Upstream-first.** If a capability could benefit any other
  starter consumer, file the upstream item before adding it to
  rubix. R2.
- **Tool outputs are `Diagnostic` + structured data**, never
  pre-formatted strings.
- **Catalogue files are the source of truth for MessageKeys.**
  No `MessageKey` constant in Rust without matching entries in
  `en.json` AND `es.json`.
- **Skill bodies + tool descriptors stay EN canonical.**
- **Tests live with the code in the same PR.**
- **Comments explain why, not what. No emojis.**

---

## 10. The actual prompt for the next session

Paste this block to a fresh agent.

```
You are starting a new coding session on the rubix project.

First, read these files in order, fully, no skimming:

  1. /home/user/code/rust/starter/rubix/NEW-SESSION.md
  2. /home/user/code/rust/starter/rubix/HOW-TO-CODE.md
  3. /home/user/code/rust/starter/rubix/FILE-LAYOUT.md
  4. /home/user/code/rust/starter/rubix/SCOPE.md
  5. /home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md
  6. /home/user/code/rust/starter/rubix/docs/sessions/2026-05-24-handover-codeless-orchestration.md
  7. /home/user/code/rust/starter/rubix/docs/sessions/2026-05-24-smoke-test-pr28.md
  8. /home/user/code/rust/starter/rubix/docs/design/agent/README.md
  9. /home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md

The handover (file 6) is the most important. It tells you:
  - What's landed on master (PRs 1–5 + demo wiring + smoke fixes)
  - What's in flight (codeless job 01KSBTCGYB1GVVKX4GG3E3XHAC
    "agent-runtime" — paused on rebased branch, ready to restart)
  - Three plausible next threads ordered by priority

Before you start coding or submitting a new codeless job, ask
the user:

  "Which thread do you want to pick up?
   (A) re-run the smoke test post-PR-#29
   (B) restart the agent-runtime codeless job
   (C) file the diff-verify bug upstream
   (D) something else"

Do NOT call /rpc/start_job without explicit confirmation from
the user. The default discipline is draft-and-show, then user
says go.

When in doubt: read the handover §4 (codeless gotchas) and §5
(canonical job-submission flow). The mechanics are not in
HOW-TO-CODE.md; the handover is the operator runbook.

Hard rules you must internalise:
  - Doc tiers. Code comments reference docs/design/ only.
  - File layout. ≤ 400 lines per file hard, ~100 typical.
  - Upstream first (R2).
  - Skills and tool descriptors stay EN canonical.
  - Tool outputs are Diagnostic + structured data, never strings.
  - Tests live with the code in the same PR.
  - No phasing markers, no emojis in code.

Don't start writing until you've read all 9 files and asked
which thread.
```

---

## 11. Anything else worth knowing

- **The starter workspace is the only one attached** to the local
  codeless server (repo_id `01KS7BBHGPQNC1EDPD8E440204`).
- **Codeless server runs on `127.0.0.1:7777`**; UI at
  `localhost:1420`.
- **Job branches are namespaced** `codeless/<job-name>` and
  always pushed to `origin`. They're force-pushable with
  `--force-with-lease` after rebase.
- **Worktrees live under `~/.codeless/worktrees/job-<JOB_ID>/`**.
  They're not in the repo proper; `git worktree list` from the
  repo shows them all.
- **The bug doc at
  `/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`**
  documents the diff-verify pre-check trap; every new job
  WORKFLOW.md should remind the agent to list paths individually.
- **`rubix-old/`** is the previous incarnation of the project.
  Don't read; don't copy. Read the SCOPE notes from PR #27 if
  you need archaeological context.
- **`mani` runs `cmd` from the *project* directory** (`rubix/`),
  not the workspace root. Caused bug B1; fixed in PR #29.
- **Phase 0 binary uses `RUBIX_BIND`** because the dev host
  often has port 8080 occupied.
- **`cargo run -p rubix-agent` stays unambiguous** thanks to
  `default-run = "rubix-agent"` in `rubix-agent/Cargo.toml`.
