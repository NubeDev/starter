# Workflow — starter-prefs-i18n

How to drive this job. Shape: 22 stages, 4 REVIEW gates, one
branch (`codeless/starter-prefs-i18n`). The user explicitly
chose the "one big job" posture; the WORKFLOW exists so the
size stays tractable.

The REVIEW gates are the load-bearing safety mechanism — they
exist *because* this is one big job, not in spite of it. Treat
each gate as a hard stop; the cost of a missed regret is
higher here than in a five-job sequence because there is no
sibling-merge gate to catch it.

## Sequencing

- **Stage 1 is prose-only.** Resolve F-0.1 (feature-gate
  `uom` + `icu_locale_core` on `starter-spi`; baseline
  refresh) and F-0.2 (capture-baseline.sh). Lock D-PI.1
  through D-PI.7 in [SCOPE.md](./SCOPE.md). Commit; no code
  changes outside `DOCS/` and the new capture-baseline.sh.
- **Stage 2 is the entry-gate REVIEW.** Phase 0 closure +
  Phase 1–5 decisions all locked. Do not advance until the
  user signs off — every later stage binds to these answers
  and there is no second-chance gate until stage 11.
- **Stages 3 → 7 land Phase 1** in dependency order: scaffold
  → resolver → store → routes → client+CLI. The resolver
  (stage 4) is the highest-value test target in the entire
  job; lock its behaviour against the SCOPE Smoke-tests-block
  cases verbatim.
- **Stages 8 → 10 land Phase 2** in dependency order:
  middleware → R8 wire shape → log audit. The log audit
  (stage 10) is the negative check on R6; if a stage before
  stage 10 introduces a log line with a converted value, the
  audit catches it at workspace integration.
- **Stage 11 is the Phase 2 REVIEW gate.** Middleware does
  not mutate response bodies; per-series shape is one-per-
  series; logs are SI-only. Phase 3 starts next.
- **Stages 12 → 15 land Phase 3** in dependency order:
  scaffold + locale → catalog loader + bundle → middleware +
  routes → seed catalogs. The seed catalogs (stage 15) are
  the load-bearing consumer-facing surface for Phase 4 — get
  the MessageKey naming consistent here and Phase 4 doesn't
  fight rename churn.
- **Stage 16 is the Phase 3 REVIEW gate.** Server-side
  i18n complete; X-I18n-Fallback opt-in works; immutable-cache
  URL shape works. Phase 4 (TS / React) starts next.
- **Stages 17 → 18 land Phase 4** in dependency order:
  PreferencesProvider + formatters → IntlProvider +
  SettingsPage. Stage 17 is pure React state; stage 18 wires
  the catalog fetch + the Settings UI.
- **Stage 19 is the Phase 4 REVIEW gate.** The "Australian
  operator" smoke passes end-to-end through the new UI. Phase
  5 starts next.
- **Stage 20 lands Phase 5** as a single small stage — the
  diagnostics rewriter behind a default-off feature.
- **Stage 21 runs the workspace-wide smokes** (Headless
  appliance, Add a language, Canonical-only logs re-run,
  Australian operator server-side counterpart) + every
  dep-tree gate + R1–R8 grep / behavioural checks.
- **Stage 22 is final cleanup + docs sweep.** Mark Phases 1–5
  as landed in `DOCS/user/scope/SCOPE.md`; close F-0.1 / F-0.2
  in `PHASE0-VERIFY.md`; commit `PHASES-1-5-VERIFY.md`.

## Per-stage discipline

- **Before any code change in a stage:**
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) the
    stage touches. R1 (canonical storage), R3 (resolver
    precedence), R5 (client-side default), R6 (one
    conversion layer), R8 (per-series metadata) are the
    most-violated rules under time pressure; check them
    every stage.
  - Re-read the
    [`starter-prefs-spi`](../starter-prefs-spi/SCOPE.md)
    sibling SCOPE for the Phase 0 decisions this job inherits
    (D-U0.1 closed Quantity set; D-U0.2 closed Unit set;
    D-U0.3 currency wire form; D-U0.4 Diagnostic param
    map).
- **Touch only what the stage names.** The
  one-big-job posture amplifies drive-by-refactor risk; a
  single stage that touches three unrelated crates costs
  hours at review.
