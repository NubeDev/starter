# Scope gaps — what `rubix/SCOPE.md` does not yet account for

This doc is a forward-looking audit. It lists capabilities that
**exist in `starter/crates/`** (or are documented as planned) that
rubix's current SCOPE either omits entirely or under-specifies.
Every item below should either land in a phase, be deferred with
rationale, or be promoted into the Non-goals list.

> **Doc tier reminder.** This file lives in `docs/scope/`. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) and
> [NEW-SESSION.md §2](../../NEW-SESSION.md), **source-code comments
> must never reference this file.** When a gap promotes into the
> system, its home is a folder under `docs/design/<area>/README.md`
> — that is the only tier code may link.

Audit method: walked every `starter-*` crate in the workspace, read
its Cargo manifest description, and mapped against rubix's six
goals plus the SCOPE rules.

## How this doc is used

1. **Before each phase starts**, scan the matching rows below — if
   a gap blocks the phase, move it into SCOPE proper *and* draft
   the new `docs/design/<area>/README.md` in the same PR.
2. **At each phase exit**, append "addressed in phase N — see
   `docs/design/<area>/`" or "deferred — see <reason>" beside each
   item touched.
3. When a row promotes, it leaves this doc and lives in:
   - **A SCOPE rule update** (rubix-wide invariant) and/or
   - **A new phase deliverable in SCOPE** and/or
   - **A new `docs/design/<area>/README.md`** (canonical
     present-tense description) and/or
   - **A new `docs/adr/NNNN-<title>.md`** if a non-obvious decision
     was made along the way.

   Never a "we'll do it later" without a tracked home.

## Severity legend

| Mark | Meaning |
|---|---|
| 🟥 | Critical — affects every goal; cannot ship a usable backend without it |
| 🟧 | Major — affects multiple goals or one production-critical concern |
| 🟨 | Important — needed for full SCOPE compliance but not blocking Phase 0–1 |
| 🟩 | Worth confirming as Non-goal-for-now |

## Phase-0 verification

The Phase 0 skeleton is on disk; six crates build; `/healthz`
serves. The linter pass that landed alongside this audit normalised
every code comment to point at `docs/design/<area>/README.md` paths
(per the new doc-tier rule). Items below are **forward-looking**;
the existing code already complies with the doc-tier rule. No
back-reference cleanup outstanding.

---

## 1. i18n + user preferences (🟥 critical, multi-phase)

**What's missing.** The current rubix SCOPE mentions `starter-i18n`
and `starter-prefs` in the consumed-crates table and the contracts
rule promises `Quantity`-typed slot values + `ResolvedPreferences`.
But:

- Phase 1 ships system-check **without** an end-to-end demonstration
  that `Quantity` values + EN/ES locales actually round-trip
  through a tool reply.
- No skill mentions formatting through `MessageKey`. The bundled
  `system-checker/SKILL.md` says "render through their preferred
  units" but does not say *how* — the agent will hallucinate raw
  floats if not steered.
- No bundled Spanish translations exist anywhere in the rubix tree.
- No mention of timezone handling. Per starter's user SCOPE:
  locale, language, and timezone are **three separate things**.
  Rubix silently conflates.
- No mention of date / time / number / currency / week-start
  format prefs. starter-prefs surfaces all of them; rubix
  references only "units".

**Why it's critical.** Per starter user SCOPE: "Shipping a
US-centric API and retrofitting i18n later is a known multi-year
trap." Rubix's six goals all emit user-facing text. Every one of
them must go through MessageKey + Quantity from day one.

**Proposed promotion.**

- **Phase 1 entry gate addition:** the system-check goal must
  round-trip at least one `Quantity` value (disk usage GB/GiB by
  caller pref) and at least one MessageKey
  (`rubix.system.disk_ok` / `rubix.system.disk_warn`).
- **Phase 1 deliverable:** a tiny `rubix-spi::i18n` module (no new
  crate — keep the six-crate count) with EN + ES catalogues for
  the first batch of MessageKeys, embedded via `include_dir!` the
  same way as `rubix-skills` and `rubix-flows`.
- **New design doc:** `docs/design/i18n-prefs/README.md` — how
  rubix consumes `starter-i18n` and `starter-prefs`, the
  three-axis model (locale / language / timezone), and how tools
  format outputs.
