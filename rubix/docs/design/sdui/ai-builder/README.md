# AI dashboard builder (Goal 1 flow + skill)

## What this design covers

The Goal-1 flow that lets an operator say *"make me a page
showing disk usage for every host I own"* and get back a real
dashboard. The AI never writes JSX, HTML, or CSS; it emits typed
tool arguments and the server renders.

Depends on [01-storage.md](./01-storage.md) (page persistence),
[02-bindings-gaps.md](./02-bindings-gaps.md) (templates actually
work), [03-host-glue.md](./03-host-glue.md) (SDUI wired), and
[04-tools.md](./04-tools.md) (verbs have bodies).

## The flow

`rubix/crates/rubix-flows/flows/dashboard-assistant.yaml`:

```yaml
id: com.rubix.dashboard-assistant
description: |
  Build, edit, list, and explain rubix dashboards via SDUI.
trigger:
  type: explicit
nodes:
  - id: builder
    kind: ai-agent
    config:
      skill_hint: com.rubix.dashboard-builder
      session_policy: continue
      cost_cap: 0.50_usd
      allowed_tools:
        - rubix.dashboard.list
        - rubix.dashboard.get
        - rubix.dashboard.page_set
        - rubix.dashboard.duplicate
        - rubix.dashboard.delete
        - rubix.dashboard.history
        - rubix.tags.list           # so the LLM can scope by tag
links: []
```

Same shape as `goals/2/3/4`. No new node kinds, no new agent
infrastructure. The `ai-agent` node + skill carries the prompt
contract; the flow is metadata.

## The skill

