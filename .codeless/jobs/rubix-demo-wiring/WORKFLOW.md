# Workflow — rubix-demo-wiring

How to drive the stages in `template.yaml`. Read this before
every stage alongside `SCOPE.md`, the authoritative source SCOPE
at
[`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md),
and the latest session handoff under
[`/home/user/code/rust/starter/rubix/docs/sessions/`](/home/user/code/rust/starter/rubix/docs/sessions/).

## Sequencing

Three work stages, three REVIEW gates between them. Strictly
linear; no parallel stages.

```
stage 1 (block A, binary wiring)     ── REVIEW ──
stage 2 (block B, runtime deps)      ── REVIEW ──
stage 3 (block C, Claude Desktop)    ── REVIEW
```

- **Block A** unblocks Block B (the `mani run run` task introduced
  in Block B points at the new `starter-config` loader from Block
  A; the bootstrap-user task assumes the auth router from Block A
  exists for the resulting user to be useful).
- **Block B** unblocks Block C (the `mcp_stdio_test` integration
  test from Block C needs a Postgres available — that's Block B's
  `dev-deps` task).
- **Block C** is the demo seam: HTTP MCP from Block A + stdio MCP
  from Block C together make step 4 of the THIN-SLICE smoke
  reachable from both `curl` and Claude Desktop.

A REVIEW gate exists between every pair because each block adds
load-bearing wiring that, if wrong, contaminates the next:

- **After block A** — a wrong authz gate placement (gate on the
  whole app instead of just the tools router) silently breaks
  the auth router by 401-ing the login endpoint.
- **After block B** — a docker-compose with `latest` tags or
  un-namespaced volumes makes the demo non-reproducible across
  machines and across re-runs.
- **After block C** — anything writing to stdout before the MCP
  framing kicks in (tracing init, println debugging, env-var
  echo) corrupts the JSON-RPC stream and Claude Desktop will
  close the connection silently.

## Per-stage discipline

Before writing any code in a stage:

1. **Re-read the corresponding block in the source SCOPE.** The
   stage text in `template.yaml` is the contract; this `WORKFLOW`
   is the process.
2. **Re-read `SCOPE.md` §"In scope", §"Out of scope", §"What is
   already landed", §"What is already answered", and §"The
   current binary gap (concrete)".** The biggest risk on this
   job is re-doing work that already happened on PR #27. Master
   at `912667e` is the truth.
3. **Run `cargo check --workspace` from the workspace root before
   any edit** so the baseline is known-clean.
4. **For block A**: read
   `/home/user/code/rust/starter/examples/authz-demo/src/main.rs`
   end-to-end before mounting the auth router. The whole exemplar
   is one file and shows the `auth_router` + `AuthState` +
   `starter_authz` policy wiring in context.
5. **For block B**: `ls /home/user/code/rust/starter/docker/`
   shows the existing compose convention. Match the file-naming
   pattern (`docker-compose.<purpose>.yml` lowercase, hyphens).
6. **For block C**: read `crates/starter-mcp/src/...` (the
   public `stdio_server` / equivalent function) before writing
   `mcp/serve.rs`. The framing comes from starter; do NOT
   reimplement. Verify by grepping the new serve.rs for any
   `serde_json::to_writer` or `tokio::io::AsyncWriteExt` direct
   write — those are signs of hand-rolled framing.

## During a stage

- **One file per verb. ≤400 lines hard, ~100 typical.** If a
  file approaches 300, stop and split.
- **Doc-tier rule.** Code comments reference
  `docs/design/<area>/README.md` only.
  `./rubix/scripts/lint-doc-refs.sh` enforces this — run it
  before closing a stage. Forbidden patterns to grep for:
  ```
  SCOPE\.md | HOW-TO-CODE\.md | NEW-SESSION\.md | FILE-LAYOUT\.md
  docs/scope/ | docs/sessions/
  ```
- **No phasing markers.** No `// Phase 0`, `// STAGE-1 done`,
  `// FIXED:`, `// Previously this used X`. The lint does not
  catch these; review must.
- **TODOs carry an owner or upstream tag.** `// TODO(name): ...`
  or `// TODO(upstream: <issue>): ...`. Never bare TODOs.
- **`Done`-doc handover paths must be listed individually** — no
  shell brace expansion (`{a,b}.sql`), no globs (`*.rs`), no
  leading `./`. The runtime's diff-verify pre-check is strict;
  see the workaround note in SCOPE.md §"Hard rules" and the
  upstream bug at
  `/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`.
  Two of three REVIEW gates on the v2 job failed because of
  this; do not repeat the mistake.

## When stuck

Codeless cannot ask the human. The escape hatch:

1. Stop work on the current block immediately.
2. Open the PR anyway with whatever compiles.
3. Add `BLOCKED: <one-line question>` to the PR description plus
   a paragraph explaining what was tried.
4. Move to the next block only if it does not depend on the
   blocked one. Otherwise stop and wait.

The human reviews the blocked PR and answers. Codeless does not
guess to unblock itself.

## At a REVIEW gate

The human runs through the REVIEW stage's checklist literally.
Each item is a one-line shell command, grep, or file read. If any
item fails, the previous stage's PR is amended (NOT replaced —
the next stage stays paused) until the checklist is green.

