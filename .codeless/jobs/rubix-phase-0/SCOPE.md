# Scope — rubix-phase-0

The authoritative design lives at
[/home/user/code/rust/starter/rubix/SCOPE.md](/home/user/code/rust/starter/rubix/SCOPE.md).
This brief is the trimmed per-job scope. Where this disagrees with
the source SCOPE, **the source SCOPE wins** — fix this file rather
than diverge.

## Goal

Land Phase 0 of `rubix` on the `starter` repo via the
`codeless/rubix-phase-0` branch. After this job:

1. The full `rubix/` tree exists with empty-but-valid crate
   skeletons (Rust + TS + Dart placeholder).
2. The Cargo workspace at the `starter` repo root includes every
   new rubix Rust crate; pnpm workspace at `rubix/` resolves every
   TS package; `mani.yaml` builds and lints both.
3. The `mani run lint` task enforces the R1 400-line-per-file
   limit so future phases cannot quietly regress.
4. `data-postgres` and `data-clickhouse` smoke tests pass against
   testcontainers, proving the `starter-store-*::testing` seams
   compose into rubix.
5. The **eight** load-bearing design docs are written and reviewed:
   `OVERVIEW.md`, `EVERYTHING-AS-NODE.md`, `NODE-AUTHORING.md`,
   `KIND-MANIFEST.md`, `AUTH.md`, `MIGRATIONS.md`, `TESTS.md`,
   `VERSIONING.md`. Other docs land just-in-time before the phase
   that needs them.
6. No domain logic, no kinds, no Studio pages — Phase 0 is
   gate-writing only.

## In scope (three stages mapping to the source SCOPE's Phase 0 bullets)

- **Stage 1 — Layout, workspace wiring, mani task surface.**
  Skeleton crates for `contracts/spi`, `agent/crates/{graph,engine,kinds-registry}`,
  `agent/crates/apps/agent`, `agent-sdk`, `agent-client-rs`,
  `agent-client-ts`, `agent-client-dart`, `ui-kit`, `ui-core`,
  `extension-ui-sdk`, `studio`, `desktop`. Workspace membership.
  `mani.yaml` with build/test/lint/status. R1 file-size lint.
- **Stage 2 — Testcontainer fixtures + CI smoke.**
  `data-postgres` and `data-clickhouse` empty crates with smoke
  tests via the existing `starter-store-*::testing` seams.
  `rubix/docs/testing/SETUP.md` documents the docker
  prerequisites.
- **Stage 3 — The eight load-bearing design docs.**
  `OVERVIEW.md`, `EVERYTHING-AS-NODE.md`, `NODE-AUTHORING.md`,
  `KIND-MANIFEST.md`, `AUTH.md`, `MIGRATIONS.md`, `TESTS.md`,
  `VERSIONING.md`.

## Out of scope (Phase 0 only — Phases 1–5 become separate jobs)

- **Phase 1** (devices + points + i18n + Studio shell) — separate
  job, blocked on `AUTH.md` + `MIGRATIONS.md` from this job.
- **Phase 2** (schedules + alarms + history + ClickHouse hookup)
  — separate job, blocked on `RUNTIME.md`.
- **Phase 3** (dashboards + first extension) — separate job,
  blocked on `SDUI.md` + `EXTENSIONS.md`.
- **Phase 4** (artifacts + warehouse marts + production hardening)
  — separate job, blocked on `ARTIFACTS.md`.
- **Phase 5** (mobile admin) — separate job, blocked on
  `agent-client-dart` v1 stable.
- **Multi-tier deployment / fleet topology.** R9 — cloud-only in
  v1.
- **Supervisor daemon (`rubixd`)** — only meaningful in a
  multi-tier world, deferred.
- **Additional block templates beyond MQTT (which itself ships in
  Phase 3, not this job).** BACnet / Modbus / etc. are downstream
  consumer concerns.
- **Any domain logic.** Devices, points, schedules, alarms,
  histories, dashboards — none of these get a single `impl` in
  Phase 0. Phase 0 is *structure + docs*, period.
- **Any kind manifests.** The skeleton crate `kinds-registry`
  exists but registers zero kinds; that's Phase 1's job.
- **Any REST/gRPC/MCP/CLI surface code.** Skeletons exist (or are
  deferred to Phase 1); no routes, no handlers, no DTOs.