- **SKILL.md template addition:** every rubix-bundled skill grows
  a "Localisation" section telling the agent to emit MessageKey
  + Quantity outputs, never raw strings or floats.
- **ADR candidate:** if rubix picks a non-obvious default
  (e.g. timezone defaults to UTC for headless, request header
  otherwise), record it.

**Owner:** Phase 1 entry gate.

---

## 2. Undo / redo (🟧 major, Phase 3+)

**What's missing.** [starter-undo](../../../crates/starter-undo)
ships a per-actor undo/redo cursor over the changelog with a
`Reversible` dispatch registry. Rubix's SCOPE does not mention it.

**Why it matters.** Three of rubix's six goals are *write* surfaces:

- **Goal 1 (dashboards):** an operator builds a dashboard, dislikes
  a widget change, wants to undo. Without `starter-undo`, the
  agent has no way to make changes reversible at request.
- **Goal 2 (user admin):** disabling the wrong user is exactly the
  kind of incident undo prevents.
- **Goal 3 (flow programmer):** deploying a flow + rolling back is
  the natural shape of "undo".

**Why it's not critical.** Rubix can ship Phase 1 (system check —
read-only) without it. Goals 4 and 6 also do not need undo in the
operator-visible sense.

**Proposed promotion.**

- **Phase 3 entry gate:** every write tool registers a
  `Reversible`. A bundled `rubix.undo.last` tool delegates to
  `starter-undo`.
- **New design doc:** `docs/design/undo/README.md` — write-tool
  reversibility contract.

**Owner:** Phase 3 entry gate.

---

## 3. Clipboard / copy-paste / duplicate (🟧 major, Phase 3+)

**What's missing.** [starter-clipboard](../../../crates/starter-clipboard)
ships a server-side, principal-scoped clipboard backing copy-paste
and duplicate, integrated with `Reversible`. Rubix omits it.

**Why it matters.** Dashboard authoring (Goal 1) and flow
authoring (Goal 3) both want "duplicate this widget / this node
to clipboard, paste into another page / flow". Without
`starter-clipboard`, every author re-implements naive duplication.

**Proposed promotion.**

- **Phase 3 entry gate:** `dashboard.duplicate` and
  `flow.duplicate` delegate to `starter-clipboard`. Bundle a
  `rubix.clipboard.paste` tool.
- **Design doc:** can live in the same `docs/design/undo/README.md`
  (same conceptual area) or a sibling `docs/design/clipboard/`.

**Owner:** Phase 3 entry gate.

---

## 4. Audit log + changelog (🟧 major, Phase 2a)

**What's missing.** Starter ships:

- [starter-changelog](../../../crates/starter-changelog) — the
  append-only change envelope + visibility registry.
- [starter-audit](../../../crates/starter-audit) — read-only
  user-audit projection.
- [starter-agent-log](../../../crates/starter-agent-log) — the
  read-only AI-agent projection.

Rubix's SCOPE mentions `starter-audit` in the consumed table but
does **not** wire it to any phase. Per `starter-authz`, decision
audit lands in the configured sink — but rubix never specifies
that sink.

**Why it matters.** Goal 2 (user admin) is exactly the surface
that needs an unambiguous audit trail. Disabling a user must be
attributable to a specific principal at a specific time.
`starter-agent-log` is rubix-shaped: every agent turn is the
natural log source.

**Proposed promotion.**

- **Phase 2a deliverable:** `starter-changelog-postgres` migration
  runs; `starter-audit` projection wired; every user-admin tool
  writes a changelog row.
- The SSE-events rule strengthens: every `agent.turn.start` event
  implicitly produces a `starter-agent-log` row carrying the
  active skill.
- **New design doc:** `docs/design/audit/README.md` — the
  changelog/audit/agent-log wiring. Update
  `docs/design/auth/README.md` to cross-link.

**Owner:** Phase 2a entry gate (add `audit/` alongside `auth/`,
`migrations/`).

---

## 5. Insights / quality flags (🟨 important, Phase 4+)

