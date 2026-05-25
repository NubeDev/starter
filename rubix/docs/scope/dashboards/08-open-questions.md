# 08 — Open questions

> **Tier:** scope (plan). Lifetime: weeks. Not referenced from code.
> See [README.md](./README.md). Every question carries a **default**
> the implementation falls back to if the operator does not answer
> before the relevant slice lands.

## How to use this file

Each entry is `Qn — <one-line question>` with:

- **Why it matters** — what changes downstream depending on the answer.
- **Options** — the candidate decisions, with the rejected ones
  marked.
- **Default** — what we ship if nobody answers.
- **Cost to revisit** — how expensive it is to change after the
  fact.

When the operator answers, fold the answer into the relevant
numbered scope file and delete the question here.

---

## Q1 — Page storage: rubix-owned PG table, or starter resolver store?

**Why it matters.** Determines who owns the schema, the
migration, and the write path. Affects upstream API surface and
who can register page-level authz resources.

**Options.**

- **A — write API in `starter-sdui-routes`.** Rubix calls
  `state.pages.upsert(...)`. Requires upstream changes.
  *Rejected pending the upstream change landing.*
- **B — rubix-owned PG table.** Rubix implements `PageProvider`
  read-side; writes go through a rubix-owned store. **Selected.**

**Default.** Option B, per [01-storage.md](./01-storage.md).

**Cost to revisit.** Low — the `PageProvider` trait is the seam;
the body of `lookup_page` can be re-pointed.

---

## Q2 — AI-authored pages: under which principal?

**Why it matters.** Affects `owner_principal` on the page row,
which gates `edit` / `delete` and routes `undo.last` properly.

**Options.**

- **A — under the LLM's flow-run principal.** Bad: every operator
  shares the same "AI" principal and can edit each other's
  AI-built pages.
- **B — under the operator who invoked the flow.** Good: each
  operator owns their own AI-built pages and undo works per-user.

**Default.** Option B. The flow runner already threads the
caller principal into `ai-agent`; tools see `Principal::subject`
as the invoking operator.

**Cost to revisit.** Low — one helper in `page_set.rs` derives
`owner_principal`.

---

## Q3 — Bundled-page upsert vs operator collision

**Why it matters.** If `rubix-agent`'s `boot/dashboards_seed.rs`
upserts bundled pages on every boot, an operator who renamed
`dashboard.overview` will lose their edit.

**Options.**

- **A — bundled wins.** Re-seed always. Operator edits are
  ephemeral. *Rejected — surprises the operator.*
- **B — operator wins.** Seed only if no row exists for the
  bundled `page_id` with `created_by != 'system'`. **Selected.**
- **C — fork on collision.** Re-seed creates `dashboard.overview-system`
  alongside the operator's `dashboard.overview`. Too magical.

**Default.** Option B. Documented in
[01-storage.md](./01-storage.md).

**Cost to revisit.** Low.

---

## Q4 — Live subscription transport: SSE, WebSocket, or polling?

**Why it matters.** Determines what `useSubscriptions()` does
under the hood. WS is more efficient at scale; SSE is simpler
and works through more proxies; polling is the laziest fallback.

**Options.**

- **A — SSE** (`GET /api/v1/ui/subscribe?subjects=...`). Server
  emits one event per slot write. **Selected for v1.**
- **B — WebSocket.** Better duplex; needed only if the client
  wants to push subject changes mid-session. Deferred.
- **C — Polling.** v1 fallback when SSE is unavailable
  (development without an SSE-aware reverse proxy).

**Default.** SSE with polling fallback baked into
`useSubscriptions`. The hook chooses based on the transport's
declared capability.

**Cost to revisit.** Medium — the transport interface is one
file; SSE → WS is a refactor of the impl, not the API.

---

## Q5 — Per-tenant authz isolation: row filter or schema-per-tenant?

**Why it matters.** v1 ships **one** Postgres database with
`tenant_id` columns on every page. Larger deployments may want
schema-per-tenant for hard isolation.

**Options.**

- **A — row filter** with `tenant_id` on every row, enforced by
  the authz layer. **Selected for v1.**
- **B — schema-per-tenant.** Stronger isolation; complicates
  migrations. Deferred indefinitely.

**Default.** Option A.

**Cost to revisit.** High — schema-per-tenant requires data
migration. Decide before scale matters.

---

## Q6 — Schemars-derived JSON schema as the AI's authoring contract

