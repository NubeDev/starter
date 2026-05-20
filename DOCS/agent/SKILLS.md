# Skills — source-of-truth notes

> This file is the source-of-truth spec for the `starter-skills`
> crate. The per-job brief at
> [`.codeless/jobs/starter-skills/SCOPE.md`](../../.codeless/jobs/starter-skills/SCOPE.md)
> sits on top of this doc and on top of
> [`DOCS/agent/SCOPE.md`](./SCOPE.md) (R4 + the skill scope rules).
> Where the per-job SCOPE.md and this file disagree, this file
> wins.
>
> Stage 1 of the `starter-skills` job is prose-only: it pins the
> four open questions (S-D1, S-D2, S-D3, S-D5) under
> [`## Decisions`](#decisions) below so later stages can compile
> against a fixed surface. The full normative body
> (R-skills-1 … R-skills-8, the `hash_bundle` algorithm, the
> public API surface, the smoke matrix) is captured in the per-job
> SCOPE.md until a follow-on prose stage lifts it here verbatim;
> that lift is **not** part of stage 1 and must not change the
> decisions recorded below.

## Decisions

Each decision below is **locked for v1**. The "Revisit when"
trigger is the only condition under which the decision is
re-opened; anything else routes through a new SKILLS.md edit
*after* a revisit trigger fires. Decisions never change silently
mid-implementation — a code change that would violate one of
these is a stop-and-write-up moment, not a drive-by.

### S-D1 — Approval CLI surface

- **Decision:** Out of scope for the `starter-skills` crate. The
  crate exposes the programmatic surface only:
  `SkillRegistry::approve(skill_id, hash)` and
  `SkillRegistry::revoke(skill_id, hash)`. Any operator-facing
  CLI (`starter skills list --quarantined`,
  `starter skills approve <id> --hash <h>`,
  `starter skills revoke <id> --hash <h>`) lands as a separate
  job against `starter-cli`, wrapping the same two methods.
- **Why:** Keeps `starter-skills` dependency-free of the CLI
  surface (no `clap`, no terminal-I/O concerns) and lets the CLI
  job own UX choices (table format, JSON output, exit codes,
  confirmation prompts) without re-opening the crate's API.
- **Revisit when:** A second consumer (not `starter-cli`) needs
  to drive approval flows and ends up re-implementing the same
  ergonomics — at that point a thin `starter-skills::cli`
  helper module is justified. Until then, `approve` /
  `revoke` are the contract.
- **Flag for:** `starter-cli` follow-on job. Cross-reference
  in that job's SCOPE under "depends on starter-skills v0.1".

### S-D2 — Resource URI scheme

- **Decision:** `file://` only in v1. Any other scheme (`s3://`,
  `https://`, `ext://`, bare relative path, opaque URI) **fails
  at parse time** with a structured error
  (`SkillParseError::UnsupportedResourceScheme { skill_path,
  resource_uri, scheme }`) naming the offending `SKILL.md` and
  the rejected URI. No silent skip, no warn-and-continue — load
  fails so the bundle is never registered in either the approved
  or quarantined list.
- **Why:** Locks the on-disk contract before extensions start
  shipping bundles. Broadening later (adding `s3://`,
  `ext://resource-id`, etc.) is additive and backwards
  compatible; narrowing later would break shipped extensions.
  Failing at parse rather than at mount keeps the failure mode
  in front of the operator who installed the bundle, not in
  front of the end user running a flow.
- **Revisit when:** A concrete consumer needs remote resources
  (e.g., ai-builder wants skill resources stored in object
  storage) **and** the on-mount hash check semantics can be
  defined for the new scheme without weakening R4. A new
  scheme MUST come with: (1) a deterministic byte stream for
  `hash_bundle`, (2) a mount-time fetch path that surfaces
  network errors as typed node failures, (3) an audit trail
  that the operator can inspect.
- **API impact:** `ResourceRef.uri` is a `String` that is
  validated at parse time; the validator is a `pub const` set
  (`SUPPORTED_RESOURCE_SCHEMES: &[&str] = &["file"]`) so
  additions are a one-line PR + a parser test.

### S-D3 — `model_hint` semantics

- **Decision:** Best-effort pass-through. The `SKILL.md`
  frontmatter `model_hint` field is documentary metadata, not a
  hard pin. The `ai-agent` node forwards the hint to `AiRunner`
  unchanged; if the runner does not recognise the model
  identifier, it falls back to its configured default and emits
  a `tracing::warn!` event including the skill id, the
  unrecognised hint, and the default actually used.
  The flow run continues — `model_hint` never blocks a run.
- **Why:** Skills are authored against a model family the
  author has tested with, but the deployment-time model
  catalogue is the operator's call (cost, region, provider
  availability). Failing the run on an unknown hint would let
  a skill author break every flow that selects the skill by
  bumping the hint to a model the operator does not have
  routed. Best-effort + WARN keeps authorship informative
  without giving the skill author production-impact authority.