- **Verify before commit:**
  - **Rust per-stage:** `cargo check -p <touched crate>`,
    then `cargo test -p <touched crate>`, then
    `cargo clippy --workspace --all-targets -- -D warnings`,
    then `cargo fmt --check`.
  - **TS per-stage (stages 17 + 18):**
    `pnpm -C packages/starter-ui-core typecheck`, then
    `pnpm -C packages/starter-ui-core test`. Stage 21
    re-runs both at workspace level.
  - **Dep-tree per Rust stage:** re-run
    `cargo tree -p starter-spi --edges normal`,
    `cargo tree -p starter-flow-spi --edges normal`,
    and (from stage 3 onward) `cargo tree -p starter-prefs
    --edges normal`, (from stage 12 onward) `cargo tree -p
    starter-i18n --edges normal`. The `starter-flow-spi`
    baseline must stay byte-for-byte unchanged from stage 1
    onward; the `starter-spi` baseline matches the
    feature-gated version stage 1 commits.
- **One logical batch per commit.** The closing trio is the
  heartbeat the UI watches and the recovery story for a
  crashed worktree.
- **No `--force`, no `--no-verify`.** Workspace policy. A hook
  failure means fix the cause, not skip it.

## REVIEW gates

Four:

- **Stage 2 — Phase 0 closure + Phase 1–5 decisions.**
  Catches the highest-impact mistakes early. F-0.1 resolution
  posture, sqlx-pool vs Repository<T>, sqlite-only,
  iso_currency landing crate, per-series shape, fingerprint
  algorithm, react-query+context vs zustand, rewriter scope.
  Seven decisions; locking them down at the entry gate is
  the entire reason this job is tractable.
- **Stage 11 — Phase 2 complete.** Catches R6 violations
  (response-body mutation in the middleware) and R8
  violations (per-value metadata) before Phase 3 starts.
- **Stage 16 — Phase 3 complete.** Catches catalog-format
  mistakes, fingerprint-URL shape regret, and X-I18n-Fallback
  semantics drift before Phase 4 binds to the wire surface.
- **Stage 19 — Phase 4 complete.** Catches UI/UX regret
  before Phase 5 lands a separate response-side rewriter.
  The "Australian operator" smoke is the user-facing
  exit criterion — if the SettingsPage doesn't drive the
  expected conversion end-to-end, the gate does not pass.

Stages 21 + 22 are verification + cleanup — the workspace
smokes are the merge gate, not a fifth REVIEW.

Write a one-line summary into `handover.md` at every gate.
Do not proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | F-0.1 resolved (uom + icu_locale_core feature-gated; starter-spi baseline updated; flow-spi baseline diff-empty); F-0.2 capture-baseline.sh in place; D-PI.1–D-PI.7 + 5 sub-decisions filled in SCOPE.md "Decisions"; no code changes outside DOCS/. |
| 3 | `crates/starter-prefs/` exists with Cargo.toml + lib.rs + empty pub mods; workspace members updated; `cargo check -p starter-prefs` green. |
| 4 | `crates/starter-prefs/src/resolver.rs` populated; the three SCOPE Smoke-tests-block resolver cases pass as unit tests; no I/O in resolver.rs. |
| 5 | `crates/starter-prefs/src/store.rs` populated; sqlite impl behind `sqlite` feature; migrations/0001_starter_prefs.sql present with INTEGER timestamps; integration tests pass under sqlx::SqlitePool::connect("sqlite::memory:"). |
| 6 | Four REST endpoints in `crates/starter-prefs/src/routes.rs` behind `routes` feature; utoipa::ToSchema on every type; admin-only paths gated; integration tests cover the happy path + 403 + ETag behaviour. |
| 7 | starter-client-rs gains the four client methods; starter-cli `prefs` subcommand present with `get`/`set`/`units`; integration test against an in-memory server passes. |
| 8 | starter-server gains accept_units_layer; UnitsCtx in request extensions; Vary: Accept-Units set; integration test asserts canonical / preferred paths and that the middleware does NOT touch response bodies. |
| 9 | SeriesEnvelope<T> in starter-prefs with R8 shape; ToSchema gives the right openapi.json fragment; round-trip serde test passes. |
| 10 | `crates/starter-server/tests/canonical_logs.rs` audit passes; CONTRIBUTING.md (or workspace equivalent) carries the "logs are canonical" rule. |
| 12 | `crates/starter-i18n/` exists with locale.rs + scaffold; parse_accept_language tests cover the SCOPE R5 examples; workspace members updated. |
| 13 | catalog.rs + bundle.rs populated; deny_unknown_fields enforced; render_or_key fallback returns the key on miss; tracing::debug! event fires on fallback. |
| 14 | accept_language_layer in middleware.rs; manifest + catalog routes in routes.rs; immutable-cache URL form works; ETag revalidation works; integration tests assert all four behaviours. |
| 15 | en.json + es.json compiled in via include_str!; tests/seed_catalog_consistency.rs asserts identical key sets between the two; every starter-owned UI string the workspace currently emits has a key in en.json. |
| 17 | packages/starter-ui-core/src/preferences/ contains PreferencesProvider + usePreferences + formatters.ts; vitest tests cover every formatter + a (locale, prefs) snapshot matrix; package.json exports updated. |
| 18 | packages/starter-ui-core/src/i18n/ contains IntlProvider + useTranslate; SettingsPage component present and bound to PreferencesPatch; vitest tests cover the form submit + language switch; pnpm typecheck + test green. |
| 20 | starter-i18n `diagnostics` feature flag behind a default-off cargo feature; DiagnosticBody marker + tower Layer present; integration tests cover opted-in / opted-out / SSE-passthrough / missing-translation paths. |
| 21 | Headless-appliance + add-a-language + canonical-only-logs + Australian-operator smokes all green; every dep-tree gate green; workspace cargo + clippy + fmt + pnpm typecheck + test green; R1–R8 grep checks land their one-line summary into PHASES-1-5-VERIFY.md. |
| 22 | DOCS/user/scope/SCOPE.md footer notes "Phases 1–5 landed in PR #<num>"; DOCS/user/scope/PHASE0-VERIFY.md marks F-0.1 + F-0.2 closed; DOCS/user/scope/PHASES-1-5-VERIFY.md present with each smoke test recorded. |

