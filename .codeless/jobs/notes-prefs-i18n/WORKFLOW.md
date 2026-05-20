# Workflow — notes-prefs-i18n

How to drive the stages defined in [`template.yaml`](./template.yaml)
against the brief in [`SCOPE.md`](./SCOPE.md). The deep design lives
in [`examples/notes/user-pref.md`](../../../examples/notes/user-pref.md);
re-read that file at the top of every stage.

## Sequencing

- Stages 1–3 are independently reversible. If Stage 2 is held up by
  a singleton-API question, Stage 3 can still draft against the
  expected hook shape — but the SDK landing PR is gated by the
  Stage-2 singleton being merged.
- The first REVIEW gate sits between Stage 3 and Stage 4: the
  `com.nube.hello` rewrite locks in the SDK hook signatures, so
  the user must sign off on the hook surface and `MessageKey`
  codegen shape before downstream code depends on them.
- Stages 4–6 are coupled and **must ship in one branch**. The
  panel rewrite assumes the hooks; the `block.yaml` wiring assumes
  the panel; the Playwright spec assumes the wiring. Do not push
  stages 4 and 5 in different PRs.
- The second REVIEW gate sits between Stage 6 and Stage 7. Cross-cut
  design (channel name, telemetry event names, fallback chain
  truncation, perf-budget thresholds) is locked here so the
  hardening pass doesn't churn the provider internals.

## Per-stage discipline

Top of every stage:

1. Re-read [`examples/notes/user-pref.md`](../../../examples/notes/user-pref.md)
   end-to-end. The stage detail there is more precise than the
   one-line stage description in [`template.yaml`](./template.yaml).
2. Re-read the two platform rules:
   [`DOCS/user/scope/SCOPE.md`](../../../DOCS/user/scope/SCOPE.md) R9
   and [`DOCS/extensions/scope/SCOPE.md`](../../../DOCS/extensions/scope/SCOPE.md)
   R11.
3. Read the `handover.md` left by the previous stage if any.
4. Run `pnpm -w build && pnpm -w test && cargo check -p starter-notes`
   to confirm the baseline is green **before** writing code. If it
   isn't, stop — do not work on top of red.

While writing:

- Tests in the same commit as the code (workspace rule). No "I'll
  add the test next stage."
- Touch only the files the stage names. If the stage forces a touch
  outside scope, note it in `handover.md` so the next stage's smoke
  knows.
- Never `--no-verify` and never `--force`. If a pre-commit hook
  fails, fix the cause.
- Singleton ids are the package + subpath
  (`@nube/starter-ui-core/preferences`, **not** `prefs`). If you
  are typing a string literal for a singleton id, double-check
  D-NP.1.
- Telemetry event names are stable (`extension.singleton_mismatch`,
  etc.) — they are dashboard keys. Do not rename. Do not
  pluralise.

End of every stage: the **closing trio** (see block below).

## REVIEW gates — what to write into handover

A REVIEW gate still commits + pushes the stage that led to the
gate; it only pauses the next stage. At the gate, the handover
must include:

- **Decisions surfaced.** The two-or-three specific decisions the
  user needs to look at. Don't summarise everything — name what
  needs eyes.
- **Diff highlights.** Three or four file:line pointers the
  reviewer should open first.
- **Open questions resurfaced.** Any of the SCOPE.md open
  questions still unresolved at this gate.
- **What the next stage will do once approved.** One paragraph.

REVIEW handovers are a single page. They are not status reports.

## Anti-patterns for this job

- **Re-creating an `IntlProvider` inside an extension.** The whole
  point of the singleton channel is one provider per page. If you
  find yourself mounting `<IntlProvider>` inside `remoteEntry.js`,
  back out — read R9 of the prefs SCOPE again.
- **Refetching `/v1/me/preferences` from the extension.** Same
  reason. The host owns the fetch; extensions read the resolved
  value off the context.
- **Inlining `Intl.DateTimeFormat()` with a hard-coded locale.**
  Always go through `useHostFormatters()`.
- **Adding new top-level deps.** `react-intl` is transitive via
  `ui-core`. Other deps require a justification in the handover.
- **Hand-authoring the closing-trio todos in
  [`template.yaml`](./template.yaml).** The runtime injects them.
  See ADDING-JOB Step 2.
- **Splitting the panel rewrite, manifest wiring, and Playwright
  spec across PRs.** Stages 4–6 ship together.
- **Renaming telemetry events to be "nicer".** They are the
  dashboard contract. Frozen on merge per the SCOPE.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list:
   `pnpm -w build`, `pnpm -w test`, `cargo check -p starter-notes`,
   plus the stage-specific smoke test (`prefs-host.test.tsx` for
   Stage 1, etc.). For Stages 5+ also run
   `pnpm -w run check:i18n`. For Stage 6 also run Playwright.
   Every step must pass. On failure: stop, fix, re-run; do not
   advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, in the same worktree, so the fresh agent that opens
   the next stage has the context it needs.
3. `git` — stage the changes (`git add` specific paths, not blanket
   `-A` unless the stage was workspace-wide), commit with the
   message `stage N: <one-line title from template.yaml>` so the
   history mirrors the template stages one-for-one, and push to
   the job's branch (`codeless/notes-prefs-i18n`) so the work is
   recoverable even if the worktree is wiped.

A stage is not "done" until all three todos are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry — do
not mark the stage `[x]`, do not advance, and never `--force` or
`--no-verify`. If a stage genuinely produced no change, say so in
the handover and mark `git` as `skipped — no diff`, but the next
stage's commit must include any side-effect files the
investigation touched.