A REVIEW that passes triggers the next stage with the same
discipline.

## When the job is finished

When stage 3 lands and its REVIEW gate passes, **codeless stops**.
The job is NOT finished until the human runs the six-step manual
smoke in
[`rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
§"Success criterion":

```bash
mani run demo                                 # boot — dev-deps + bootstrap + run
curl -c cookies.txt -X POST \
  http://127.0.0.1:8088/api/v1/auth/login \
  -d '{"email":"op@example.com","password":"rubix-dev"}'

curl -b cookies.txt -H "Accept-Language: es-AR" \
  -X POST http://127.0.0.1:8088/api/v1/tools/rubix.system.disk \
  -d '{}'

# (Claude Desktop config from rubix/dev/claude-desktop.example.json
#  pasted into claude_desktop_config.json — restart Claude Desktop —
#  call the rubix tool from a conversation)

psql postgres://rubix:rubix-dev@127.0.0.1:5433/rubix \
  -c "SELECT actor, action, kind FROM changelog ORDER BY at DESC LIMIT 5"

clickhouse-client --port 9001 --database rubix \
  -q "SELECT * FROM system_disk_history ORDER BY epoch_ms DESC LIMIT 5"
```

If any step fails, the human files a one-line issue per failure;
codeless takes a follow-up job to fix.

## Anti-patterns specific to this job

- **Leaving `let _mcp_router = ...` in `main.rs`.** That's the
  exact line the job is supposed to delete. If it survives stage
  1, the demo cannot work.
- **Mounting the authz gate on the WHOLE app instead of just the
  tools router.** This 401s the login endpoint and breaks step 2.
  The gate wraps `routes::tools::router` only.
- **Hand-rolling JSON-RPC framing in `mcp/serve.rs`.** The
  framing lives in `starter-jsonrpc-stdio` and is wrapped by
  `starter-mcp`. If you find yourself calling
  `tokio::io::AsyncWriteExt::write_all` from `mcp/serve.rs`,
  stop — you've slipped the contract.
- **Writing to stdout before the MCP loop starts** (tracing init
  to stdout instead of stderr, `println!` debugging, env-var
  echo). Stdout is sacred in stdio MCP mode; corrupting it
  silently breaks Claude Desktop.
- **Hardcoding `localhost` or `127.0.0.1` in `agent.toml`** so
  the demo can't be aimed at a remote DB. Use config keys; the
  dev file fills them in for the local case.
- **Using `latest` image tags in the docker-compose.** Pin every
  image to a specific tag; un-pinned images make `mani run demo`
  non-reproducible across days.
- **Reading SCOPE.md or HOW-TO-CODE.md from source code
  comments.** The lint catches the obvious forms; review catches
  the rest.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's `verify:` list (or `verify_cmd`).
   Every step must pass. On failure: stop, fix, re-run; do not
   advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, in the same worktree, so the fresh agent that opens
   the next stage has the context it needs (per SCOPE Constraint 2:
   anything that must survive a stage boundary is on disk, not in
   the agent's head).
3. `git` — stage the changes (`git add -A` from the worktree root,
   or specific paths if the stage was surgical), commit with the
   message `stage N: <one-line title from template.yaml>` so the
   history mirrors the template stages one-for-one, and push to
   the job's branch (`codeless/rubix-demo-wiring`) so the work
   is recoverable even if the worktree is wiped.

A stage is not "done" until all three todos are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry — do
not mark the stage `[x]`, do not advance, and never `--force` or
`--no-verify`. If a stage genuinely produced no change (e.g. an
investigation stage that only updated `SCOPE.md` and that doc was
already current), say so in the handover and mark `git` as
`skipped — no diff`, but the next stage's commit must include any
side-effect files the investigation touched.

When listing paths in the `Done` block of any handover or run
note, **list every touched file on its own line**. Do NOT use
shell brace expansion (`{a,b}.sql`), globs (`*.rs`), or leading
`./`. The runtime's diff-verify pre-check is strict and will
reject the stage with a misleading `failed` status if it can't
match the path literally. (See SCOPE.md §"Hard rules" and the
upstream bug at
`/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`.)

## References

- Source SCOPE:
  [`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
- Most recent session handoff: under
  [`/home/user/code/rust/starter/rubix/docs/sessions/`](/home/user/code/rust/starter/rubix/docs/sessions/) —
  read the latest-numbered file.
- Per-job scope: `./SCOPE.md`
- Rubix architecture:
  [`/home/user/code/rust/starter/rubix/SCOPE.md`](/home/user/code/rust/starter/rubix/SCOPE.md)
- Contributor entry point:
  [`/home/user/code/rust/starter/rubix/HOW-TO-CODE.md`](/home/user/code/rust/starter/rubix/HOW-TO-CODE.md)
- File-layout rules:
  [`/home/user/code/rust/starter/rubix/FILE-LAYOUT.md`](/home/user/code/rust/starter/rubix/FILE-LAYOUT.md)
- Session boot:
  [`/home/user/code/rust/starter/rubix/NEW-SESSION.md`](/home/user/code/rust/starter/rubix/NEW-SESSION.md)
- Upstream PR ledger:
  [`/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md`](/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md)
- Diff-verify bug to work around:
  [`/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`](/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md)