**Why it matters.** The LLM emits `body_json` as a typed
`ComponentTree`. `starter-ui-ir` has **53 variants** (confirmed:
`crates/starter-ui-ir/src/component.rs:237`); the full
`schemars`-generated schema is large enough that a naive single
`page_set` turn approaches the `0.50_usd` cost cap on its own.

**Options.**

- **A — full generated schema.** Token-heavy but always correct.
  *Rejected per peer review B5.*
- **B — pruned schema, selected per `skill_hint`.** The LLM only
  sees the variants relevant to the current authoring task
  (e.g. "kpi-grid" surface = `page`, `grid`, `kpi`, `card`,
  `text`, `divider`, `action`). Variants outside the subset are
  emitted as `Component::Custom` with a renderer-id the LLM
  cannot synthesise. **Selected for v1.**

**Default.** Option B. The pruned schema lives in
`rubix-skills/skills/dashboard-builder/schemas/` as one JSON
file per skill_hint subset; `build.rs` generates each by walking
the `schemars` root and filtering. The codeless implementer
**must** include a measured token count for the default subset
in the commit message — if it exceeds 3000 tokens, the subset is
still too broad.

**Cost to revisit.** Low — subsets are additive JSON files.

---

## Q7 — Renderer-id session cache (W7 hash optimisation)

**Why it matters.** Old rubix-agent's [`sessions.rs`](../../../../examples/rubix-agent/crates/dashboard-transport/src/sessions.rs)
lets the client send its full `custom_renderers` list once per
session and pass only a hash on subsequent resolves. Saves
bandwidth at scale.

**Options.**

- **A — ship in v1.** Adds session storage, hash negotiation.
  *Rejected — premature optimisation for v1.*
- **B — defer to v2.** **Selected.** Capability handshake stays
  inline on every resolve in v1.

**Default.** Option B.

**Cost to revisit.** Low — additive, no schema change.

---

## Q8 — Where does the seeded `dashboard.overview` page get its title and copy?

**Why it matters.** Per `docs/design/i18n-prefs/README.md`
(*"Domain code never holds a localised string."*), bundled page
bodies shipped from `rubix-tools` / `rubix-flows` **are** domain
content. EN literals in a bundled JSON would break the contract
in the same crate the design doc lives next to.

**Options.**

- **A — author title and labels as MessageKeys** via a new
  `$msg.<key>` binding source. `Text.content = "{{$msg.rubix.dashboard.overview.title}}"`.
  Resolver substitutes against the request locale's catalogue at
  resolve time. **Selected per peer review D13.**
- **B — author the page in EN, translate on demand.** *Rejected* —
  violates the i18n contract.
- **C — pre-translate seeds, one file per locale.** *Rejected* —
  fan-out and key drift.

**Default.** Option A. The `$msg` source is added in this
slice as a 6th `Source` enum variant in
[`02-bindings-gaps.md`](./02-bindings-gaps.md) (see G6, ~30 LOC
in `parse.rs` + `eval.rs`). Bundled pages cite MessageKeys; the
keys ship in the rubix `MessageBundle` (en + es) the same commit.

**Cost to revisit.** Low — `$msg` is additive.

---

## Q9 — `Component::Custom` renderer-id registration: file, env, or extension?

**Why it matters.** Extensions can already contribute UI variants
via `renderer_id`. The question is *where* the rubix-shipped
custom renderers are listed.

**Options.**

- **A — hard-coded in `@nube/rubix-client-react`.** OK for v1.
  **Selected.**
- **B — discovered via extension manifest at boot.** Right
  long-term; deferred to Phase 4 extension work.

**Default.** Option A.

**Cost to revisit.** Low.

---

## Q10 — RSQL-backed `Table` source: v1 or v2?

**Why it matters.** The IR's `Component::Table.source` accepts
RSQL queries. The host's `QueryEngine` impl in v1 either
implements them or returns a `Diagnostic`.

**Options.**

- **A — full RSQL in v1.** Lifts more of
  `domain-rsql-aggregation` from the old rubix-agent.
  *Defer — too big.*
- **B — kind-only filter in v1.** `kind==<x>` works;
  conjunctions / aggregations return a `Diagnostic`. **Selected.**
- **C — no table source in v1.** Bundled pages avoid `Table`s.
  Worse demo.

**Default.** Option B. Documented in
[03-host-glue.md](./03-host-glue.md) under `RubixQueryEngine`.

**Cost to revisit.** Medium — but additive; existing queries
keep working.

---

## How an AI session uses this file

When starting work on slice N, read questions cross-referenced
to that slice (the slice's own scope file points back here). If
the default is acceptable, proceed. If not, ask the operator
**before** writing code — answering after the fact costs
refactors.