**What's missing.** [starter-insights](../../../crates/starter-insights)
provides `RuleRegistry`, `QualityFlagRegistry`, and rust/rhai/sql
rule kinds. Rubix's SCOPE mentions it once in the consumed table
but does not commit it to a phase.

**Why it matters.** Goal 6 (analytics reports) is exactly an
insights consumer — "is this metric outside its normal band?" is
a quality flag, not a raw query. Goal 5 (system checks) overlaps:
disk-usage thresholds are insights rules.

**Proposed promotion.**

- **Phase 4 deliverable:** the system-check skill consults
  `starter-insights` rules instead of hard-coded thresholds. The
  analytics-reporter goal produces reports that *cite* triggered
  quality flags.
- Upstream-first check: rubix-specific rule definitions go where?
  Likely `starter-insights` itself grows a "default rule pack";
  rubix contributes domain rules.
- **New design doc:** `docs/design/insights/README.md` or extend
  `docs/design/warehouse/README.md`.

**Owner:** Phase 4 entry gate.

---

## 6. Secrets — beyond the file backend (🟨 important, deployment-shape)

**What's missing.** SCOPE consumes `starter-secrets-file` by
default and lists `starter-secrets-keyring` as opt-in. But the
deployment shape (containerised vs. desktop appliance) drives
which backend is correct, and rubix does not say.

**Proposed promotion.**

- **New design doc:** `docs/design/deploy/README.md` (or extend
  `docs/design/overview/README.md`) — rubix in a container uses
  file-backed; rubix on an operator's desktop uses keyring.
- Phase 0 already ships file-backed (default); document the
  switch as a config knob.

**Owner:** before any external deployment story exists.

---

## 7. Blob storage (🟧 major, Phase 3+)

**What's missing.** Starter has a full blob family
(`starter-blob-{memory,fs,s3,garage,compose,axum}`). Rubix does
not mention any. But:

- Goal 1 (dashboards) may embed user-uploaded images.
- Goal 6 (analytics reports) likely renders to PDF and stores
  somewhere — `starter-export` writes; rubix needs a place to put
  the rendered bytes.
- Extension bundles (Phase 5) are blobs.

**Proposed promotion.**

- **Phase 3 entry gate:** decide blob backend per deployment
  shape (`starter-blob-fs` for single-host, `starter-blob-s3` for
  cloud). Default is `starter-blob-fs`.
- **Phase 4 deliverable:** weekly-report flow writes to the blob
  store; the resulting URL is the report's primary surface.
- **Phase 5:** extension bundles loaded via `starter-blob-*`
  rather than direct filesystem.
- **New design doc:** `docs/design/blobs/README.md`.

**Owner:** Phase 3 entry gate.

---

## 8. Export pipeline (🟨 important, Phase 4)

**What's missing.** [starter-export](../../../crates/starter-export)
provides a PDF / HTML / CSV / JSON pipeline. Goal 6 (analytics
reports) is the canonical consumer. Rubix's SCOPE does not name
the crate.

**Proposed promotion.**

- Add `starter-export` to the consumed-crates table.
- **Phase 4 deliverable:** `rubix.analytics.report` tool returns
  a `starter-export` job id; the rendered file is fetched from
  the blob store (item 7).
- **Design doc:** extend `docs/design/warehouse/README.md` or
  create `docs/design/reports/README.md`.

**Owner:** Phase 4 entry gate.

---

## 9. Tags (🟨 important, Phase 3 + Phase 4)

**What's missing.** [starter-tags](../../../crates/starter-tags)
ships a shared tag vocabulary (`TagSet`, `TagQuery`,
`TagDefinition`) with PG/CH/in-process compilation targets — and
the SCOPE warehouse rule says "tag types are `Bool | Str` only".

But rubix's tool inventory has no tag-related tool. Without one,
dashboards cannot filter "show me the alarms tagged
`tenant=acme`", flows cannot subscribe by tag, and the
warehouse's tag-driven reads have no operator surface.

**Proposed promotion.**

- Add `rubix.tag.{create,assign,list}` tools to Goal 1 or as a
  new shared toolset under `rubix-tools::tags`.
- The contracts rule that fixes tag types `Bool | Str` should
  cite `starter-tags` explicitly.
- Possible upstream: `starter-tool-tags` per the upstream-first
  rule.
