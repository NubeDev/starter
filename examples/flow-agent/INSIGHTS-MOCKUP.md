# Insights Mockup — Scope

A UI mock-up inside `examples/flow-agent` that demonstrates the
end-user experience of the [Insights capability](../../DOCS/Insights/SCOPE.md).
The **real `starter-ai` agent** drives create / edit / query of rules;
**fake data lives in JSON fixtures** served by thin Axum handlers so
the agent (and the UI) read it via the same REST surface a real
Insights backend would expose.

> Parent: [`examples/flow-agent/SCOPE.md`](./SCOPE.md). All F-rules
> apply (single binary, reuse packages verbatim, SSE+REST only, no
> login, no CLI, Apple-modern shadcn aesthetic, nested sidebar).
> This document adds **I-rules** for the Insights surface only.

---

## One-line summary

Four new pages in `frontend/src/pages` (`RulesList`, `RuleEditor`,
`PipelineCanvas`, `VerdictsView`) plus one new fixture-backed REST
module (`src/insights_mock.rs`) that lets the real agent author,
chain, and query rules over fake-but-shaped-correctly data.
Printable verdict reports via `@media print` on the detail pages.

## Why this exists

[`DOCS/Insights/SCOPE.md`](../../DOCS/Insights/SCOPE.md) decides
*what* Insights is. There is currently no surface that shows what it
*feels like* to live in. A mock-up answers four questions the SCOPE
deliberately doesn't:

1. What does the operator actually click when they want to write a
   rule? Drag from a palette, type into a code box, or talk to the
   agent?
2. How does the agent surface a *proposal* vs an *applied* edit?
3. What does a "nice printable" verdict report look like for a
   building manager who lives in PDFs and email?
4. How does the read path *feel* — is it instant, or is there a
   spinner that the SCOPE's materialisation contract is meant to
   prevent?

We answer these against fake data so the mock-up ships in days, not
weeks, and so design decisions surface before any Insights crate
exists.

---

## Hard rules (I-rules)

### I1 — Real agent, fake data

The agent is the real `starter-ai` runner already wired in
flow-agent. It calls real tools that read/write JSON fixtures via the
real REST surface. No mocked LLM. No mocked HTTP. The only thing
that's fake is the **storage layer** — fixture files on disk in place
of the verdict log / rule registry / rollup tables. This is
load-bearing: the agent's prompt, tool definitions, streaming, and
error paths must be the ones we'll ship.

### I2 — Fixtures are the schema preview

Fixture JSON shapes match the eventual Insights schema (rule rows,
verdict rows, coverage, tags, evidence). When the real
`starter-insights` crate lands, the fixture loader is replaced with a
SQL query and **no UI / agent code changes**. If a UI element needs
a field the SCOPE doesn't define, that's a SCOPE bug — flag it, fix
SCOPE, then add to fixtures. Never invent fields in fixtures alone.

### I3 — No new packages, no new crates

All Rust changes land in `examples/flow-agent/src/`. All frontend
changes land in `examples/flow-agent/frontend/src/`. Reuse
`starter-ui-flow` for the pipeline canvas, `starter-ui-chat` for the
agent surface, `starter-ui-kit` for everything else. F2 still binds.

### I4 — Print is a first-class surface

The verdict report page has `@media print` CSS that produces a
clean, single-column, header-and-table layout. `Ctrl+P` → PDF must
look like something a facilities manager would attach to an email
without apology. Tested in the F6 checklist.

### I5 — The mock-up is throwaway, the JSON shapes are not

The pages, hooks, and CSS may be rewritten when the real Insights
crate lands. The fixture JSON shapes (rule, verdict, coverage, tags,
evidence) are **the wire contract preview** and must match what
SCOPE describes. Treat fixtures with more care than UI.

---

## Surfaces

### Frontend pages (all new)

```
frontend/src/pages/
  RulesList.tsx         table of rules from /api/insights/rules
                        columns: id, kind, namespace, severity, tags,
                        recent-verdict-summary (sparkline), updated_at
                        actions: New rule, Open editor, Chat about rule

  RuleEditor.tsx        split pane:
                        - left: rule body editor (Monaco) +
                          schema panel (id, kind, severity, tags,
                          retroactive, idempotent — D5/D6 fields)
                        - right: agent chat scoped to this rule
                          (uses starter-ui-chat; agent tools below)
                        - bottom: dry-run output (verdict + evidence)

  PipelineCanvas.tsx    wraps starter-ui-flow's editor; node palette
                        includes the six rule kinds + align, window.*,
                        verdict.join, rollup.*. Edges typed
                        Dataset/Verdict/Frame.
                        Agent dock at the right.

  VerdictsView.tsx      filterable list + detail. Filter by rule_id,
                        tag, severity, time range. Detail page has
                        print stylesheet (I4): rule header, verdict
                        outcome, evidence rows, coverage, AI
                        explanation if present, signature line.
```

