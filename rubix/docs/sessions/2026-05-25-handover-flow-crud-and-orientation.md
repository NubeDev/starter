# 2026-05-25 — Handover: flow CRUD + rubix / starter / codeless orientation

> **Tier:** session note. Lifetime: days. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source code must
> never reference this file.**

This is a context-setting handover for the next session, not a
job spec. Two threads:

1. **What's already on disk for "flow CRUD".** A surprising
   amount; reading this note before drafting any new job
   prevents re-creating what exists.
2. **The operator runbook** — how rubix, starter, the codeless
   workspace, and the docs all relate; how to submit a codeless
   job; where to look for what.

## 1. Orientation — the four moving parts

### 1.1 The four code roots on this machine

```
/home/user/code/rust/starter/                <- the main monorepo (this repo)
├── crates/                                  <- starter-* Rust libraries (≈ 80+ crates)
├── packages/                                <- starter-* TypeScript + React packages
├── starter-extensions/                      <- sibling workspace (sister to starter)
│   ├── crates/                              <-   starter-ext-* (host, supervisor, spi…)
│   └── packages/                            <-   starter-ext-ui, starter-ext-sdk-ts
├── rubix/                                   <- the *product* built on starter
│   ├── crates/                              <-   rubix-* Rust (agent, tools, flows…)
│   ├── packages/                            <-   rubix-client-ts, rubix-client-react
│   ├── frontend/                            <-   the React SPA the operator sees
│   ├── extensions/                          <-   bundled rubix extensions
│   └── docs/                                <-   *this file lives in docs/sessions/*
└── DOCS/                                    <- starter-side authoritative scopes
    ├── flow/scope/SCOPE.md
    ├── agent/SCOPE.md
    ├── tools/scope/SCOPE.md
    └── extensions/scope/SCOPE.md

/home/user/code/rust/codeless-workspace/codeless/
└── setup/                                   <- ADDING-JOB.md, GETTING-STARTED.md…
```

The hard split:

- **starter** is "small libraries you `cargo add`." The flow engine,
  the agent loop, every transport, every store. No product opinions.
- **rubix** is the *product* — one binary, one frontend, six bundled
  goal-flows, the dev-ops UX. It imports starter, never the other way.
- **starter-extensions** is a sibling workspace that ships the
  extension framework (loader, supervisor, host server, frontend
  federation runtime). Lives outside `starter/` so consumers who
  don't want extensions don't depend on it.
- **codeless** is *external to this repo entirely* — a separate
  workspace under `~/code/rust/codeless-workspace/codeless/`. It's the
  job-runner the operator drives this work through; it doesn't ship in
  any rubix or starter artifact.

### 1.2 Where each kind of document lives

| Tier | Path | Lifetime | Who reads |
|---|---|---|---|
| **Authoritative SCOPE** | `DOCS/<area>/scope/SCOPE.md`, `rubix/SCOPE.md` | years | every contributor; never cited from code |
| **Design (present-tense)** | `rubix/docs/design/<area>/README.md` | months | every contributor; **the only tier code may cite** |
| **Scope plans (rubix-specific)** | `rubix/docs/scope/THIN-SLICE.md`, `GAPS.md` | months | contributors and operators |
| **Session notes** | `rubix/docs/sessions/<date>-<topic>.md` | days–weeks | operator + the next session; **never cited from code** |
| **ADRs** | `rubix/docs/adr/NNNN-<title>.md` (when needed) | forever | when a non-obvious decision was made |
| **Codeless job specs** | `.codeless/jobs/<name>/{SCOPE,WORKFLOW}.md` + `template.yaml` | days–weeks (active), forever (history) | the codeless runtime |

Rule (R3 in `rubix/SCOPE.md`): **code comments link
`docs/design/<area>/README.md` only.** Never SCOPE.md, NEW-SESSION.md,
HOW-TO-CODE.md, FILE-LAYOUT.md, `docs/scope/`, or `docs/sessions/`.
`./rubix/scripts/lint-doc-refs.sh` enforces this; every codeless job
runs it.