## Anti-patterns

- **Skipping the stage 2 REVIEW.** This is the single most
  important gate in the job. Seven decisions cascade into
  ~19 stages of follow-on work; getting them wrong here
  produces hours of rework.
- **Letting any phase's middleware mutate response bodies.**
  R6 says exactly one conversion layer per surface. The
  middleware exposes UnitsCtx / LocaleCtx; the handler
  decides what to serialise. A stage that finds itself
  walking response JSON to convert values has misread R6.
- **Adding `iso_currency` to `starter-spi`.** D-PI.3 keeps
  it in `starter-prefs` only. A stage that thinks the SPI
  needs ISO 4217 validation has misread D-PI.3.
- **Adding `iso_currency` to `starter-i18n`.** Same — it
  belongs in `starter-prefs`. The i18n crate handles
  catalogs and locales; currency is a preference-domain
  concept.
- **Per-value unit metadata.** R8 says one-per-series.
  Stage 9's SeriesEnvelope<T> is the shape; a stage that
  embeds {value: 72, unit: "fahrenheit"} per point has
  slipped R8.
- **Logging a converted value.** R1 + R6: logs are canonical.
  Stage 10's audit is the negative check; do not introduce
  log statements that include "°F" / " psi" / " mph" / " lb"
  anywhere.
- **`#[non_exhaustive]` on the Phase 0 enums.** D-U0.1 /
  D-U0.2 from the prior sibling job locked the closed-enum
  guarantee. This job does not add variants; it does not
  weaken the guarantee either.
- **`todo!()` / `unimplemented!()` bodies.** Workspace
  CLAUDE.md no-half-finished-implementation rule. If a stage
  cannot complete, mark it `[!]` and halt.
- **Rewriting SSE / chunked responses in Phase 5.** R5 says
  scope-limited. The Phase 5 rewriter bails on
  text/event-stream and chunked transfer-encoding; a stage
  that tries to rewrite a streaming response has misread R5.
- **Touching `starter-flow-spi` for anything other than
  feature-gating in stage 1.** F-0.1 closure is the one
  legitimate change; any other touch is out of scope.
- **Adding a new Quantity / Unit variant.** D-U0.1 / D-U0.2
  closed the v1 enum membership. A stage that needs a new
  variant has misread the Phase 0 SCOPE; the right answer is
  a separate PR on `starter-spi`, not this job.
- **Renaming MessageKey strings between stage 15 and stage
  18.** The seed catalogs at stage 15 are the contract Phase
  4 binds to. A rename mid-Phase-4 cascades into rerun pnpm
  test and possibly resnapshot. Lock the names at stage 15
  and stick.
- **Two PrefsResolver impls.** D-PI.1 says sqlx::Pool-backed,
  one impl in this job. A stage that adds a second store
  impl (Postgres, memory-for-tests beyond the test fixture)
  has exceeded scope.
- **Bringing in a new workspace dep beyond the planned set.**
  The Phase 1 / 2 / 3 stages name their deps explicitly
  (sqlx + serde + utoipa + tracing + iso_currency for prefs;
  sha2 + icu_locale_core for i18n). Adding anything else is
  a stage-fail; surface it as an issue + a separate PR.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items,
in order. The user watches these tick over in the `Stages`
overview; they are how the user confirms a long-running stage
actually landed instead of just looking like it did. Do **not**
rename or reorder them.

1. `checks` — run the stage's verify list (per the "Done when"
   table above). Every step must pass. On failure: stop, fix,
   re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the
   active session doc, in the same worktree, so the fresh
   agent that opens the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push
   to the job's branch (`codeless/starter-prefs-i18n`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook
fails, fix the cause.