### Sidebar additions (in `app-sidebar.tsx`)

A new collapsible "Insights" section above Pages:

```
Insights
├── Rules           → /insights/rules
├── Pipelines       → /insights/pipelines       (reuses FlowsList shape)
└── Verdicts        → /insights/verdicts
```

`AgentsList` and `AgentChat` stay where they are; the agent docks
*into* Rule Editor and Pipeline Canvas via `starter-ui-chat`, it
doesn't move.

### Backend (Rust)

One new module:

```
src/insights_mock.rs
  - GET    /api/insights/rules             list
  - GET    /api/insights/rules/:id         detail (+ recent verdicts)
  - POST   /api/insights/rules             create  (writes JSON)
  - PATCH  /api/insights/rules/:id         update  (writes JSON)
  - POST   /api/insights/rules/:id/dry-run synthesises a verdict
                                           from fixtures, no engine
  - GET    /api/insights/verdicts          list (filter by rule_id,
                                           tag, severity, time)
  - GET    /api/insights/verdicts/:id      detail
  - GET    /api/insights/pipelines         list
  - GET    /api/insights/pipelines/:id     detail (graph)
  - POST   /api/insights/pipelines         create/update graph
```

All handlers are thin: read/write the JSON files in `fixtures/insights/`
under a `tokio::sync::RwLock<InsightsFixtures>` cached in app state.
No engine. No SQL. Reload on file change is not required (refresh-
based is fine for a mock).

### Agent tools (registered with the real `AiRunner`)

The agent gets exactly these tools, all backed by the same REST
endpoints above:

| Tool | Purpose | Maps to skill bundle |
|---|---|---|
| `insights.rule.list` | discover rules | (utility) |
| `insights.rule.read` | read a rule body + schema | (utility) |
| `insights.rule.propose` | propose a new rule (returns a *proposal*, not a write) | `rule-author` |
| `insights.rule.apply` | apply an approved proposal | `rule-author` |
| `insights.rule.dry-run` | synthesise a verdict from fixtures | `rule-author` |
| `insights.verdict.query` | filter verdicts (rule, tag, severity, time) | `explain` |
| `insights.verdict.explain` | narrate a verdict in plain English | `explain` |
| `insights.pipeline.read` | read pipeline graph | (utility) |
| `insights.pipeline.propose-edit` | propose adding/removing/wiring nodes | `rule-author` |
| `insights.pipeline.apply-edit` | apply pipeline edit | `rule-author` |

`propose` vs `apply` is load-bearing: the agent never silently
mutates. The UI shows a diff card; the operator clicks Approve.

---

## Fixtures

```
examples/flow-agent/fixtures/insights/
  rules.json         array of rule objects (see I2)
  verdicts.json      array of verdict objects
  pipelines.json     array of pipeline graphs
  coverage.json      synthetic coverage timeseries per rule
  tags-index.json    tag → rule_ids lookup
  README.md          how to regenerate / what's where
```

Seed content covers three reference scenarios:

1. **IoT** — `device.online@1`, `sensor.has-recent-data@1`,
   `sensor.in-range@1` across 12 devices, 7 days of verdicts, a few
   intentional `Severity::Error` rows.
2. **Energy** — a building (`hq-london`) with `meter.baseline-deviation@1`,
   `meter.weather-normalised-overrun@1` (custom Rhai), 30 days of
   verdicts including a gap day (`partial-onboarding`) and a
   retroactive-correction day (D5 from the Insights SCOPE).
3. **Bills reconciliation** — the canonical multi-source chain
   (meter + weather + tariff + occupancy → align → derive → assert
   → AI judge → explain), 1 month, with intentional dirty data and
   one AI-judge verdict.

All three are minimal — enough rows to *look* real, not enough to
need indexing logic in the mock backend.

---

## Print stylesheet (I4 detail)

`VerdictsView.tsx` detail and `RuleEditor.tsx` detail both ship a
print stylesheet:

- Hide sidebar, header, agent dock, all chrome.
- Title + rule id + verdict id + signed-by + generated-at as a
  printable header.
- Body: rule schema table, verdict outcome (severity badge ok in
  greyscale), evidence rows as a real `<table>`, coverage as a
  `<details>`-collapsed appendix, AI explanation as a quoted block
  with model id + cost line for auditability.
- Footer: page numbers via `@page`, a single faint line, no logo
  unless one is configured.
- Tested by F6 checklist: `Ctrl+P` → preview screenshot diff.

---

## Pages — agent interaction patterns

### RuleEditor (the heart of the demo)

Two-pane layout. Left pane is the rule code + schema. Right pane is
agent chat scoped to this rule. Three button states the agent
produces:

```
[ Proposed change ]   diff preview, Approve / Discard / Refine
[ Applied ]           green check, undo within 30s
[ Dry-run result ]    verdict card with evidence
```