- **Any frontend pages.** The Studio shell directory exists but
  contains no pages, no router, no providers.
- **Any block authoring.** `extensions/` and `extension-ui-sdk`
  skeletons exist; no actual block ships in Phase 0.
- **Other docs from the source SCOPE's `docs/design/` list**:
  `RUNTIME.md`, `ARTIFACTS.md`, `BACKUP.md`, `QUERY-LANG.md`,
  `EXTENSIONS.md`, `LOGGING.md`, `UI.md`, `MCP.md`,
  `NODE-RED-MODEL.md`, `HOW-TO-ADD-CODE.md`, `SDUI.md`. These
  land just-in-time before the phase that needs them, per the
  source SCOPE.

## Constraints

- **R1** — 400 lines per file. Every doc, every Cargo.toml, every
  lib.rs respects the limit. The `mani run lint` task enforces.
- **R4** — layer arrow `contracts → domain → transport`. Even in
  Phase 0, the empty skeletons must respect dep direction:
  `rubix-spi` depends only on `starter-spi`; `graph` /
  `engine` / `kinds-registry` depend on `rubix-spi`; no
  transport crates exist yet (Phase 1 lands them); domain crates
  don't exist yet either.
- **R5** — `rubix-spi` has zero internal deps; depends only on
  `starter-spi`. Its skeleton reflects this.
- **R7** — `rubix-extensions-sdk` and `@rubix/extension-ui-sdk`
  are the only block-facing surfaces. Skeletons created here
  declare zero exports until Phase 3 wires the real surface.
- **R8** — `extension-ui-sdk` re-exports from `ui-core`. Empty
  re-export pattern in the skeleton, no parallel
  implementations.
- **R9** — cloud-only in v1. No fleet/edge plumbing skeleton.
- **R11** — tests live with the code. The data-postgres /
  data-clickhouse smokes in stage 2 follow this from day one.
- **R12** — comments explain why, never what. No
  session-progress chatter in the skeletons.
- **R13** — drive everything through mani. The `mani.yaml`
  landed in stage 1 is the contributor's entry point;
  build/test/lint/status all go through it.
- **starter root R0** — no monolith re-imports. `rubix/` consumes
  `starter/crates/*` via path-deps; no rubix code lands in
  `starter/crates/`.
- **MSRV / lint gates**: `cargo build --workspace`,
  `cargo clippy --workspace --all-features -- -D warnings`,
  `cargo fmt --check`, `mani run lint` all green at every stage
  boundary.

## Deliverables (what "done" looks like)

1. `codeless/rubix-phase-0` branch with one commit per stage
   (three stages = three commits, plus two for REVIEW handovers),
   pushed via mani.
2. `cargo build --workspace` green at every stage boundary.
3. `cargo clippy --workspace --all-features -- -D warnings` green
   at every stage boundary.
4. `cargo fmt --check` green at every stage boundary.
5. `pnpm install` at `rubix/` resolves; pnpm workspace links work
   (a cross-package import like `extension-ui-sdk` depending on
   `ui-core` via `workspace:*` compiles even though both are
   empty).
6. `mani run build --all` green; `mani run lint` green (and
   demonstrably fails on a synthetic 401-line file added under a
   fixture path and removed within the same stage).
7. `cargo test -p rubix-data-postgres -- --ignored` green
   against a local docker Postgres.
8. `cargo test -p rubix-data-clickhouse -- --ignored` green
   against a local docker ClickHouse.
9. `rubix/docs/testing/SETUP.md` documents the docker
   prerequisites in one command's worth of copy-paste.
10. The **eight** design docs exist under `rubix/docs/design/`,
    each under 400 lines, each citing the source SCOPE rule
    numbers it expands on, and **`AUTH.md` + `MIGRATIONS.md` are
    sufficiently complete that a Phase 1 contributor can start
    against them without re-reading the source SCOPE**.

## Open questions — RESOLVED (2026-05-23, before start)

The source SCOPE is unusually well-resolved on architecture — R1
through R13 fix the load-bearing decisions. Three job-specific
resolutions follow.

### Q1 — Scope realism: is this Phase 0 or all five phases?

**Answer: Phase 0 only. The other four phases each become their
own follow-up job.**

