# Scope — starter-skills

> Source of truth: [`DOCS/agent/SKILLS.md`](../../../DOCS/agent/SKILLS.md)
> in the starter repo, which itself sits on top of
> [`DOCS/agent/SCOPE.md`](../../../DOCS/agent/SCOPE.md) (R4 + the
> skill scope rules) and
> [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
> (the run lifecycle and `SkillSelector` seam). This file is the
> per-job brief the runner reads before every stage; it is
> intentionally short. When this file disagrees with the
> source-of-truth SKILLS.md, that doc wins — open an issue and
> update this file.

## Goal

Ship the **`starter-skills`** crate that lives behind the existing
`SkillSelector` seam in `starter-flow-spi`. The seam is already
wired and tested end-to-end (see
[`crates/starter-flow-spi/src/skill.rs`](../../../crates/starter-flow-spi/src/skill.rs)
and
[`crates/starter-flow/tests/stage5_skill_threading.rs`](../../../crates/starter-flow/tests/stage5_skill_threading.rs));
the only implementation today is `NullSkillSelector` returning
`SkillSelection::None`. This job ships the missing implementation:
the parser, the content-hash + quarantine state machine, the
`ApprovalStore` trait + impls, three selectors (LLM, keyword,
first), the on-mount resource-hash verification in the `ai-agent`
node, the `starter-ext-flow` wiring of `contributes.skills`, and
the two reference `SKILL.md` bundles that unblock ai-builder
Phase 5.

**No new SPI crate. No new wire format. No new adapter.** One
crate, one new `ApprovalStore` table per store crate, one new
branch in `starter-ext-flow`.

## In scope

- **`starter-skills` workspace crate** with the public API from
  SKILLS.md §"Public API surface (v1)":
  - `SkillRegistry::builder() -> SkillRegistryBuilder` with
    `with_approval_store(...)`, `with_default_selector(...)`,
    `load_dir(...)` (approved-by-default, modulo frontmatter
    `trust: quarantined`), `load_dir_quarantined(...)` (always
    quarantined), `extend(...)` (always quarantined), `build()`.
  - `SkillRegistry::list()`, `list_quarantined()`, `get(...)`,
    `approve(...)`, `revoke(...)`, `reload()`.
  - `SkillRegistry` implements `SkillSelector` from
    `starter-flow-spi`.
- **`SKILL.md` parser** with `serde` `deny_unknown_fields` on the
  frontmatter (R-skills-1). Body and resources read once and held
  by `Arc`. No templating at any point.
- **`starter_skills::approval::hash_bundle(path)`** implementing
  agent R4 byte-for-byte per SKILLS.md R-skills-2: length-prefixed
  framing, exact CRLF / lone-CR → LF byte transforms on text
  files, path separators normalised to `/`, sort lexicographic,
  hex-encoded `blake3` digest.
- **`ApprovalStore` trait + `InMemoryApprovalStore`** in
  `starter-skills`. Append-mostly semantics — drift never mutates
  the store (R-skills-3 + R-skills-7).
- **SQLite + Postgres `ApprovalStore` impls** in
  `starter-store-sqlite` and `starter-store-postgres`, each behind
  a default-on `skill-approvals` feature with one new table
  (`skill_approvals`).
- **Three selectors**: `LlmSkillSelector` (default; wraps
  `AiRunner`; 2 s hard timeout, no retries, fail-to-None graceful
  degradation, metric + tracing on every outcome — R-skills-5),
  `KeywordSkillSelector` (deterministic, no LLM), and
  `FirstSkillSelector` (test fixture).
- **On-mount resource hash verification** in the `ai-agent` node
  body (`crates/starter-flow-nodes/src/ai_agent.rs`): when
  mounting resources from a frozen `SkillSelection`, read the
  bytes, `blake3` them, and fail the run with
  `SkillResourceHashMismatch` on drift. Load-bearing for the
  quarantine guarantee under concurrent `reload()`.
- **`starter-ext-flow` wires `contributes.skills`** through
  `SkillRegistry::extend(...)`. The field is specified in agent
  R-agent-4 but not yet implemented; this job wires it. Extension
  skills land quarantined regardless of frontmatter trust.
- **Two reference `SKILL.md` bundles** at the workspace root under
  `skills/`:
  - `skills/starter.ai-builder.dashboards/SKILL.md`
  - `skills/starter.ai-builder.themes/SKILL.md`
  - Content lifted verbatim from ai-builder SCOPE §"Skills for
    ai-builder".
- **All nine smoke tests** from SKILLS.md §"Smoke tests" passing
  in CI for the new crate, plus the additional ai-agent / engine
  smokes touched by the on-mount check.
- **Dep-tree CI gate** on `starter-skills`: no provider SDK
  appears in `cargo tree -p starter-skills --edges normal`. The
  `LlmSkillSelector` talks through `AiRunner` only.

## Out of scope

- **Vector / embedding-based selection.** R-skills-5 defers this.
  Embedding storage, re-embedding on bundle update, and provider
  routing for embeddings are a separate scope.
- **A live file-watcher.** The registry exposes `reload()`; firing
  it on a `notify` event is the host's job, not this crate's.
- **Skill versioning beyond content hash.** R-skills-2 — the hash
  *is* the version. No semver field, no compatibility check.
- **Cross-skill composition / `include:`.** A skill cannot import
  another skill's body. If two skills share boilerplate, the
  operator ships the boilerplate twice. Composition is a flow
  concern.
- **Skill-side tool definitions.** `allowed_tools` is a *filter*
  over the host's `ToolRegistry` (agent R3 / flow R8). A skill
  cannot define a new tool; it can only narrow.
- **A quarantine bypass mechanism.** R-skills-4 — there is no
  `--allow-quarantined` flag, no env var, no override.
- **A remote skill registry.** Skills come from disk paths and
  loaded extensions. HTTP-fetched skills are out.
- **The operator CLI surface** (`starter skills list
  --quarantined`, `starter skills approve <id> --hash <h>`).
  `starter-skills` exposes `registry.approve(...)`; the
  `starter-cli` wrapping is a separate job (S-D1).
- **Future resource URI schemes** (`s3://`, `ext://`). V1 is
  `file://` only; other schemes parse-fail at load time (S-D2).
- **Integrating the two reference skills with ai-builder.** This
  job lands the skill bundles on disk; wiring them into an
  ai-builder flow is the ai-builder job's work.
- **Editing agent SCOPE R4.** S-D5 flags a possible "drop
  line-ending normalisation" simplification; that is an
  agent-SCOPE-level edit, out of this job's scope.

## Hard rules (load-bearing)

- **R-skills-1** — `SKILL.md` is parsed once, at load time.
  Frontmatter via `serde` `deny_unknown_fields`. Body and
  resources read once and held by `Arc`. **No templating, no
  interpolation, no env expansion at any point.** A `{{x}}` in a
  skill body is literal text the model sees. This is the agent
  R4 anti-prompt-injection guarantee.
- **R-skills-2** — Bundle hash is exactly the algorithm in
  SKILLS.md: enumerate, exclude (`.DS_Store`, `Thumbs.db`,
  `*.swp`, `*.swo`, `*~`, `.git/`, `.idea/`, `__pycache__/`),
  normalise text-file line endings (CRLF → LF then lone CR → LF;
  no BOM, no UTF-16 special-casing), normalise path separators
  to `/`, sort lexicographic, length-prefixed framing
  (`u64_le(path_len) || path || u64_le(content_len) || content`),
  `blake3`, hex-encode lowercase.
- **R-skills-3** — Trust matrix is authoritative:
  - `load_dir(...)` + frontmatter `approved`/absent → **approved**
  - `load_dir(...)` + frontmatter `quarantined` → **quarantined**
  - `extend(...)` (any frontmatter) → **quarantined**
  - `ApprovalStore` row keyed on `(skill_id, hash)` flips
    quarantined to approved. Hash mismatch re-quarantines.
- **R-skills-4** — `select(...)` never returns a quarantined
  bundle. No override flag.
- **R-skills-5** — Default selector is `LlmSkillSelector`. Failure
  semantics normative: 2 s hard timeout, no retries, on
  timeout/5xx/network/parse/unknown-id return
  `SkillSelection::None` with metric + WARN log + tracing span.
  Graceful degradation, never blocks a flow run.
- **R-skills-6** — Selector inputs are `(&SlotMap, &Principal)`.
  The default selector treats the `prompt` slot as the query;
  the principal is available to custom selectors but not passed
  to the LLM by default.
- **R-skills-7** — `ApprovalStore` is a trait (not a crate); rows
  are append-mostly. `revoke` is operator-driven only. Drift on
  reload **does not mutate the store** — the prior row stays
  inert, the new bundle hash has no row, so the matrix
  re-quarantines.
- **R-skills-8** — `select()` does no I/O beyond the optional LLM
  call. Approvals are cached at registry build time; refresh via
  explicit `reload()` only.

## Constraints

- `starter-skills` depends only on `starter-flow-spi` + `serde` +
  `serde_yaml` + `thiserror` + `async-trait` + `blake3` +
  `tracing`. No `tokio` runtime dep beyond what `async-trait`
  needs. No provider SDK (the dep-tree CI gate enforces).
- `ResourceRef.uri` is `file://` only in v1. Other schemes
  parse-fail at load time (S-D2 locked).
- `EXCLUDED` is a `pub const` slice; additions go via PR.
- `SHUTDOWN_DEADLINE_DEFAULT` is not relevant here (that's the
  service registry, different scope).
- The `LlmSkillSelector` 2 s timeout is the default; override via
  builder.
- On-mount hash check in the `ai-agent` node lives in
  `crates/starter-flow-nodes/src/ai_agent.rs`. The frozen
  `ResourceRef.content_hash` is the source of truth; on
  mismatch, fail the run with `SkillResourceHashMismatch`
  surfaced as a typed node failure.

## Phasing

- **Phase 1** — crate skeleton + `SKILL.md` parser. No registry yet.
- **Phase 2** — `hash_bundle(path)` + `EXCLUDED` list + property
  tests (collision, line-ending stability, pinned-digest).
- **Phase 3** — `SkillRegistry` + `ApprovalStore` trait +
  `InMemoryApprovalStore` + two smoke tests (quarantine on
  extend; re-quarantine on hash mismatch).
- **Phase 4** — three selectors + `SkillRegistry: SkillSelector`
  + engine integration + two smoke tests (selection frozen per
  run; quarantined never reaches strategy).
- **Phase 4b** — on-mount resource hash verification in
  `ai-agent` node + two smoke tests (resource hash mismatch
  aborts the run; no I/O on `select`).
- **Phase 5** — SQLite + Postgres `ApprovalStore` impls behind a
  `skill-approvals` feature.
- **Phase 6** — `starter-ext-flow` wires `contributes.skills`
  through `extend(...)`.
- **Phase 7** — two reference `SKILL.md` bundles at workspace
  root under `skills/`.
- **Smoke gate** — all nine smoke tests + dep-tree CI gate green.

## Deliverables

- New workspace crate `starter-skills` (SemVer 0.1.0) with the
  parser, `hash_bundle`, registry, `ApprovalStore` trait, three
  selectors, and `InMemoryApprovalStore`.
- Modifications to `starter-flow-nodes/src/ai_agent.rs` adding
  on-mount resource hash verification with the
  `SkillResourceHashMismatch` typed error.
- `skill_approvals` table + `SkillApprovalStore` impls in
  `starter-store-sqlite` and `starter-store-postgres`, behind a
  default-on `skill-approvals` feature.
- One new branch in `starter-ext-flow` wiring
  `contributes.skills` through `SkillRegistry::extend(...)`.
- Two reference bundles:
  `skills/starter.ai-builder.dashboards/SKILL.md` and
  `skills/starter.ai-builder.themes/SKILL.md`.
- Nine smoke tests passing in CI plus the dep-tree CI gate on
  `starter-skills`.

## Open questions (resolve in stage 1)

The source SKILLS.md flags four open questions; the runner pins
them in stage 1 before any code lands. Bias for each below.

1. **S-D1 — Approval CLI surface.** Bias: **out of scope for this
   crate**. `starter-skills` exposes `registry.approve(...)` /
   `registry.revoke(...)`; the `starter skills list
   --quarantined` / `starter skills approve <id> --hash <h>` CLI
   is a follow-on job against `starter-cli`. Note in the SKILLS.md
   Decisions section.
2. **S-D2 — Resource URI scheme.** Bias: **`file://` only in
   v1**. Other schemes parse-fail at load time with a structured
   error. Broadening is a future-version concern. Lock in the
   API.
3. **S-D3 — `model_hint` semantics.** Bias: **best-effort
   pass-through**. The `ai-agent` node passes `model_hint` to
   `AiRunner`; if the runner does not know the model, it falls
   back to its default and logs a `WARN`. The frontmatter field
   is documentary, not strictly enforced.
4. **S-D5 — Line-ending normalisation.** Bias: **keep agent R4 as
   specified**. This crate implements R4 byte-for-byte (CRLF / CR
   → LF on text files). Dropping normalisation in favour of "commit
   LF" is a possible simplification but is an agent-SCOPE-level
   edit. Out of scope for this crate; flag in SKILLS.md Decisions.

Record decisions in **`DOCS/agent/SKILLS.md`** under a new
`## Decisions` section before stage 3 (Phase 1) begins. Do not
edit `DOCS/agent/SCOPE.md` in this stage — R4 stays as-is.

## Decisions

(populated in stage 1)

## Cross-cutting checks the runner must keep honest

- **Dep-tree CI gate** — `cargo tree -p starter-skills --edges
  normal` snapshot test fails the build if any of `async-openai`,
  `anthropic-ai-sdk`, `anthropic-sdk`, `google-genai`,
  `aws-sdk-bedrockruntime`, `mistralai`, `ollama-rs` appears.
  `LlmSkillSelector` talks through `AiRunner` only.
- **No `secrecy` import** — n/a here; `starter-skills` does not
  handle secrets.
- **Frontmatter `deny_unknown_fields`** — a CI test loads a
  fixture `SKILL.md` with an unknown field and asserts the
  parser rejects it with a structured error naming the file.
- **No templating** — a CI test loads a `SKILL.md` whose body
  contains `{{user_name}}` and asserts the model-facing body is
  byte-identical (the template syntax is literal text, never
  expanded).
- **Hash algorithm pinned** — the fixture-bundle digest test
  pins a specific hex string; any algorithm refactor that
  changes the digest fails the build.
- **`select()` does no I/O** — the smoke test with a panicking
  `ApprovalStore` proves the cache holds.
- **On-mount resource hash check** — the resource-mismatch smoke
  proves the `ai-agent` node aborts the run on drift; without it
  the `reload()` race would silently leak.
- **Trust matrix** — the "extension is quarantined regardless of
  frontmatter" smoke proves an extension cannot self-approve.