### 1.3 The reading order for a fresh session

Non-negotiable in order:

1. `rubix/NEW-SESSION.md` — non-negotiables, doc-tier rule, smoke test.
2. `rubix/HOW-TO-CODE.md` — contributor entry point, decision tree, crate map.
3. `rubix/FILE-LAYOUT.md` — Rule Zero (verb-per-file, ≤400 lines).
4. `rubix/SCOPE.md` — R1–R13, the six goals, phases.
5. `rubix/docs/scope/THIN-SLICE.md` — what's lit up so far.
6. **This file.**
7. The session note relevant to whatever broke last — the latest
   `rubix/docs/sessions/2026-05-2*-*.md` covering your area.

After step 7 you can answer NEW-SESSION's §1 self-test.

## 2. State of play — what's merged

Master commit at handover: `f13f71d`. Twenty-plus PRs landed across
mid-May 2026:

| PR | What |
|---|---|
| #27 | thin-slice — Goal 5 (system check), MCP demo, ClickHouse, REST + CLI parity |
| #28 | demo wiring — `mani run demo` |
| #29 | smoke blockers (B1–B4) |
| #30 | agent-runtime — real Claude loop replaces hand-rolled flow |
| #31 | smoke followups (B5–B8.2 + engine quiescence tracker) |
| #32 | goals 2/3/4 lit up — user-admin, clickhouse-ruler, flow-programmer end-to-end with undo + PG `flows_definitions` + cross-instance NOTIFY |
| #34 | extensions wired — `starter-extensions` imported, lifecycle REST, PG enablement persistence |
| #35 | frontend wired — REST + SSE, auth, typed React hooks, `@nube/starter-client-react` + `@nube/rubix-client-react` |
| #36 | tool registry gap fix |
| #37 | frontend surfaces — flows, extensions, admin/users, admin/access, admin/warehouse |

Four real goals (2, 3, 4, 5); two stubbed (1 dashboards SDUI, 6
weekly-report). See `rubix/docs/scope/THIN-SLICE.md` "Goals lit up
beyond the thin slice" for the per-goal evidence table.

### 2.1 Two known bugs in the queue

Both diagnosed, prompts written, not fixed yet:

- **Auth path mismatch** — every authenticated REST call 401s because
  `@nube/starter-client-ts` calls `/auth/*` but the backend mounts
  `/api/v1/auth/*`. Prompt:
  [`2026-05-24-auth-path-mismatch-fix.md`](./2026-05-24-auth-path-mismatch-fix.md).
  Fix is three path changes in `auth.ts`.
- **Admin Tabs layout regression** — `/admin/access` and
  `/admin/warehouse` render with a grey ellipse around the tab list
  and tab content as a right-side column. Likely cause is Tailwind v4
  source-scan missing the `@nube/starter-ui-kit` package. Prompt:
  [`2026-05-24-admin-tabs-layout-fix.md`](./2026-05-24-admin-tabs-layout-fix.md).

If `make start` surfaces a third issue, write a similar diagnostic
session note first; do not let the next codeless job inherit
ambient bugs without naming them.

## 3. Flow CRUD — what's actually on disk

The operator asked "write a scope for me to CRUD a flow, values and
settings". Before drafting a job, read the inventory below; a lot
already exists.

### 3.1 Backend — what `flow_ops.*` does today

Source: `rubix/crates/rubix-tools/src/flow_ops/`.

```
flow_ops/
├── deploy.rs       <- POST /api/v1/tools/rubix.flow_ops.deploy
├── duplicate.rs    <- POST /api/v1/tools/rubix.flow_ops.duplicate
├── lint.rs         <- POST /api/v1/tools/rubix.flow_ops.lint
├── list.rs         <- POST /api/v1/tools/rubix.flow_ops.list
├── store.rs        <- FlowDefStore trait + PG impl (flows_definitions row)
├── validate.rs     <- semantic validation past YAML parse
└── mod.rs
```