The source SCOPE describes five phases that span "build the whole
Niagara-style product on top of starter." Phase 0 alone creates
~20 crates and writes ~8 design docs; that is the maximum work
this job can credibly land inside a $300 / 4h cap. Phases 1–5
each carry comparable or greater scope (Phase 1 alone is
"devices + points + i18n + Studio shell with web + Tauri
desktop + login flow"), and each has explicit entry gates that
the prior phase's docs must satisfy.

**Decision.**
1. **Phase 0 is the only phase this job commits to.** Stages
   1–3 of `template.yaml` map to the three Phase 0 deliverable
   clusters in the source SCOPE.
2. Cap at **30000¢ / 4h**, same as the other queued starter
   jobs. Stage 1 (skeleton crates) is mechanical; stage 2
   (testcontainer wiring) reuses existing seams; stage 3 (eight
   design docs) is the load-bearing remainder.
3. Phase 1 onward each become their own job
   (`rubix-phase-1-devices-points`, `rubix-phase-2-schedules`,
   etc.), submitted after the prior phase's REVIEW gate
   approval. Same pattern as the prior insights split.
4. The runner halts at the stage-3 REVIEW gate (or earlier if
   any stage `[!]`s). Phase 1 is not started by this job under
   any circumstances.

### Q2 — Workspace shape: one Cargo workspace or two?

**Answer: one Cargo workspace at the `starter` repo root, with
rubix crates added as members. The split is *directory*
(`starter/crates/` vs `rubix/`), not *workspace*.**

Ground truth from `starter/Cargo.toml` shows the existing
workspace has 30+ members under `starter/crates/`; the rubix
SCOPE explicitly positions rubix as "the canonical consumer" of
starter in the same workspace as a sibling tree. Splitting into
two workspaces would force every cross-workspace dep to use
`path = "../starter/..."` syntax and break `cargo check
--workspace` running both halves in one shot — neither of which
the source SCOPE asks for.

**How to apply.** Stage 1 edits `starter/Cargo.toml`'s
`[workspace] members` array to add the new rubix Rust crates
inline (alphabetised under a `# rubix tree` comment header).
The pnpm side is separate because TS workspaces are
package-manager-specific; `rubix/pnpm-workspace.yaml` scopes the
TS packages without affecting the starter-side npm/pnpm story
(if any).

### Q3 — Are the eight design docs really enough for Phase 1?

**Answer: the eight named in the source SCOPE are the minimum.
Phase 1's entry gate explicitly names `AUTH.md` and
`MIGRATIONS.md` as required; the other six set context. The
review at the end of stage 3 confirms `AUTH.md` and
`MIGRATIONS.md` are complete enough to start Phase 1 against —
the other six only need to be coherent.**

The source SCOPE Phase 0 bullet says: *"Other docs land
just-in-time before the phase that needs them"*. So:
- `OVERVIEW.md`, `EVERYTHING-AS-NODE.md`, `NODE-AUTHORING.md`,
  `KIND-MANIFEST.md`, `TESTS.md`, `VERSIONING.md` — write to
  expand on the corresponding rules in the source SCOPE,
  cite-and-elaborate is the discipline; don't try to be
  exhaustive.
- `AUTH.md` and `MIGRATIONS.md` — **must be Phase 1-ready**.
  A Phase 1 contributor reads these before writing a single
  domain crate. Stage-3 REVIEW gate confirms this property
  explicitly.
- `RUNTIME.md` is named in the source SCOPE as a Phase 2 entry
  gate, not a Phase 0 deliverable. Out of scope for this job.

If the stage-3 REVIEW gate finds `AUTH.md` or `MIGRATIONS.md`
incomplete, halt and rework. Phase 1 starting against a
half-written `AUTH.md` is the rot the source SCOPE warns about.

## References

- Source SCOPE (authoritative):
  [/home/user/code/rust/starter/rubix/SCOPE.md](/home/user/code/rust/starter/rubix/SCOPE.md)
- Parent starter SCOPE: `starter/DOCS/SCOPE.md` (R0 — rubix lives
  alongside starter as a consumer, not inside starter/crates/).
- Workspace shape ground truth: `starter/Cargo.toml`.
- Existing testing seams: `starter-store-postgres::testing` and
  `starter-store-clickhouse::testing` (used at stage 2 for the
  empty-schema smokes).
- mani docs in the codeless-workspace:
  [`../../../codeless-workspace/DOCS/MANI.md`](../../../codeless-workspace/DOCS/MANI.md).