- **New design doc:** `docs/design/tags/README.md` or extend the
  warehouse doc.

**Owner:** Phase 3 entry gate.

---

## 10. Cache (🟩 worth confirming, Phase 4+)

**What's missing.** [starter-cache](../../../crates/starter-cache)
exists as a thin async cache trait with pluggable backends. Rubix
doesn't mention it.

**Position.** Probably **Non-goal for v0**. Per the observable-
state rule, caches are explicitly *not* nodes; they're in-memory
fields. A tool that needs caching should do it inline or via
`starter-cache` per its own discretion, not as a SCOPE decision.

**Proposed promotion.** Add to Non-goals: "Cache strategy is
per-tool, not a SCOPE decision."

**Owner:** Phase 4 (just to confirm).

---

## 11. Service surfaces (Slack, Telegram, email) (🟩 worth confirming, deferred)

**What's missing.** Starter has `starter-service-slack`,
`starter-service-telegram`, and an `EventSink` pattern. Rubix's
Goal 5 (`rubix.alert.send`) is a service consumer but the SCOPE
doesn't say which service.

**Proposed promotion.**

- **Phase 1 deliverable:** `rubix.alert.send` is a thin shim that
  the operator wires to either Slack, Telegram, email
  (`starter-service-email` does not exist yet — upstream
  candidate), or a log line.
- **New design doc:** `docs/design/alerts/README.md`.

**Owner:** Phase 1.

---

## 12. CLI building blocks beyond clap (🟨 important, Phase 2b)

**What's missing.** [starter-cli](../../../crates/starter-cli)
provides clap building blocks. Rubix's Phase 2b says "CLI hits the
same endpoints" — but doesn't say *how* the subcommand-per-tool
mapping happens.

**Open question:** does `starter-cli` auto-generate a subcommand
per tool from the `ToolRegistry`? That would be the right shape;
if it doesn't, it's an upstream candidate.

**Proposed promotion.**

- **Phase 2b entry gate:** confirm `starter-cli`'s subcommand-from-
  registry support. If absent, file the upstream PR per
  `docs/design/starter-changes/README.md`.
- **Design doc:** extend `docs/design/agent/README.md` with the
  CLI surface description once decided.

**Owner:** Phase 2b entry gate.

---

## 13. Port management (🟩 worth confirming)

**What's missing.** [starter-port](../../../crates/starter-port)
auto-picks a free port (Vite-style). Useful in dev; mostly
irrelevant for production. Not in SCOPE.

**Position.** Confirm Non-goal for v0 production, but rubix-agent
should use it in dev mode (where the operator may run multiple
agents). Already implicitly handled by `RUBIX_BIND` env var in
the Phase 0 binary.

**Owner:** no action needed; documented here for completeness.

---

## 14. Tauri (🟩 explicit Non-goal — confirm)

**What's missing / not missing.** [starter-tauri](../../../crates/starter-tauri)
exists. SCOPE Non-goals correctly excludes it ("no frontend").

**Position.** Already covered. Listed here only so the audit is
complete.

---

## 15. Config layering (🟨 important, Phase 0+)

**What's missing.** Phase 0 ships with hard-coded defaults +
`RUBIX_BIND` env var. [starter-config](../../../crates/starter-config)
ships a four-layer loader (defaults < file < env < flags).
Rubix's SCOPE references the crate in the consumed table but
phases never use it.

**Proposed promotion.**

- **Phase 2a deliverable:** swap `RUBIX_BIND` for a proper
  `starter-config`-loaded config file. The migration ordering
  + DB connection string + secret-store backend + LLM provider
  config all flow through one loader.
- **New design doc:** `docs/design/config/README.md` documenting
  the schema.

**Owner:** Phase 2a (when the first non-trivial config exists).

---

## 16. Flow surfaces — `FlowAsService` (🟧 major, Phase 4) — ✅ addressed in branch `codeless/rubix-goal-6-weekly-report`

**Status.** Addressed. The durable cron scheduler landed upstream
across three commits:

- `starter-cron` crate — 5/6/7-field cron grammar + `next_fire`
  evaluator (replaces the 5-field-only parser that previously
  rejected bundled `weekly-report.yaml`).