PG storage: `flows_definitions` table (PR #32) with columns
`flow_id`, `revision_id`, `body_yaml`, `created_by`, `created_at`,
`superseded_at NULL`. Cross-instance NOTIFY via
`rubix_flows_definitions` channel; `boot/flow_notify.rs` listens and
reloads the `FlowRegistry` on event.

**What's NOT there** — the gap for full CRUD:

- `rubix.flow_ops.get(flow_id) -> { body_yaml, revision_id, deployed_at, supersession_count }`
  — frontend `useFlowDefinition` currently synthesises a placeholder
  graph because no endpoint returns the body. The detail route has a
  banner admitting this. **Highest-leverage gap.**
- `rubix.flow_ops.update(flow_id, body_yaml, expected_revision_id?)`
  — optimistic-concurrency update. Today operators must call
  `deploy` directly; an update verb makes the conflict semantics
  first-class.
- `rubix.flow_ops.delete(flow_id)` — soft-delete every revision (mark
  all `superseded_at = NOW()`). Bundled flows (from `include_dir!`)
  must be protected.
- `rubix.flow_ops.history(flow_id) -> [{ revision_id, deployed_at, deployed_by, superseded_at }]`
  — list prior revisions. The data exists; only the SELECT verb is
  missing.

### 3.2 Client (TS) — what's wired

`rubix/packages/rubix-client-ts/src/endpoints/flow_ops.ts` ships:

```
flowDeploy(req)     -> { summary, flow_id, revision_id, prior_revision_id?, deployed_at_ms }
flowLint(req)       -> { summary, errors: LintDiagnostic[] }
flowList()          -> { summary, count, flows: [{ flow_id, revision_id }] }
flowDuplicate(req)  -> { summary, source_flow_id, source_revision_id, target_flow_id, target_revision_id }
```

Wire shapes mirror `rubix-spi/src/dto/flow_ops/*`. CSRF is threaded
for every call (even reads, per the `/api/v1/tools/*` mount).

**What's missing** corresponds to §3.1 — `flowGet`, `flowUpdate`,
`flowDelete`, `flowHistory`.

### 3.3 Client (React) — hooks wired

`rubix/packages/rubix-client-react/src/hooks/flow-ops.ts` ships:

```
useFlowList()                     -> UseQueryResult<FlowListResponse>
useFlowLint(opts?)                -> UseMutationResult<...>
useFlowDeploy(opts?)              -> UseMutationResult<...>
useFlowDuplicate(opts?)           -> UseMutationResult<...>
useFlowDefinition(flowId, opts?)  -> UseQueryResult<{ revision_id, graph }>
```

`useFlowDefinition` is a synthetic placeholder pending the backend
`flow_ops.get`. It builds a `FlowGraph` shape from the list metadata
but **does not** return the real YAML body.

The hook file already exports a structural `FlowGraph` interface
mirroring `@nube/starter-ui-flow`'s shape — kept inline so the
transport package doesn't take a hard UI dep.

### 3.4 Frontend routes — what's mounted

`rubix/frontend/src/routes/flows/`:

```
index.tsx      <- /flows           list view (table; columns: flow_id, revision_id, last_deployed_at, supersession_count)
$flowId.tsx    <- /flows/<id>      read-only canvas via <FlowCanvas readOnly>
```

Both use `useFlowsList` / `useFlowDefinition` from
`@nube/rubix-client-react`. The detail route builds the
`NodeKindRegistry` once at module load via
`rubix/frontend/src/lib/flow-registry.ts`:

```ts
// flow-registry.ts: seeds from @nube/starter-ui-flow's BUILTIN_NODE_KINDS
//                   (ai-agent, tool-call, trigger, branch, transform, subflow)
//                   then overrides ai-agent with RubixAiAgentNode which
//                   renders skill_hint + allowed_tools[] as badges.
```

**The rubix-side override pattern is established.** New rubix-specific
node renderers land in `rubix/frontend/src/lib/flow-nodes/` and
register through `flow-registry.ts`. Do not modify
`@nube/starter-ui-flow` itself.

### 3.5 starter-ui-flow — what it actually does

Reading this saved ~half a day of "do we need to add editing?":

`packages/starter-ui-flow/src/canvas/FlowCanvas.tsx` already supports
authoring. `nodesDraggable={!readOnly}`, `nodesConnectable={!readOnly}`,
`onChange?: (graph: FlowGraph) => void`. Pass `readOnly={false}` and an
`onChange` handler and you have a working graph editor. `useFlowGraph`
is the hook the canvas uses internally; `useTypedConnect` validates
slot kinds on connect.

`NodePalette.tsx` renders a category-grouped list of registered kinds
with an `onPick(spec)` callback so the host can append a node.

`useFlowGraph` skips transient mutations (`dimensions`, `select`,
`dragging`) so `onChange` only fires on persistent changes — that's
the seam to wire dirty-tracking against.

**The full graph-edit experience is one route away** — pass
`readOnly={false}`, add a `<NodePalette>` slot, wire `onChange` to
local state plus a save button that calls `flowDeploy`.

### 3.6 Settings / values

"Values and settings" in the rubix flow YAML maps to:

**Per-flow root fields** (`rubix-flows::yaml::RubixFlowYaml`):
- `id`, `description`, `trigger` (`explicit` / `schedule`),
  `cron_expr` (when trigger is schedule)
- `nodes[]`, `links[]`

**Per-node `config` bag** (`RubixNodeYaml.config`, free-form YAML
mapping). The keys actually consumed today by `ai-agent` nodes:
- `session_policy` (`continue` / `fresh`)
- `skill_hint` (e.g. `com.rubix.system-checker`)
- `cost_cap` (e.g. `0.10_usd`)
- `allowed_tools[]` (list of reverse-DNS tool ids)

A "values and settings" editor exposes both. Today nothing renders
either as a form — they're invisible in the canvas unless the
node's custom component shows them (rubix's `RubixAiAgentNode`
renders `skill_hint` + `allowed_tools[]` count as badges; other
fields are not surfaced).

