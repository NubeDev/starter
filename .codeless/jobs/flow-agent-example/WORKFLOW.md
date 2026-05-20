# Workflow — flow-agent-example

How to drive the seven remaining stages of `examples/flow-agent/`
with the same rigor recent starter jobs established: one outcome
per stage, decisions confirmed at the stage entry gate, a single
commit per stage, the closing trio at the tail.

## Sequencing

- **Stage 1 (flow editor)** is the foundation. Until users can lay
  out a graph and round-trip it through PUT, every later stage is
  blocked. Do not begin stage 2 with an editor that loses node
  positions on reload.
- **Stage 2 (engine wiring)** and **stage 3 (SSE overlay)** are
  sequential, not batched. Stage 2 must produce visible
  `RunEvent`s on the existing `EventHub.runs` broadcast channel
  before stage 3 subscribes from the frontend — otherwise stage 3
  has nothing to show and the verify step is meaningless.
- **Stage 4 (chat)** is independent of stages 2–3 and could run in
  parallel in principle. Keep it sequential anyway: shared file is
  `src/rest.rs`, and reviewing a single concern per stage is
  easier than reviewing two interleaved ones.
- **REVIEW gate after stage 4** — pause for the user. Stage 5 is
  the most novel piece (synthesising flows as AI tools); the user
  should confirm stages 1–4 feel right before that lands. The
  REVIEW gate **still commits + pushes** the stage 4 work; it
  only pauses stage 5.
- **Stage 5 (bridge)** depends on stages 2 and 4 both being green.
- **Stage 6 (sidebar)** has two halves — upstream primitive, then
  example consumption. Land the primitive first (with its own
  typecheck), then the example's nested tree, in the same stage
  commit.
- **Stage 7 (polish)** is last. Do not paint a half-built UI.

## Per-stage discipline

Before writing code in any stage:

1. Re-read `examples/flow-agent/SCOPE.md` for the hard rules
   (F1–F7) and the per-file size budget. They are load-bearing.
2. Confirm the stage's named files match the example SCOPE's
   surface tables. If not, fix the SCOPE before code lands — do
   not let code drift the design silently.
3. For backend stages: skim `examples/notes/src/flow_demo.rs` for
   the canonical wiring pattern. It is the closest in-tree
   reference for `starter-flow` engine assembly.
4. For frontend stages: skim the relevant package README
   (`@nube/starter-ui-flow`, `@nube/starter-ui-chat`, etc.) before
   guessing at the API shape.

After writing code:

- `cargo build -p flow-agent` must compile clean on every backend
  stage. `cargo clippy -p flow-agent -- -D warnings` on stages 2,
  4, 5, and 7.
- `pnpm --filter flow-agent-frontend typecheck` must pass on every
  frontend stage. (`pnpm install` first if a dep landed.)
- The manual smoke described in the stage's `template.yaml` entry
  must work locally — actually fire the curl + click the button.
  Type checks do not verify feature correctness.

## What stays green from Phase 1

- `cargo build -p flow-agent` (already green at commit `83e48e8`).
- `pnpm --filter flow-agent-frontend typecheck` (already green at
  commit `83e48e8`).
- The REST surface for flows + agents CRUD at `/api/*` —
  do not regress the existing endpoint shapes; only **add** to
  them. The SSE surfaces (`/api/events`, `/api/flows/{id}/events`)
  are scaffolded; stages 2, 3, and 6 populate them with real
  events but do not change their wire shape.

## REVIEW gate (after stage 4)

When the gate fires, write into `handover.md`:

- **What stages 1–4 ship:** one-line summary of each.
- **Open questions resolved at stage entry:** the three from
  `SCOPE.md` and any that emerged.
- **Smoke evidence:** the curl commands + their outputs, the
  screenshots (or terminal-only descriptions) of the editor with
  live overlay and the chat stream working end-to-end.
- **Known issues entering stage 5:** any rough edges the user
  should know about before signing off on the bridge.

The user reads the handover, OKs, and the runner advances to stage
5. If the user pushes back on anything, fix it in a follow-up stage
5b before stage 5 proper — do not paper over.

## Anti-patterns specific to this job

- **Do not patch starter packages in the example.** F2 is hard. If
  `@nube/starter-ui-kit`, `@nube/starter-ui-flow`, or
  `@nube/starter-ui-chat` is missing something you need, **fix it
  upstream** in the same stage commit, do not inline a private
  copy in `examples/flow-agent/frontend/`. Stage 6 is the canonical
  example: the Sidebar primitive lands in the kit, the example
  imports it like any downstream.
- **Do not invent a new transport.** SSE for streams, REST for
  CRUD. No WebSocket, no gRPC. If the streaming needs something
  SSE genuinely cannot do (bidi, binary), open a discussion in
  the handover — do not just add it.
- **Do not paste raw hex into TSX.** Tailwind tokens from the
  kit's `styles.css` only. The F6 checklist hard-fails if any
  page TSX contains a `#` colour literal.
- **Do not skip the optimistic-lock conflict handler in the
  editor (stage 1).** A 409 must be visible to the user and keep
  their edits — silent overwrite is data loss.
- **Do not call `useSse` inside a `<FlowCanvas>` child** — the
  subscription belongs at the page level so it doesn't churn on
  every canvas re-render. The example SCOPE F3 table names the
  endpoint; the subscription lives in `FlowEditor.tsx`.

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
   the next stage has the context it needs.
3. `git` — stage the changes (`git add -A` from the worktree root,
   or specific paths if the stage was surgical), commit with the
   message `stage N: <one-line title from template.yaml>` so the
   history mirrors the template stages one-for-one, and push to
   the job's branch (`codeless/flow-agent-example`) so the work
   is recoverable even if the worktree is wiped.

A stage is not "done" until all three todos are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry — do
not mark the stage `[x]`, do not advance, and never `--force` or
`--no-verify`. If a stage genuinely produced no change, say so in
the handover and mark `git` as `skipped — no diff`, but the next
stage's commit must include any side-effect files the
investigation touched.