[`rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md`](../../../crates/rubix-skills/skills/dashboard-builder/SKILL.md)
already exists. Update its `allowed_tools` list to include
`get`, `duplicate`, `delete`, `history`, and `tags.list`. Keep
the existing rules ("don't invent widget types", "ask before
spawning a 4-panel dashboard for a single-metric ask").

## The JSON dialect the LLM emits

The LLM produces a **`ComponentTree`** JSON object via the
`page_set` tool's `body_json` argument. That is the same wire
shape `/api/v1/ui/resolve` returns — there is exactly **one**
authoring surface, regardless of source (operator click in the
UI, AI tool call, codeless job).

To make this practical for a token-limited LLM, the
`dashboard.page_set` request schema is **strict**: the OpenAPI
spec for `body_json` is a **pruned** `ComponentTree` JSON
schema (decision in
[`08-open-questions.md#q6`](./08-open-questions.md#q6--schemars-derived-json-schema-as-the-ais-authoring-contract)).
The LLM gets a validation error on first try, fixes it on the
second turn.

### Pruned-schema delivery

`schemars::JsonSchema` is derived on every IR type
([`crates/starter-ui-ir/src/`](../../../../crates/starter-ui-ir/src/)).
A `build.rs` in `rubix-skills/skills/dashboard-builder/` emits
one JSON schema per skill-hint subset:

```
rubix/crates/rubix-skills/skills/dashboard-builder/schemas/
  kpi-grid.json       ← page, grid, row, col, card, kpi, text, divider, action
  table-view.json     ← page, table, action, ref_picker, date_range
  chart-board.json    ← page, grid, card, chart, kpi, text
  detail-page.json    ← page, row, col, card, kpi, text, divider, action
  full.json           ← every portable variant (G5); fallback only
```

The pruned schema lists only the relevant `Component` variants
in the `oneOf`. Variants outside the subset still parse server
side (so an operator-authored page from another skill isn't
rejected), but the LLM never sees them and so cannot synthesise
them.

**Codeless-implementer guard.** The PR must include a token
measurement of each subset (cl100k tokenizer, schema as the only
document). If any subset exceeds **3000 tokens** the subset is
still too broad — split it further before commit.

## Bundled "starter" pages the AI duplicates

Nine `include_dir!`-embedded JSON files under
`rubix/crates/rubix-flows/dashboards/`:

| File | Page id | Purpose |
|---|---|---|
| `index.json` | `dashboard.index` | **The operator-facing dashboard picker is itself an SDUI page** — a table of dashboards with row actions (open, duplicate, delete, history). Linked from the rubix nav. Proves the substrate carries its own admin UI. |
| `overview.json` | `dashboard.overview` | The current demo dashboard, reproduced in IR. Shipped as the default home. |
| `single-kpi.json` | `dashboard.single-kpi` | Smallest possible useful page: title + one `kpi` widget. The LLM's go-to for "show me one metric" prompts. |
| `disk-by-host.json` | `dashboard.disk-by-host` | Template: `Repeat` over hosts, one KPI per host. |
| `flow-health.json` | `dashboard.flow-health` | Table of flows with `last_run_at`, `error_count`, sparkline of `runs_per_hour`. |
| `single-host.json` | `dashboard.host` | Per-host detail, parameterised by `$target`. |
| `alarms.json` | `dashboard.alarms` | Active alarms with row actions (`acknowledge`, `mute`). |
| `team-usage.json` | `dashboard.team-usage` | Per-team principal counts + recent activity. |
| `blank.json` | `dashboard.blank` | One-card scaffold the AI starts from when "create a page about X" has no obvious template. |

`dashboard.index` deserves a note: rather than ship a hand-coded
React "dashboards list" route in `rubix/frontend/`, that screen
*is* an SDUI page that reads through `rubix.dashboard.list` via
the `/ui/table` source. This is the proof that the substrate is
sufficient for its own administration UI.

Log-tail / live-stream dashboards (proposed in peer review D9)
are deferred to v2 — they require SSE in the renderer's
subscription protocol, which v1 ships as polling-only.

These bundled pages double as:

1. The default content on a fresh install.
2. Fixtures for the substrate tests in
   [02-bindings-gaps.md](./02-bindings-gaps.md).
3. The skill's prompt-loadable "examples" — the SKILL.md cites
   these by `page_id`, the agent reads them via
   `rubix.dashboard.get`, and the LLM duplicates the closest
   match before editing.

Bundled pages are owned by `created_by="system"`. The AI
duplicates → operator gets an editable copy → AI further edits
that copy. The original is never mutated.

## i18n in bundled pages — MessageKey, never literal

Per [`08-open-questions.md#q8`](./08-open-questions.md#q8--where-does-the-seeded-dashboardoverview-page-get-its-title-and-copy)
bundled JSONs cite catalogue keys, never EN strings:

```json
{
  "variant": "text",
  "content": "{{$msg.rubix.dashboard.overview.title}}"
}
```

Keys ship in the rubix message bundle in the **same commit** as
the bundled JSON. The skill prompt instructs the LLM that
user-visible strings authored in `page_set` MUST also be
MessageKeys; literal English is a contract violation. The codeless
job's MessageBundle covers `en` + `es` at minimum.

## Prompt contract (what SKILL.md tells the LLM)

Already in the existing
[`SKILL.md`](../../../crates/rubix-skills/skills/dashboard-builder/SKILL.md)
— "ask before spawning a 4-panel dashboard for a single-metric
ask", "don't invent widget types". Append:

1. **Always `list` and `get` before authoring.** Reuse beats
   re-create. The AI cites the page it duplicated in its summary.
2. **Pick the closest bundled template first.** SKILL.md ships a
   `## Bundled templates` index (table below) so the LLM can
   choose without calling `list` for trivial requests. If no
   template matches, duplicate `dashboard.blank`.
3. **Output one `body_json` per turn.** Don't speculate about
   layouts in prose; emit a `page_set` call and let the resolver
   validate.
4. **Bindings, not data dumps.** Charts read `{{$target/...}}`
   from the live graph — never paste sample data into the page.
5. **User-visible strings are MessageKeys.** Every `text.content`,
   `card.title`, `action.label` etc. is `{{$msg.<key>}}` — never
   a literal English string. New keys go in the rubix
   `MessageBundle` in the same `page_set` turn.
6. **One responsibility per page.** "Disk usage" and "user
   admin" are two pages, not one — link between them with
   `NavigateTo` actions.
7. **Render-side validation runs at `page_set`.** A reject means
   the LLM has misread the schema; show the diagnostic verbatim
   (including its JSON-pointer `locator`) in its self-correction
   turn.

### Bundled template index (added to SKILL.md verbatim)

```
| page_id                  | When to duplicate                         |
|--------------------------|-------------------------------------------|
| dashboard.single-kpi     | "show me one number / current value"      |
| dashboard.overview       | "a home page", "a summary"                |
| dashboard.disk-by-host   | per-entity grid of the same KPI/widget    |
| dashboard.flow-health    | tabular operational status                |
| dashboard.host           | per-entity detail page (use $target)      |
| dashboard.alarms         | row actions, acknowledge/mute style work  |
| dashboard.team-usage     | counts + activity feed                    |
| dashboard.index          | a list-of-things picker (rare)            |
| dashboard.blank          | nothing above fits                        |
```

The LLM **must** read at least one template via
`rubix.dashboard.get` before its first `page_set`. The skill
post-condition check rejects a `page_set` whose conversation has
zero prior `get` calls.

## Diagnostic shape on `page_set` validation failure

When `page_set` rejects the LLM's draft, the response carries a
structured `Diagnostic` the LLM can self-correct against — not a
free-form error string:

```json
{
  "key": "rubix.dashboard.validation_failed",
  "params": { "reason": "unknown variant `kpi_grid`, expected one of [kpi, grid, ...]" },
  "locator": "/root/children/2/children/0"
}
```

The `locator` is a **JSON Pointer (RFC 6901)** into the rejected
`body_json` tree, computed by the schema validator. SKILL.md
tells the LLM: "on a validation failure, walk to `locator` in
your previous draft, fix only that subtree, and re-emit
`page_set`." This bounds the self-correction blast radius.

The `Diagnostic` shape is shared with `starter-ui-bindings`
`BindingError` (already returns `{key, params}`); adding the
optional `locator: Option<String>` field is the only delta on
the wire type — ~5 LOC in `starter-sdui-routes/src/types.rs`.

## What changes in the existing assistant scaffolding

[`rubix/crates/rubix-tools/src/dashboard/assistant.rs`](../../../crates/rubix-tools/src/dashboard/assistant.rs)
currently exists as a placeholder. After this scope, it becomes a
single ≤ 80-line dispatch shim:

```text
1. Take an operator prompt (string).
2. Build an ai-agent runtime context with the dashboard-assistant
   flow id.
3. Invoke the flow; collect the final reply text.
4. Return { reply, page_id_changed?, diagnostics[] }.
```

The actual work — read schema, draft tree, call `page_set` —
happens inside the `ai-agent` node's runner loop, exactly like
the other rubix goals (R8).

## Tests in the same diff

A new integration test
`rubix/crates/rubix-agent/tests/goal_1_dashboard_assistant_test.rs`
that uses the `FixtureRunner` (deterministic
`AiRunner`) so the test doesn't depend on a live Claude API key:

1. Fixture turn 1: ask "show me disk usage by host".
2. Fixture turn 2: the runner is wired to emit a `tool_call` to
   `dashboard.duplicate` against `dashboard.disk-by-host`.
3. Fixture turn 3: the runner emits a `page_set` with a tiny
   edit (title change).
4. Assertion: a new page exists with the edited title, the
   bundled source is untouched, and undo restores the prior body.

This proves the *plumbing* works without committing to a specific
LLM behaviour.

## Acceptance

1. `make demo` boots; an operator can call
   `rubix.dashboard.assistant.list_and_clone` (or the equivalent
   MCP tool via `FlowAsTool`) and get a working dashboard
   modelled on a bundled template.
2. The nine bundled pages render through the SDUI route without
   binding errors against an empty rubix entity graph (the graph
   impl returns `None` for unknown slots; `BindingError` produces
   inline `Diagnostic`s the renderer surfaces but doesn't crash on).
3. The `goal_1_dashboard_assistant_test.rs` integration passes
   against the fixture runner.
4. SKILL.md is updated with the seven additional rules and the
   bundled-template index table.
5. The nine bundled JSON files exist under
   `rubix/crates/rubix-flows/dashboards/` and validate against the
   `ComponentTree` JSON schema at build time (a `build.rs`
   compile-time test, ≤ 30 LOC).
6. No bundled JSON file contains a literal English user-visible
   string — a `build.rs` grep asserts every `text.content`,
   `card.title`, `action.label` either starts with `{{$msg.` or
   is empty.
7. A `page_set` call with a deliberately broken `body_json`
   returns a `Diagnostic` with a non-empty JSON-pointer
   `locator`, and the integration test verifies the second turn
   uses that pointer to patch only the targeted subtree.
8. Each pruned schema in
   `rubix/crates/rubix-skills/skills/dashboard-builder/schemas/`
   is ≤ 3000 cl100k tokens (measured in CI by a tiny `xtask`).