### 3.7 starter authoring substrate (what we get for free)

From `DOCS/flow/scope/SCOPE.md` and the registry code:

- `starter-flow::FlowRegistry::register` supports **multi-revision per
  flow**, immutable revisions. Already used by rubix's
  `flows_definitions` PG row pattern.
- `starter-flow::definition::DefinitionManager::publish` /
  `publish_delete` — the **one write chokepoint**. Every source
  (file watch, REST handler, NOTIFY listener) publishes through it.
- `starter-flow-watch` — file watcher that hot-reloads bundled flows
  on disk edits.
- `FlowAsTool` — every flow auto-surfaces as an MCP tool.
- `FlowAsService` — schedule trigger via durable PG `scheduled_flows`
  (PR for Goal 6 wiring exists; landed as part of goals-2-4-3 PR #32).

**Implication for the next job:** the "update a flow" path goes
through `flow_ops.deploy`/`update` → `FlowDefStore::insert_revision` →
`pg_notify('rubix_flows_definitions')` → `boot/flow_notify.rs` listener
→ `DefinitionManager::publish` → `FlowRegistry::register`. Every link
in that chain already exists; what's missing is the verb at the front
and the SELECT at the back (the GET endpoint).

## 4. The shape of the next codeless job

Don't write the job yet — write the spec for the next session to
write the job. The agenda:

### 4.1 Backend additions (rubix-side; small)

- `rubix.flow_ops.get` verb file under `rubix/crates/rubix-tools/src/flow_ops/get.rs` — SELECT latest non-superseded row, return `body_yaml` + metadata.
- `rubix.flow_ops.update` — semantic alias over deploy with optimistic concurrency via `expected_revision_id`.
- `rubix.flow_ops.delete` — supersede every revision. Refuse on bundled flow ids (`com.rubix.*` per `include_dir!`) — they reload from disk on restart anyway.
- `rubix.flow_ops.history` — SELECT all revisions for a flow id.
- Wire shapes in `rubix-spi/src/dto/flow_ops/`. MessageKeys per R5: `rubix.flow.fetched`, `rubix.flow.updated`, `rubix.flow.delete_refused_bundled`, `rubix.flow.deleted`, `rubix.flow.history_returned` — en + es catalogues same commit.
- Integration test in `rubix/crates/rubix-agent/tests/goal_3_flow_programmer_test.rs` (or a new sibling) covering create → get → update → history → delete → restore-via-undo.
- All four verbs auto-surface as tools via the existing tool registry — no MCP wiring.

### 4.2 Client-ts additions

Per-verb files mirroring §3.2. Each ≤ 80 lines, CSRF threaded.

### 4.3 Client-react additions

`useFlowDefinition` rewired to call the real `flowGet` instead of
synthesising. New `useFlowUpdate`, `useFlowDelete`, `useFlowHistory`
mutations / queries. Invalidations under `['rubix','flow_ops']`.

### 4.4 Frontend routes

- `/flows/$flowId` — gets an "Edit" affordance when the user is admin.
- `/flows/$flowId/edit` — new route. `<FlowCanvas readOnly={false} onChange={…}>`, `<NodePalette onPick={…}>`, "Save" button calling `flowUpdate(flow_id, body_yaml, expected_revision_id=current_revision_id)`. Conflict toast on 409.
- `/flows/$flowId/history` — new route. Lists prior revisions; click → loads that revision into a read-only canvas. "Restore" button → `flowDeploy` the prior body as a new revision.
- `/flows/new` — new route. Blank-or-duplicate-and-edit flow. "Save" → `flowDeploy`.
- Per-flow settings panel — sidebar on `/edit` rendering the root-level fields (description, trigger, cron_expr) as typed form inputs.
- Per-node settings panel — sidebar on `/edit` rendering the selected node's `config` bag (skill_hint, cost_cap, allowed_tools[]) as typed form inputs. Selection state comes from `<FlowCanvas>`'s xyflow selection.
- Bidirectional `FlowGraph ↔ body_yaml` conversion — one verb file per direction. The forward direction (`yaml → FlowGraph`) already exists in `useFlowDefinition`; the reverse (`FlowGraph → yaml`) is new. Both must roundtrip on the six bundled flows.

### 4.5 starter-side?

**Probably nothing.** `@nube/starter-ui-flow` already supports
editing, NodePalette, typed connect. The only candidate for an
upstream change is if the settings sidebar pattern (typed forms over
the per-node `config` bag) is common enough that a generic
`<NodeConfigForm>` belongs upstream. Default to no — author the
rubix-specific form in rubix; promote to upstream later if a second
consumer needs the same shape.

Verify at the start of the next session by re-grepping
`@nube/starter-ui-flow` for any "settings" or "config" component that
might already exist; this handover is dated and the package iterates.

### 4.6 Versioning + undo

Both already work backend-side. The history surface in §4.4 reads
existing data. `rubix.undo.last` already reverts the most recent
`flow_ops.deploy` per the goals-2-4-3 wiring. The new
`flow_ops.update` and `flow_ops.delete` register `Reversible` the
same way deploy does.

## 5. Codeless — operator runbook

### 5.1 Where everything lives

- **Codeless server** runs on `127.0.0.1:7777` (RPC) + UI at
  `localhost:1420`. Started outside this repo.
- **Workspace attached:** `starter` → `repo_id` `01KS7BBHGPQNC1EDPD8E440204`.
  Confirm with `curl -sX POST http://127.0.0.1:7777/rpc/list_workspaces -d '{}'`.
- **Worktrees** land under `/home/user/.codeless/worktrees/job-<JOB_ID>/`
  on a fresh branch `codeless/<job-name>`.
- **Setup docs:** `/home/user/code/rust/codeless-workspace/codeless/setup/`
  — `ADDING-JOB.md` is the canonical how-to, `GETTING-STARTED.md` the
  intro.

### 5.2 Submitting a job — the canonical flow

The mechanics from prior sessions (see
[`2026-05-24-handover-codeless-orchestration.md`](./2026-05-24-handover-codeless-orchestration.md)
§5 for the long-form runbook). Compressed here:

**Step 1 — write the three spec files to `/tmp/`.** Do NOT write
them into `.codeless/jobs/<name>/` directly; the server creates that
directory.

```
/tmp/job-SCOPE.md       <- what + why + constraints; points at the
                           authoritative SCOPE doc for the area
/tmp/job-template.yaml  <- name + goal + stages[] (each stage one
                           outcome, REVIEW gates at risky boundaries)
/tmp/job-WORKFLOW.md    <- per-stage discipline + closing-trio block
                           (mandatory; copy from ADDING-JOB.md Step 3)
```

YAML stages must be quoted strings — the codeless parser is strict.
Each stage is one long sentence packed with concrete deliverables.
REVIEW stages alternate between work blocks.

**Step 2 — submit as draft.**

```bash
REPO_ID="01KS7BBHGPQNC1EDPD8E440204"

python3 <<PY >/tmp/submit.json
import json
print(json.dumps({
  "repo_id":           "$REPO_ID",
  "prompt":            None,
  "template_yaml":     open("/tmp/job-template.yaml").read(),
  "runner":            "claude",
  "branch":            "codeless/<job-name>",
  "workspace_mode":    "worktree",
  "cost_cap_cents":    10000,        # $100 typical for a 10-15 stage job
  "wall_clock_cap_ms": 28800000,     # 8 hours
  "start_immediately": False         # ← ALWAYS false
}))
PY

RESP=$(curl -s -X POST http://127.0.0.1:7777/rpc/submit_job \
  -H 'content-type: application/json' --data @/tmp/submit.json)
JOB_ID=$(echo "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
```

**Step 3 — overlay the real SCOPE + WORKFLOW** (the server seeded
placeholders).

```bash
for f in SCOPE.md WORKFLOW.md; do
  python3 -c "
import json
print(json.dumps({
  'job_id':'$JOB_ID','filename':'$f','content':open('/tmp/job-'+'$f').read()
}))" > /tmp/wf.json
  curl -s -X POST http://127.0.0.1:7777/rpc/write_job_file \
    -H 'content-type: application/json' --data @/tmp/wf.json
done
```

**Step 4 — verify draft state.**

```bash
curl -s -X POST http://127.0.0.1:7777/rpc/get_job \
  -H 'content-type: application/json' -d "{\"job_id\":\"$JOB_ID\"}" \
  | python3 -m json.tool \
  | grep -E '"status"|"branch"|"workspace_mode"|"cost_cap_cents"|"started_at"'

# expect:
#   "status":         "draft"
#   "branch":         "codeless/<job-name>"
#   "workspace_mode": "worktree"
#   "started_at":     null
```

**Step 5 — show the operator the UI link and wait.**

```
http://localhost:1420/jobs/$JOB_ID?workspace=$REPO_ID&tab=chat
```

**Default discipline: never call `start_job` without the operator's
explicit "start it" / "go".** This has bitten every prior session;
the operator always wants draft-first review.

**Step 6 — start when authorised.**

```bash
curl -s -X POST http://127.0.0.1:7777/rpc/start_job \
  -H 'content-type: application/json' \
  -d "{\"job_id\":\"$JOB_ID\"}"
```

### 5.3 Known codeless gotchas

From accumulated history; tell the agent in WORKFLOW.md to avoid.

| # | Gotcha | Workaround |
|---|---|---|
| G1 | YAML stages must be quoted strings (not bare scalars). Bare scalars containing `:`, `[`, `<` break the parser. | Always wrap each `stages:` entry in `"..."` |
| G2 | Diff-verify pre-check rejects brace-expanded paths in `Done`-doc handovers (e.g. `routes/{mod.rs,tools.rs}`). Marks otherwise-correct REVIEW gates as `failed`. | Tell the agent in WORKFLOW.md to list every path individually. No globs, no leading `./`. |
| G3 | Pre-creating `.codeless/jobs/<name>/` on disk causes the next `submit_job` to 409. | Never pre-create. Write spec files to `/tmp/` and submit via RPC. |
| G4 | `delete_job` returns FK violation. Job rows stay forever in the DB. | Don't worry — harmless once `status=stopped` and `ended_at` is set. Names can be reused. |
| G5 | `workspace_mode: in-repo` commits straight onto your source tree's branch. | Always pass `"workspace_mode": "worktree"` and a `"branch": "codeless/<job-name>"`. |
| G6 | `mani` runs `cmd` from the *project* directory, not the repo root. Paths inside a mani task are relative to the project dir. | Caught as bug B1 in pr28 smoke; mani.yaml fixed. Still worth flagging. |
| G7 | Agent burns context re-discovering the same upstream APIs. | Pre-draft API sketches in SCOPE.md; name exemplar file paths + line numbers. |
| G8 | A `Done`-doc listing a path the agent "ran" but didn't modify trips diff-verify. | Tell the agent in WORKFLOW.md: paths in `Done` mean "files I modified", not "files I touched / scripts I executed". |
| G9 | Multiple commits per stage need explicit handling — the closing-trio `git` step means "all this stage's commits are made and pushed". | WORKFLOW.md must spell out which stages bundle which commits in what order. |

### 5.4 Restarting a paused / stopped job

Per the previous handover. If a job stops mid-flight (REVIEW gate
failure, cost cap hit, operator pause):

```bash
cd /home/user/.codeless/worktrees/job-<JOB_ID>
git fetch origin master
git rebase origin/master              # resolve conflicts here
git push --force-with-lease origin codeless/<job-name>

curl -s -X POST http://127.0.0.1:7777/rpc/start_job \
  -H 'content-type: application/json' \
  -d "{\"job_id\":\"<JOB_ID>\"}"
```

Codeless picks up at the next un-passed stage. Always confirm with
the operator before restarting — it's their cost cap.

### 5.5 Cleanup — when a job is done

Three resources need attention:

- **Job row** — leave as `completed`. `delete_job` doesn't work cleanly; the row is harmless.
- **Worktree** — `git worktree remove --force /home/user/.codeless/worktrees/job-<JOB_ID>`. Some old worktrees still on disk; cleanup is operator-discretion. See `git worktree list`.
- **Branch** — once the PR is merged, `git branch -D codeless/<job-name>` locally and `git push origin --delete codeless/<job-name>` remotely. `git remote prune origin` cleans stale tracking refs.

The name `codeless/<job-name>` is then reusable for a fresh job.

## 6. Reading order for the next session (in 30 seconds)

```
1. rubix/NEW-SESSION.md                                                  <- non-negotiables
2. rubix/HOW-TO-CODE.md                                                  <- contributor entry
3. rubix/SCOPE.md                                                        <- R1–R13
4. rubix/docs/scope/THIN-SLICE.md                                        <- what's lit up
5. THIS FILE                                                             <- you are here
6. rubix/docs/sessions/2026-05-24-frontend-surfaces.md                   <- where the frontend got to
7. rubix/docs/sessions/2026-05-24-auth-path-mismatch-fix.md              <- bug in the queue
8. rubix/docs/sessions/2026-05-24-admin-tabs-layout-fix.md               <- bug in the queue
9. rubix/docs/design/flows/README.md                                     <- the present-tense flow design
10. rubix/crates/rubix-tools/src/flow_ops/{deploy,duplicate,lint,list}.rs <- the verb files
11. rubix/packages/rubix-client-ts/src/endpoints/flow_ops.ts             <- the wire methods
12. rubix/packages/rubix-client-react/src/hooks/flow-ops.ts              <- the hooks
13. rubix/frontend/src/routes/flows/{index,$flowId}.tsx                  <- the routes
14. packages/starter-ui-flow/src/canvas/FlowCanvas.tsx                   <- editing already works upstream
15. /home/user/code/rust/codeless-workspace/codeless/setup/ADDING-JOB.md  <- if drafting a new job
```

## 7. The actual next-session prompt

Paste this to a fresh agent.

```
You are starting a new coding session on rubix at
/home/user/code/rust/starter.

Read these files in order, fully, no skimming:

  1. rubix/NEW-SESSION.md
  2. rubix/HOW-TO-CODE.md
  3. rubix/FILE-LAYOUT.md
  4. rubix/SCOPE.md
  5. rubix/docs/scope/THIN-SLICE.md
  6. rubix/docs/sessions/2026-05-25-handover-flow-crud-and-orientation.md
  7. rubix/docs/sessions/2026-05-24-auth-path-mismatch-fix.md
  8. rubix/docs/sessions/2026-05-24-admin-tabs-layout-fix.md
  9. rubix/docs/design/flows/README.md
  10. rubix/docs/design/agent/README.md

File 6 is the load-bearing handover; read it carefully. It documents:
  - The flow CRUD scope (§4) — backend verbs + client + react + frontend routes
  - The codeless runbook (§5) — how to submit and start jobs
  - The doc-tier convention rubix lives by

The two open bugs in the queue (files 7 + 8) are not the next
priority; flow CRUD is. But if `make start` surfaces either of
them blocking the work, fix that one first.

Before drafting any new codeless job, you must:
  1. Re-grep @nube/starter-ui-flow for any settings/config UI that
     might have landed since 2026-05-25. The handover is dated.
  2. Re-read rubix/crates/rubix-tools/src/flow_ops/*.rs to confirm
     no new verb has landed since the handover. The codebase moves
     fast.
  3. Ask the operator to confirm:
       a) authoring scope — full graph edit + per-node settings + per-flow settings? or settings-only?
       b) versioning UX — full history list + restore? or only undo.last?
       c) delete semantics — soft-delete supersedes all revisions? or no delete?

The default answers (Recommended in the prior session): (a) full,
(b) full history, (c) yes soft-delete with bundled-flow protection.

Hard rules you must internalise:
  - R1 — verb per file, ≤ 400 lines.
  - R2 — upstream-first. starter changes before rubix consumes.
  - R3 — code comments link docs/design/<area>/README.md only.
  - R4 — Diagnostic + structured data, never strings.
  - R5 — catalogue files are source of truth for MessageKeys.
  - R6 — tests live with the code in the same commit.

Do NOT call /rpc/start_job without explicit operator confirmation.
Default discipline: draft, show UI link, wait for "go".
```

## 8. Anything else worth knowing

- **Bootstrap user:** `op@example.com` / `rubix-dev-passwd` (admin).
  Created idempotently by `rubix/Makefile`'s `bootstrap` target.
- **Dev URLs:** rubix-agent at `127.0.0.1:8088`, vite dev server at
  `127.0.0.1:5185` (Makefile says 5173 — drift; verify before driving
  Playwright against it).
- **rubix-old/** is the previous incarnation of the project. Don't
  read; don't copy. Read PR #27 if you need archaeological context.
- **Phase 0 binary uses `RUBIX_BIND`** because the dev host often has
  port 8080 occupied.
- **The 9 / 15 stale codeless job names** under `.codeless/jobs/` are
  history. Reuse only after deleting the matching directory.
- The `2026-05-23-next-steps-*.md` series (5 files) is the Phase 2b
  history. Useful background but not actionable for flow CRUD.
- **`/extensions` route works**, lists the 1 bundled extension
  (`com.rubix.example`), supervisor lifecycle works. SSE event stream
  too. Worth confirming nothing regressed before driving a new job.
