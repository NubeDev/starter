# Workflow — cache-v1-v2-v3

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md`, the proposal at
[/home/user/code/rust/starter/rubix/docs/proposal/fe-cache-opt-in.md](/home/user/code/rust/starter/rubix/docs/proposal/fe-cache-opt-in.md),
the v0 progress at
[/home/user/code/rust/starter/rubix/docs/sessions/cache-v0-progress.md](/home/user/code/rust/starter/rubix/docs/sessions/cache-v0-progress.md),
and the v0 runbook at
[/home/user/code/rust/starter/rubix/docs/operations/cache-runbook.md](/home/user/code/rust/starter/rubix/docs/operations/cache-runbook.md).

## Sequencing

Four stages with three REVIEW gates between them:

```
stage 1 (v1) → REVIEW → stage 2 (v2) → REVIEW → stage 3 (v3) → REVIEW → stage 4 (docs)
```

- Stage 2 cannot start until v1's `CacheSpec` grew the SWR fields
  and the bucket-tag registry — stage 2's `time_series:` block
  layers on the bucket tag machinery v1 introduces.
- Stage 3 cannot start until v2's `WindowedFetcher` shape is
  reusable — v3's SDUI integration and tower layer build on top of
  it via the spec the dispatcher already consumes.
- Stage 4 (docs) cannot start until v3 is end-to-end green —
  flipping the proposal status while code is still in flight
  produces a doc that lies.

REVIEW gates are mandatory because the user explicitly asked for
"any outstanding questions" to be raised. Each REVIEW gate writes
its answers to `SCOPE.md` §"Open questions" so they survive on
disk, not in the agent's head.

## Per-stage discipline

Before writing any code or docs in a stage:

1. Re-read the corresponding phase of the proposal:
   - **Stage 1 (v1)**: §Layer 3 (invalidation), §Layer 6b
     (SWR semantics spelled out), §"Failure modes" → empty-result
     case, §"Pieces the original revision missed" → read-only
     handler declaration + empty-TTL edge case.
   - **Stage 2 (v2)**: §Layer 4b (time-windowed reads),
     §Layer 6c (tenant-shared coalescing / two-layer cache),
     §"Companion crate — starter-windowed" (the whole section).
   - **Stage 3 (v3)**: §Layer 2 (SDUI integration + core HTTP
     tower layer), §Layer 3(b) (event-bus invalidator),
     §Layer 7 (backends → Valkey), §"Pieces the original revision
     missed" → cold start, §Layer 3 (dimension-scoped tags +
     WarehouseWriter chokepoint discussion).
   - **Stage 4 (docs)**: the whole proposal, with focus on
     §"Why this is deferred" / §"What would un-defer this" — the
     status flip writes the retrospective.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". The biggest
   risk on this job is feature creep past v3 into D-NP territory
   (cross-process read-your-write semantics, offline-first SDUI,
   `foyer` backend, second WindowedFetcher engine). Stay strictly
   within the three phases.
3. Re-read `SCOPE.md` §"Open questions" before the stage starts.
   Any question relevant to this stage gets a written answer in
   the handover by stage end, not silently coded around.
4. Run the v0 regression fence: `cargo test -p starter-cache` from
   the starter repo root, plus `cargo test -p starter-ext-server`
   (from `starter-extensions/`), plus
   `cargo test -p rubix-agent --test admin_cache_test`. All v0
   tests stay green at every stage boundary.

Before committing a stage:

1. `cargo build --workspace` green from the starter repo root.
2. `cargo clippy --workspace --all-features -- -D warnings` green.
3. `cargo fmt --check` green.
4. `mani run build --all` green; `mani run lint` green.
5. `mani run test --all` green — including the v0 regression
   fence — plus any testcontainer-backed `#[ignore]` tests this
   stage introduces (run explicitly with
   `cargo test -- --ignored` for the touched crates and record
   the transcript in the handover).
6. Stage-specific extras:
   - **Stage 1**: existing v0 sidecars at
     `rubix/extensions/com.nubeio.rubixos/kinds/com.nubeio.rubixos.warehouse_query.cache.yaml`
     still parse cleanly (no v1 fields required of them); the
     canary's added `stale_while_revalidate: 30s` line shows up
     in the canary smoke test.
   - **Stage 2**: `starter-windowed` builds and tests green as a
     standalone crate with zero dep on `starter-cache` —
     `cargo build -p starter-windowed --no-default-features`
     and `cargo test -p starter-windowed`. Verify by examining
     `Cargo.toml` directly that `starter-cache` does not appear.
   - **Stage 3**: `RUBIX_CACHE_INVALIDATOR=event-bus` boot path
     exercised in a smoke test with two `CacheLayer` instances
     sharing one bus; Valkey backend feature-gated; warmer
     smoke covers a synthetic process-restart;
     `// TODO(cache-invalidation):` markers gone from the five v0
     sites — repo grep for that string returns no live-code
     matches.
   - **Stage 4**: `cargo doc --workspace --no-deps` green;
     `cargo test --doc` green; the runbook's "Anatomy of the
     response" section diffs cleanly against an actual
     `GET /api/v1/admin/cache/specs` payload captured from a
     local run.