- `starter-store-postgres` migration `scheduled_flows/0001_init.sql`
  — durable table with a `pg_notify('starter_scheduled_flows', …)`
  trigger on insert + `next_run_at` / `enabled` change.
- `starter-flow-surfaces::FlowAsService` — `register_schedule` /
  `unregister_schedule` API, `Clock` trait (`SystemClock` +
  `TestClock`), and a `tick()` loop claiming due rows via
  `SELECT FOR UPDATE SKIP LOCKED LIMIT 32`, dispatching through
  `FlowRunner`, then writing `last_run_*` + recomputing
  `next_run_at`.

Rubix-agent now seeds `scheduled_flows` at boot from every bundled
YAML carrying `trigger: schedule`, and the `[scheduler]` section
of `AgentConfig` toggles the tick task. The
`FlowAsTool` / `FlowAsService` distinction is documented in
[`docs/design/scheduling/README.md`](../design/scheduling/README.md)
and cross-linked from [`docs/design/flows/README.md`](../design/flows/README.md)
+ [`docs/design/agent/README.md`](../design/agent/README.md).

The upstream PR ledger now carries the three corresponding entries
under [`docs/design/starter-changes/`](../design/starter-changes/README.md).

---

## 17. Skills CLI / `mani` integration (🟨 important, throughout)

**What's missing.** Rubix's `mani.yaml` covers build / test / run /
healthz. It does **not** include skill / flow / extension
operations:

- `mani run skill-list` — list approved + quarantined skills.
- `mani run skill-approve <hash>` — approve a quarantined skill.
- `mani run flow-deploy <yaml>` — deploy a flow from disk.
- `mani run ext-load <dir>` — load an extension into a running
  rubix-agent.

**Proposed promotion.** Add these tasks to `mani.yaml` as they
become relevant (each phase introduces the ones it needs).

**Owner:** Phase 1+ (incremental).

---

## 18. ADR series (✅ already seeded)

**Status.** Done. Three ADRs landed in `docs/adr/`:

