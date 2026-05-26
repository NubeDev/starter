# NEW-SESSION — paste this to a fresh AI session

You are picking up the **data-flow** scenario in this repo. Your
job is to land **one stage** end-to-end, then stop. Do not try to
land the whole pipeline.

## Read these, in this order

1. [rubix/docs/sessions/data-flow/README.md](./README.md) — the scenario (energy + water, deliberately messy) and stack map.
2. [rubix/docs/sessions/data-flow/USAGE.md](./USAGE.md) — bring the stack up, log in, call a verb. Operator creds are `op@example.com` / `rubix-dev-passwd`.
3. [rubix/docs/sessions/data-flow/PROGRESS.md](./PROGRESS.md) — find the row marked ⏳. That is your stage.
4. The stage doc for that row (`01-producer.md` / `02-…` / etc.).

Then start work. Use the design docs the stage links to as ground
truth; do not invent shapes that contradict them.

## Rules for this session

- **A stage is not done until live e2e testing is done.** Unit
  tests, `cargo test`, and `HTTP 200` responses are necessary but
  not sufficient. You must restart the stack from your built
  binary, log in as the operator, drive the verbs the stage doc
  lists, and inspect ClickHouse / Postgres directly to confirm
  every bullet in the stage's "Success bar". Run the e2e drive
  **twice** with a cold restart between runs — flaky on the
  second run means not done. See [USAGE.md §6](./USAGE.md#what-finished-means-read-this-before-anything-else).
- **Do not create a new git branch.** Work on whatever branch is
  currently checked out. If you think a branch is needed, stop
  and ask the user — do not switch or create one yourself.
- **Do not start the next stage** once the current stage's
  success bar is green. Update PROGRESS.md, commit, and stop.
- **Do not edit locked sections** of stage docs (Scope, Wire
  shape, Schema, Decisions taken once ticked). If a lock looks
  wrong, write a session note via
  [_SESSION-TEMPLATE.md](./_SESSION-TEMPLATE.md) and raise it to
  the user — do not silently change it.
- **One stage per session.** Spillover goes into a date-stamped
  session note next to PROGRESS.md, named per the convention in
  USAGE.md §6.

## Heads-up: parallel AI sessions

The user **may have other AI sessions running on the same git
branch** at the same time. Consequences you need to handle:

- **Check `git status` before and after any edit.** Files you did
  not touch may appear modified — that is another session's work,
  not yours. Do not revert, stash, or "clean up" anything you did
  not create.
- **Re-read PROGRESS.md right before you commit.** Another session
  may have flipped a row while you were working. Rebase your row
  update on top of theirs rather than overwriting.
- **Stick to your stage's files.** If the stage doc says you own
  `rubix/crates/rubix-tools/src/warehouse/`, do not also edit
  `dashboard/` or `flow_ops/` — another session may be in there.
- **Pull before commit, push fast.** Treat the branch as shared.
  If a push is rejected, rebase (don't force) and re-push.
- **Bringing the stack up: always `make restart` first**, falling
  back to `make start` if `restart` fails. The e2e success bar
  needs the stack running off your freshly-built binary, and
  `restart` is the only target that guarantees that. `make clean`
  is still off-limits without asking — it wipes docker volumes
  (Postgres + ClickHouse data) other sessions may depend on.
- **Commit messages should name the stage** (e.g.
  `feat(data-flow/02): bind rubix.warehouse.ingest`) so the user
  can untangle parallel work in the log.

## When you finish

1. Stage's success bar is green via **live e2e** (not just unit
   tests) → run it twice with a cold stack restart between runs
   to be sure it's not flaky.
2. Paste the live inspection output (CH `count(*)`, PG
   `starter_changes` rows, log greps — whatever the success bar
   names) into your session note as proof.
3. Update [PROGRESS.md](./PROGRESS.md) per its "How to update
   this file" checklist. Evidence column must reference the
   **live** run, not a unit test.
4. `git status` — confirm only files you intended are modified.
5. Commit with a stage-scoped message.
6. Stop. Hand back to the user with a one-line summary of what
   landed and the SHA.

If you get stuck, walk the stage doc's "If it fails" list in
order, then write a follow-up note from the template and stop.
Don't improvise past the third check.