Commit + push via **mani** from the codeless-workspace root:

```
./bin/mani --config mani.yaml run commit --projects starter \
  MSG='stage N: <one-line title from template.yaml>'
./bin/mani --config mani.yaml run push --projects starter
```

No `--force`, no `--no-verify`.

## REVIEW gate behaviour

REVIEW gates pause the runner until the user approves. At each
gate:

1. **Update the handover with what landed in the preceding stage**:
   files touched, tests added (with names), new public API
   surface, any breaking changes (there should be none if R10 was
   respected). Link to the relevant `git log` lines.
2. **Answer the open questions from `SCOPE.md`** that are now
   resolvable, **in writing**. Update the `SCOPE.md` §"Open
   questions" section with the answer inline, dated, in the same
   shape as the v0 progress doc's resolved-questions blocks.
3. **Surface new open questions** that the stage uncovered — the
   user explicitly asked for outstanding questions to come up at
   REVIEW gates. If a stage uncovers a design decision that the
   proposal does not pin down (e.g. "Q9: how do we expose the
   warmer's last-run timestamp via the admin endpoint?"), add it
   to `SCOPE.md` §"Open questions" with a proposed default and
   wait for the user.
4. **Commit + push the preceding stage** (the trio runs at the end
   of the preceding stage; the REVIEW gate only pauses the *next*
   stage). The gate's own work is the handover update — that
   commits as a `docs(cache): REVIEW gate after stage N` commit
   on its own.
5. Do **not** start the next stage until the user explicitly
   approves. If the user comes back with "keep going" without
   addressing an open question, treat that as "answer it
   yourself with your best judgment and document the choice in
   the decisions log" — never silently bypass an unresolved Q.

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
   session doc
   (`rubix/docs/sessions/cache-v1-v2-v3-progress.md`, created in
   stage 4 but updated cumulatively starting stage 1 so the
   history is built up incrementally), in the same worktree, so
   the fresh agent that opens the next stage has the context it
   needs.
3. `git` — stage the changes (`git add -A` from the worktree
   root, or specific paths if the stage was surgical), commit with
   the message `stage N: <one-line title from template.yaml>` so
   the history mirrors the template stages one-for-one, and push
   to the job's branch (`codeless/cache-v1-v2-v3`) so the work is
   recoverable even if the worktree is wiped.

A stage is not "done" until all three todos are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry — do
not mark the stage `[x]`, do not advance, and never `--force` or
`--no-verify`.

## Anti-patterns specific to this job

- **Do not** start a stage by reading the v0 sources to "remember
  how it works". The v0 progress doc + the runbook are the
  authoritative source for v0 shape; if you need to know what v0
  did, those docs were written for exactly that purpose. Re-read
  them, then check current code only when verifying you understood
  correctly.
- **Do not** break v0 wire-shape. Every new `CacheSpec` field is
  optional with a default that matches v0 semantics. A v0 sidecar
  must parse cleanly under the v1 parser; a v1 sidecar must parse
  cleanly under the v2 parser; etc. Verified by keeping the v0
  canary tests green at every stage boundary.
- **Do not** widen scope past v3. The proposal explicitly stops at
  v3 (multi-node fan-out, Valkey, SDUI, tower, warmer, dimension
  tags, chokepoint). Do not implement `foyer`, do not implement
  a second `WindowedFetcher` engine beyond Timescale/PG, do not
  implement cross-replica read-your-write semantics. Those are
  post-v3 work.
- **Do not** add a `time_series:` or `inner_scope:` field to
  `CacheSpec` in stage 1. Stage 1 is v1 only — SWR + empty + bucket
  tags + read-only handlers. v2's spec extensions land in stage 2.
- **Do not** add an SDUI page-level `cache:` block in stage 2.
  Stage 2 is `starter-windowed` + the spec extensions that go with
  it. SDUI integration is stage 3.
- **Do not** ship the `WarehouseWriter` chokepoint in stage 1 or
  stage 2. The chokepoint is a stage-3 v3 deliverable — until then
  the v0 `// TODO(cache-invalidation):` markers stay where they
  are. Touching them in an earlier stage drags v3 work forward and
  breaks the phase boundaries.
- **Do not** add deprecated keys / forwarding shims for v0 sidecar
  fields. Every new key is additive; nothing v0 needs deprecating.
  A "for safety" alias is wrong shape.
- **Do not** introduce a parallel `Cache` trait or a second
  `CacheLayer` shape. v1/v2/v3 grow the existing types
  additively — the same trait, the same layer, more fields. The
  proposal's "one Cache trait, multiple backends" line is
  load-bearing.
- **Do not** pull `serde_yaml` into the workspace dep surface.
  The v0 decision was explicit: hand-rolled parser, line-number
  errors. New keys grow that parser.
- **Do not** add `unsafe` to `starter-cache`. The v0 crate is
  `#![forbid(unsafe_code)]`. If a v1+ feature needs unsafe, the
  feature is wrong-shaped.
- **Do not** silently change the dispatcher's streaming-bypass
  behaviour. The
  `streaming_dispatch_bypasses_cache_even_with_sidecar` test is
  the regression fence; if it goes red at any stage, the work in
  that stage broke a v0 invariant and must be reverted before
  proceeding.
- **Do not** treat the read-only handler declaration as v0
  retrofit. It lands in stage 1 as part of v1 — handlers that
  predate v1 land without the declaration and the dispatcher
  treats absence as "unknown, no auto-invalidate". Stage 1
  updates the existing rubixos handlers as needed.
- **Do not** assume the `RubixEventBus` satisfies at-least-once
  delivery. The v3 event-bus invalidator must verify (Q6 in
  `SCOPE.md` §"Open questions") and either confirm or layer a
  persistence shim.
- **Do not** flip the proposal status in any stage other than
  stage 4. The status is the load-bearing signal that this job is
  done; flipping it mid-job creates a "Landed but actually still
  in flight" lie that future readers will trip over.
- **Do not** `--force`, do not `--no-verify`. If a hook fails, fix
  the cause.

## When to halt

- A v1 SWR scenario from the proposal §Layer 6b cannot be
  implemented under the existing `CacheLayer` shape without a
  significant refactor (e.g. the layer's `get_or_load_labelled`
  signature doesn't accommodate stale-while-fresh). Halt at
  stage 1; the resolution is to widen the layer API, which is a
  design decision worth surfacing at the v1 REVIEW gate rather
  than improvising.
- The bucket-tag registry's per-row fan-out (Q2 in
  `SCOPE.md` §"Open questions") proves prohibitively expensive
  during stage 1 perf checks — surface at the REVIEW gate; the
  resolution might be to defer per-row bucket tags to v2 and ship
  only table tags in v1, which is a phase-boundary shift worth
  the user's call.
- A `MartSpec`-style audit reveals an existing sidecar that
  cannot be translated to v1 (e.g. the canary's existing key
  shape conflicts with a v1 field's semantics). Halt; the
  resolution is to reshape the canary, which is a design
  question.
- `starter-windowed` cannot be made engine-agnostic — the
  `WindowedFetcher` trait shape leaks a Timescale-specific
  concept. Halt at stage 2; the trait shape is the load-bearing
  decoupling.
- No non-cache consumer of `starter-windowed` exists or can be
  identified for stage 2 (Q3 in `SCOPE.md` §"Open questions").
  Halt at the v2 REVIEW gate; landing `starter-windowed` without
  a non-cache consumer is the "premature platform" failure mode
  the proposal explicitly warns about.
- The `RubixEventBus` lacks at-least-once semantics and adding a
  persistence shim is a larger project than a stage can absorb
  (Q6). Halt at stage 3's event-bus integration; surface at the
  REVIEW gate with a proposed deferred sub-issue and a decision
  about whether v3 ships without the event-bus invalidator (which
  would push it to v3.1).
- The `WarehouseWriter` chokepoint design (Q7) forces a wider
  refactor of the warehouse-write surface than a single stage
  can absorb. Halt at stage 3; the resolution is to split the
  chokepoint into its own job, which is a template change worth
  surfacing.
- Valkey license / connection-pooling story (Q8) is non-trivial.
  Halt at stage 3; the resolution may be to defer the Valkey
  backend to v3.1, which keeps v3 as the SDUI + tower + chokepoint
  ship and Valkey as a follow-up.
- Stage-3 repo grep for `// TODO(cache-invalidation):` finds
  references the chokepoint missed. Do not paper over with a
  one-line fix — re-open stage 3 and route the missed sites
  through the chokepoint, then re-run the trio.
- Stage-4 status flip would lie. If anything from v1/v2/v3 is
  not actually green at stage 4 start, halt; the resolution is to
  re-open the offending stage and fix it before the proposal
  status flip.