Typical script the demo must support, end to end, with the real
agent:

> "Write me a rule that flags when this building's kWh is more than
> 20% above last week's same-hour baseline, weather-normalised."
>
> Agent reads `rules.json` for tag conventions, drafts a `rule.rhai`
> body using fixture column names, posts to
> `insights.rule.propose`. UI shows the diff. Operator clicks
> Approve. UI calls `insights.rule.apply`. Agent then calls
> `insights.rule.dry-run` and narrates the resulting verdict.

### VerdictsView

Two scripts:

> "Why did `meter.baseline-deviation@1` flag building hq-london
> yesterday at 14:00?"
>
> Agent calls `insights.verdict.query` with the filter, picks the
> matching verdict, calls `insights.verdict.explain`, returns a
> narrative referencing evidence rows.

> "Print me the last week's verdicts for hq-london."
>
> Agent surfaces a verdict list, operator clicks one, hits Ctrl+P.
> (Bulk print is out of scope for the mock-up.)

### PipelineCanvas

> "Add an AI judge after the baseline deviation rule and route low-
> confidence verdicts to suppress."
>
> Agent calls `insights.pipeline.read`, computes a graph patch,
> calls `insights.pipeline.propose-edit`. UI shows the patched
> graph in a diff overlay (added nodes highlighted green, edges
> dashed). Operator clicks Approve.

---

## Phases

- **Phase 1.** Fixtures + `insights_mock.rs` + `RulesList.tsx` +
  `VerdictsView.tsx` (list + detail with print stylesheet, no
  editor, no agent tools yet). Sidebar wired. Demo: browse rules
  and verdicts, print a verdict.
- **Phase 2.** `RuleEditor.tsx` + agent tools `rule.list`,
  `rule.read`, `rule.propose`, `rule.apply`, `rule.dry-run`. Demo:
  the rule-authoring script above end to end.
- **Phase 3.** `VerdictsView` upgraded with agent dock; tools
  `verdict.query`, `verdict.explain`. Demo: the explain script.
- **Phase 4.** `PipelineCanvas.tsx` (wraps `starter-ui-flow`'s
  editor against `pipelines.json`) + tools `pipeline.read`,
  `pipeline.propose-edit`, `pipeline.apply-edit`. Demo: the
  pipeline-edit script.

Each phase is independently demoable; later phases don't break
earlier ones.

---

## Non-goals

- No real rule execution. Dry-run synthesises a plausible verdict
  from fixtures; it does not evaluate Rhai / SQL / Rust.
- No real backfill. Fixtures already contain "historical" rows.
- No streaming verdicts. The mock is poll-based on the existing SSE
  channel only where flow-agent already streams.
- No auth, no multi-tenant. Single fake operator. F4 binds.
- No real `starter-insights` crate. When that lands, this mock-up's
  backend module is deleted; the frontend pages remain mostly
  intact thanks to I2.
- No PDF generation server-side. Browser print is the only export
  path (I4).

---

## Acceptance checklist

- [ ] Fixtures cover IoT + Energy + Bills scenarios with at least
      one intentional dirty-data row each.
- [ ] All ten agent tools registered and visible in the agent's
      tool list at session start.
- [ ] `propose` → diff card → `apply` flow demonstrably round-trips
      to fixture JSON on disk.
- [ ] `Ctrl+P` on a verdict detail produces a one-page, sidebar-
      free, chrome-free print preview.
- [ ] A retroactive-correction verdict renders with the
      `starter.quality.retroactive-correction@1` flag visible
      (sanity check against [SCOPE.md D5](../../DOCS/Insights/SCOPE.md)).
- [ ] No new packages in `packages/`. No new crates in `crates/`.
      No source copied from `starter-ui-*` into the example.
- [ ] Switching theme (light/dark) does not break the print
      stylesheet — print is always light.
- [ ] Agent never writes a rule without an Approve click; verified
      by the audit trail in fixture commits.

---

## Open decisions

- **MD1 — Editor: Monaco vs CodeMirror.** Monaco gives Rhai/SQL
  syntax via existing language workers; CodeMirror is lighter. Lean
  Monaco unless the bundle-size budget complains.
- **MD2 — Where the agent proposal "diff card" lives.** Inside the
  chat bubble (chat-native), or as a side panel that the chat
  references by id? Side panel is cleaner for pipeline edits; chat
  bubble is cleaner for rule edits. Probably both, gated by tool.
- **MD3 — Fixture write strategy.** Each `apply` overwrites the
  whole file, or appends to a journal? Overwrite is simpler;
  journal supports the undo-within-30s pattern. Likely journal,
  flushed on tab close.
- **MD4 — How much of the print stylesheet ships in
  `starter-ui-kit`.** If the print layout is genuinely reusable for
  any tabular detail page, lift it. If it's verdict-specific, keep
  it local. Decide once Phase 1 ships.
