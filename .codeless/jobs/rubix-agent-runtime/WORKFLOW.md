# Workflow — rubix-agent-runtime

How to drive the stages in `template.yaml`. Read this before
every stage alongside `SCOPE.md`, the authoritative source SCOPE
at
[`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md),
the starter agent SCOPE at
[`/home/user/code/rust/starter/DOCS/agent/SCOPE.md`](/home/user/code/rust/starter/DOCS/agent/SCOPE.md),
and the latest session handoff under
[`/home/user/code/rust/starter/rubix/docs/sessions/`](/home/user/code/rust/starter/rubix/docs/sessions/).

## Sequencing

Three work stages, three REVIEW gates between them. Strictly
linear; no parallel stages.

```
stage 1 (block A, YAML loader)     ── REVIEW ──
stage 2 (block B, upstream crates) ── REVIEW ──
stage 3 (block C, wire runtime)    ── REVIEW
```

- **Block A** lands the YAML loader and deletes the hand-rolled
  flow. After A, the bundled flows are registered as MCP tools
  but **fail on invocation** because no behaviour is bound to
  the `ai-agent` kind. This is expected and is the signal that
  the loader is wired correctly.
- **Block B** is upstream-only (no rubix code touched). Lands
  `starter-ai-agent` (the primitive) + `starter-flow-node-loop`
  (the wrapper). Both crates ship with their own tests against
  a `MockAiRunner`; B can be merged-and-tested without touching
  rubix.
- **Block C** wires Block B's `AiAgentNode` into rubix-agent's
  boot path. After C, the bundled flows fire real Claude CLI
  loops and the same tests that passed against the hand-rolled
  fake pass against the real runtime.

The dependency chain is hard: A → B → C. A blocked stage blocks
the whole job. There is no parallel work to fall back to.

A REVIEW gate exists between every pair because each block adds
load-bearing wiring that, if wrong, contaminates the next:

- **After block A** — a YAML loader that silently drops a flow
  (e.g. a parser that returns `Ok` on a malformed file) means
  Block C wires an empty registry and the demo silently passes
  with `mcp_tools=0`.
- **After block B** — a violation of the two-crate split (e.g.
  `starter-ai-agent` accidentally pulling `starter-flow-spi`)
  defeats the whole architectural commitment. Cargo tree must
  prove the layers are clean.
- **After block C** — a recorded-LLM fixture that doesn't match
  the current loop output means the test is asserting a fiction.
  Re-recording on every drift hides regressions.

## Per-stage discipline

Before writing any code in a stage:

1. **Re-read the corresponding block in the source SCOPE.** The
   stage text in `template.yaml` is the contract; this `WORKFLOW`
   is the process.
2. **Re-read `SCOPE.md` §"What is already answered" and §"The
   two-crate split — pre-drafted API sketch".** Six decisions
   are locked (Q1, loop scope, provider, cleanup, layering, PR
   shape). The API sketch IS the implementation contract for
   Block B — match it exactly.
3. **Re-read `SCOPE.md` §"What is already landed".** PR 27 + PR
   28 are on master at `8d84235`. Don't re-do them.
4. **Re-read starter `DOCS/agent/SCOPE.md`** for R2 (AiRunner-
   only LLM seam), R-agent-1 (ai-agent is THE agent primitive),
   and R-agent-2 (sessions through the engine's SessionStore —
   we're deferring this; LONG-TERM.md is the contract).
5. **Run `cargo check --workspace` from the workspace root before
   any edit** so the baseline is known-clean.
6. **For block A**: `cat rubix/crates/rubix-flows/flows/scheduled-system-check.yaml`
   is the schema you parse. Read it carefully — note the
   `trigger`, `nodes[]` with id/kind/config, optional `links[]`.
   The loader must accept all six bundled YAMLs without
   per-file branches.
7. **For block B**: read the `AiRunner` trait at
   `crates/starter-spi/src/ai/runner.rs:51` end-to-end before
   writing `AgentLoop::run`. The `run` signature is
   `async fn run(&self, input: RunnerInput, session_id: SessionId,
   on_event: OnEvent, cancel: &dyn Cancel) -> Result<RunResult,
   RunnerError>`. The loop builds `RunnerInput`, drains
   `on_event` (or discards it for v0), and inspects `RunResult`.
8. **For block C**: confirm `starter_ai::runners::claude::ClaudeRunner`
   compiles and `ready()` returns true on the dev host (the
   operator has the `claude` CLI installed). If `ready()` is
   false, your test fixture is the only path that works — flag
   it in the PR description.

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
  `// FIXED:`, `// Previously this used X`.
- **TODOs carry an owner or upstream tag.**
- **`Done`-doc handover paths must be listed individually** —
  no shell brace expansion (`{a,b}.sql`), no globs (`*.rs`),
  no leading `./`. The runtime's diff-verify pre-check is
  strict; see the workaround note in SCOPE.md §"Hard rules"
  and the upstream bug at
  `/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`.
- **No live LLM in CI.** Tests use `MockAiRunner` (Block B
  provides it) or recorded fixtures from
  `starter-server::testing` (Block C uses them). A test that
  spawns the real `claude` CLI is a CI flake waiting to happen.

## When stuck

Codeless cannot ask the human. The escape hatch:

1. Stop work on the current block immediately.
2. Open the PR anyway with whatever compiles.
3. Add `BLOCKED: <one-line question>` to the PR description plus
   a paragraph explaining what was tried.
4. **Do NOT move to the next block — A → B → C is a hard
   dependency chain.** A blocked stage blocks the whole job.

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
The job is NOT finished until the human runs a real end-to-end
smoke:

```bash
# Bring the dev stack up (if not already running)
mani run demo

# In a second terminal, hit the MCP surface with a real query
RUBIX_PRINCIPAL_EMAIL=op@example.com \
RUBIX_CONFIG="$PWD/rubix/dev/agent.toml" \
cargo run -p rubix-agent --bin rubix-admin -- mcp <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"com.rubix.scheduled-system-check","arguments":{"prompt":"how full is the disk?"},"_meta":{"acceptLanguage":"es-AR"}}}
EOF
```

The second call should produce Spanish prose carrying a real
percentage from the local disk — not a hardcoded number from a
hand-rolled flow. If the LLM doesn't pick the tool or the
response is the literal fake string from before this job, the
runtime didn't actually land.

## Anti-patterns specific to this job

- **Leaving the `com.rubix.diag-render` `NodeBehavior` in
  `boot/mcp.rs`.** That's the fake. Block A deletes it; if it
  survives, the imposter is still active.
- **Implementing `AgentLoop` inside rubix-tools first "to
  prototype."** The architectural commitment is two upstream
  crates. Building it in rubix first creates a fork that has
  to be re-deleted in Block B. Skip the prototype; build it
  upstream from the start.
- **Letting `starter-ai-agent` depend on `starter-flow-spi`.**
  The whole point of the two-crate split is that the primitive
  is engine-agnostic. The REVIEW gate for Block B checks this
  with `cargo tree`.
- **Recording an LLM fixture by running the live CLI and
  capturing whatever comes out.** Fixtures must be deterministic
  — record once against a stable prompt, check in the response,
  and assert structural properties (not literal strings the LLM
  might phrase differently next time).
- **Adding `cost_cap` / `SessionStore` / `Cancel` observation
  to `starter-ai-agent` "while you're in there."** Those are
  LONG-TERM. The thin v0 contract is locked. Anything beyond
  it bloats this job and forces the next extension job to
  un-bake decisions.
- **Translating skill bodies or `ToolDescriptor` fields.** EN
  canonical, always. The agent loop dispatches by tool id; the
  reply is rendered at the transport edge.
- **Reading `SCOPE.md` / `HOW-TO-CODE.md` / `NEW-SESSION.md`
  from source code comments.** The lint catches the obvious
  forms; review catches the rest.

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
   the job's branch (`codeless/rubix-agent-runtime`) so the work
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
match the path literally.

## References

- Source SCOPE:
  [`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
- Starter agent SCOPE (authoritative for R2, R-agent-1, R-agent-2):
  [`/home/user/code/rust/starter/DOCS/agent/SCOPE.md`](/home/user/code/rust/starter/DOCS/agent/SCOPE.md)
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