- **Revisit when:** Operators report that silent fall-back is
  hiding miscalibrated skills in production — symptom is a
  skill that "works on my machine" but produces noticeably
  different output once routed to the fallback model. At that
  point a `strict_model_hint: bool` frontmatter field (default
  `false`) can opt a skill author into hard-fail on unknown
  hint without breaking existing skills.
- **Logging shape (normative):** `tracing::warn!(skill_id =
  %id, requested_model = %hint, fallback_model = %actual,
  "model_hint not recognised by AiRunner; falling back")`. A
  metric counter (`skill.model_hint.fallback{skill_id,
  requested}`) tracks the rate so operators can spot drift.

### S-D5 — Line-ending normalisation in `hash_bundle`

- **Decision:** Keep agent R4 byte-for-byte as specified in
  [`DOCS/agent/SCOPE.md`](./SCOPE.md). `starter-skills`
  implements R4 without divergence:
  - CRLF (`\r\n`) → LF (`\n`) on text-classified files.
  - Lone CR (`\r` not followed by `\n`) → LF (`\n`).
  - No BOM handling. No UTF-16 special-casing.
  - Binary files (non-text-classified) hashed as-is, no
    transform.
  The classification rule and the file-type heuristic are
  defined in the per-job SCOPE under R-skills-2; this decision
  does not override them.
- **Why:** R4 is a stability contract for the content-hash
  approval store. Operators who have already approved a bundle
  hash must be able to upgrade `starter-skills` without that
  hash silently changing. Diverging now (e.g., dropping
  normalisation in favour of a "commit LF" convention) would
  re-quarantine every previously-approved bundle on the first
  upgrade and require a coordinated approval-store flush
  across SQLite + Postgres deployments.
- **Revisit when:** A concrete operator hits the failure mode
  R4 normalisation was meant to prevent (e.g., a Windows
  contributor's CRLF-tainted commit re-quarantines an
  otherwise unchanged skill on the next CI build of a Linux
  deployment) **and** the operator's preferred fix is "drop
  normalisation, enforce LF via repo hook" rather than
  "fix the CRLF leak". That conversation is an
  agent-SCOPE-level edit (R4 itself moves), not a
  `starter-skills` edit; this crate follows whatever R4 says.
- **Out of scope for this job:** Editing
  [`DOCS/agent/SCOPE.md`](./SCOPE.md) R4. The trade-off is
  flagged here so a future R4 revision has the context;
  rewriting R4 in this job changes scope.

---

### Decision-change protocol

A decision above is changed only by:

1. A new prose-only stage on the `starter-skills` job (or its
   successor) that edits this `## Decisions` section, names the
   revisit trigger that fired, and links the issue / handover
   that justified the change.
2. A code stage that follows the prose change, never precedes
   it. Code that would violate a recorded decision is a
   stop-and-write-up moment — the runner pauses and surfaces
   the conflict in the stage handover, it does not silently
   drift.

This protocol exists so that future agents reading the codebase
can trust that the trust matrix, URI scheme, model-hint
semantics, and hash algorithm have not quietly diverged from
what this file records.