- [0001 — Postgres only](../adr/0001-postgres-only.md)
- [0002 — Backend only](../adr/0002-backend-only.md)
- [0003 — Agent is starter's ai-agent](../adr/0003-agent-is-starter-ai-agent.md)

When a future ADR is needed, follow the format in
[docs/adr/README.md](../adr/README.md). No action outstanding.

---

## 19. `starter-ai` provider routing (🟨 important, Phase 1)

**What's missing.** SCOPE consumes `starter-ai`. The Phase 1
boot sketch in `docs/design/agent/README.md` shows
`starter_ai::registry::Registry::with_defaults().get(&Provider::ClaudeCli)`.
But:

- What if Claude CLI isn't installed on the host?
- Does rubix support fallback (Claude CLI → Anthropic REST →
  Copilot CLI)?
- How is the provider selected — config, per-flow, per-skill?

**Proposed promotion.**

- **Phase 1 deliverable:** provider selection lives in
  `starter-config` (one knob, one provider; no auto-fallback for
  v0).
- **New design doc:** `docs/design/ai-providers/README.md`.

**Owner:** Phase 1.

---

## 20. Backup / restore (🟩 already deferred — verify)

**What's already covered.** SCOPE Non-goals correctly lists
"No backup/restore in v0." Six places hold state.

**Open thread.** When Phase 6+ adds backup, the design lives in
`docs/design/backup/` per the doc-tier rule. Noted here so the
deferred-doc placeholder is captured.

**Owner:** post-v0.

---

## 21. Rate limiting (🟩 already deferred — verify)

**What's already covered.** SCOPE Non-goals correctly lists "No
agent rate limiting / per-tenant token quota in v0."

**Owner:** post-v0.

---

## 22. Doc tier discipline (🟧 major, structural — always-on)

**What's been put in place.** The four-tier doc model is now
load-bearing:

- `docs/sessions/` — throwaway notes.
- `docs/scope/` — this file lives here; plans, not the system.
- `docs/adr/` — immutable decisions.
- `docs/design/<area>/README.md` — present-tense canonical
  description, **the only tier code may link**.

[HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md) defines it;
[NEW-SESSION.md §2](../../NEW-SESSION.md) enforces it at session
boot.

**What needs to keep happening.**

- Every phase that introduces a capability **creates or extends a
  `docs/design/<area>/README.md` in the same PR** as the code.
  Phase exit reviewers verify the doc references in source files
  actually resolve.
- Every session note that has settled is promoted to a design doc
  per HOW-TO-CODE §0a, and the session note is deleted.
- A grep-and-CI check (future) catches any new code comment that
  references `SCOPE.md`, `HOW-TO-CODE.md`, `NEW-SESSION.md`,
  `docs/scope/`, or `docs/sessions/`.

**Why it's a gap not a closed item.** The rule is written; the
**CI lint that enforces it** is not. Until the lint exists,
discipline is honour-system.

**Proposed promotion.**

- **Phase 1 deliverable:** a simple grep-based pre-commit hook
  (or `mani run lint-doc-refs` task) that fails if any
  `rubix/crates/**/*.rs` file references a forbidden doc path.

**Owner:** Phase 1.

---

## Summary table — what gets promoted, where

| # | Item | Sev | Target | Design doc landing site |
|---|---|---|---|---|
| 1 | i18n + prefs end-to-end demo | 🟥 | Phase 1 entry gate | `docs/design/i18n-prefs/` |
| 2 | Undo / redo for write goals | 🟧 | Phase 3 entry gate | `docs/design/undo/` |
| 3 | Clipboard for duplicate | 🟧 | Phase 3 entry gate | `docs/design/undo/` (shared) or `clipboard/` |
| 4 | Audit log + `starter-agent-log` | 🟧 | Phase 2a entry gate | `docs/design/audit/` |
| 5 | Insights / quality flags | 🟨 | Phase 4 | `docs/design/insights/` |
| 6 | Secrets backend per deploy shape | 🟨 | any time | `docs/design/deploy/` |
| 7 | Blob storage | 🟧 | Phase 3 entry gate | `docs/design/blobs/` |
| 8 | Export pipeline | 🟨 | Phase 4 | `docs/design/reports/` or `warehouse/` |
| 9 | Tags toolset | 🟨 | Phase 3 | `docs/design/tags/` or `warehouse/` |
| 10 | Cache strategy | 🟩 | Non-goal note | — |
| 11 | Alert sinks (Slack/Telegram/email) | 🟩 | Phase 1 | `docs/design/alerts/` |
| 12 | CLI subcommand-from-registry | 🟨 | Phase 2b entry gate | extend `agent/` |
| 13 | Port picker | 🟩 | done (RUBIX_BIND) | — |
| 14 | Tauri | 🟩 | already Non-goal | — |
| 15 | Config layering | 🟨 | Phase 2a | `docs/design/config/` |
| 16 | `FlowAsService` named | ✅ | addressed in `codeless/rubix-goal-6-weekly-report` | `docs/design/scheduling/` + starter-changes ledger |
| 17 | `mani` skill/flow/ext tasks | 🟨 | incremental | none (config) |
| 18 | ADR series | ✅ | done | `docs/adr/` |
| 19 | AI provider routing | 🟨 | Phase 1 | `docs/design/ai-providers/` |
| 20 | Backup (deferred) | 🟩 | post-v0 | `docs/design/backup/` (later) |
| 21 | Rate limiting (deferred) | 🟩 | post-v0 | — |
| 22 | Doc-tier lint | 🟧 | Phase 1 deliverable | — (CI / hook) |

## Next actions

1. **Most critical:** item 1 (i18n + prefs). Decide whether Phase 1
   entry gate gets the demo, or whether it's a Phase-1-exit gate.
   Either way, ship an EN + ES catalogue from day one, a
   `Quantity` round-trip test, and the new
   `docs/design/i18n-prefs/README.md`.
2. **Audit log (item 4)** lands in Phase 2a alongside auth — same
   PR draws `docs/design/audit/README.md`.
3. **Items 2 + 3 + 7 + 9 + 16** all converge on Phase 3. Audit
   whether Phase 3 needs a split (Phase 3a dashboards, Phase 3b
   flow programmer) once these items are folded in.
4. **Doc-tier lint (item 22)** is a Phase 1 deliverable — shipping
   it early prevents drift across every later phase.
