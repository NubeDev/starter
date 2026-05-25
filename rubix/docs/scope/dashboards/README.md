# Dashboards — scope (Goal 1, SDUI + AI builder)

> **Tier:** scope (plan). Lifetime: weeks–months. Per
> [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md) **source code must
> not reference any file in this folder.** When a section lands,
> promote the present-tense parts into `docs/design/sdui/` and
> update code links to point there.

## What this scope is

The plan to deliver **rubix Goal 1 — build / program dashboards
via SDUI**, end to end. Operator-authored *and* AI-authored
dashboards via the same backend path, renderable on web, mobile,
and Tauri without a per-platform component library.

Goal 1 is one of two SCOPE goals still stubbed today
(`docs/scope/THIN-SLICE.md` — "Goals lit up beyond the thin
slice" table). The infrastructure exists across `starter-*`
crates; the connecting work and four substrate gaps are listed
below.

## What we already have (no design needed)

Verified by inspection on the workspace as of this scope draft:

| Substrate | Status | Source |
|---|---|---|
| Component IR (v5) — `page`, `grid`, `kpi`, `chart`, `table`, `form`, `tabs`, `select`, `slider`, `toggle`, `date_range`, `divider`, `custom`, `Repeat` | ✅ shipping | [`crates/starter-ui-ir/`](../../../../crates/starter-ui-ir/) |
| Binding grammar — `$target $self $stack.<alias> $user $page`, `.slot` `/child` operators | ✅ shipping | [`crates/starter-ui-bindings/src/parse.rs`](../../../../crates/starter-ui-bindings/src/parse.rs) |
| Length-prefixed evaluator + `SubscriptionPlan` derivation | ✅ shipping | [`crates/starter-ui-bindings/src/eval.rs`](../../../../crates/starter-ui-bindings/src/eval.rs), [`subscription.rs`](../../../../crates/starter-ui-bindings/src/subscription.rs) |
| `EntityGraph` trait for the host's node tree | ✅ shipping | [`crates/starter-ui-bindings/src/graph.rs`](../../../../crates/starter-ui-bindings/src/graph.rs) |
| Typed Rust builder DSL (`dashboard(...)`, `kpi`, `table`, `line_chart`, `seed_page`) | ✅ shipping | [`crates/starter-ui-builder/`](../../../../crates/starter-ui-builder/) |
| HTTP surface — `POST /api/v1/ui/resolve`, `/action`, `GET /table` with `PageProvider` / `QueryEngine` / `HandlerRegistry` traits, capability handshake, R8 DoS caps | ✅ shipping | [`crates/starter-sdui-routes/`](../../../../crates/starter-sdui-routes/) |
| Theme tokens with migrations | ✅ shipping | [`crates/starter-ui-theme/`](../../../../crates/starter-ui-theme/) |
| Tag substrate (for "show me dashboards tagged X") | ✅ shipping | [`crates/starter-tags/`](../../../../crates/starter-tags/) |
| Skill file for the AI builder | ✅ shipping | [`rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md`](../../../crates/rubix-skills/skills/dashboard-builder/SKILL.md) |
| Six `rubix.dashboard.*` tool **stubs** (no body) | ⚠️ stubs only | [`rubix/crates/rubix-tools/src/dashboard/`](../../../crates/rubix-tools/src/dashboard/) |
| Hand-coded `dashboard.tsx` with `SPARK_DEVICES` etc. (placeholder, not SDUI-rendered) | ⚠️ placeholder | [`rubix/frontend/src/routes/dashboard.tsx`](../../../frontend/src/routes/dashboard.tsx) |

## What we have to build

Eight scope files, each ≤ ~250 lines, each one verb of the work:

| # | File | One sentence |
|---|---|---|
| 1 | [01-storage.md](./01-storage.md) | Where dashboard page bodies live in PG, with revisions, NOTIFY, and authz registration. |
| 2 | [02-bindings-gaps.md](./02-bindings-gaps.md) | Six substrate fixes in `starter-ui-bindings` / `starter-ui-ir` (per-variant Bindable, qualifiers, Repeat, synthetic ids, portable subset flag, `$msg` source). |
| 3 | [03-host-glue.md](./03-host-glue.md) | Rubix implements `EntityGraph` / `PageProvider` / `QueryEngine` / `HandlerRegistry` and wires `sdui_router` into the agent. |
| 4 | [04-tools.md](./04-tools.md) | Fill the six `rubix.dashboard.*` tool bodies — create, update, delete, list, page_set, duplicate, history. |
| 5 | [05-frontend-renderer.md](./05-frontend-renderer.md) | `@nube/starter-ui-sdui-react` package + `<SduiPage page_ref target_ref />`, subscription wiring, action dispatch. |
| 6 | [06-ai-builder.md](./06-ai-builder.md) | Goal-1 flow `com.rubix.dashboard-assistant` + the JSON dialect the LLM emits via tool args. |
| 7 | [07-fetch-plan.md](./07-fetch-plan.md) | Phase-4 batched historical pulls — deferred but scoped so the IR doesn't drift. |
| 8 | [08-open-questions.md](./08-open-questions.md) | Decisions we haven't made yet, with the default if no one answers. |

## Dependency order

```
01-storage  ──┐
              ├──►  03-host-glue  ──►  04-tools  ──►  06-ai-builder
02-bindings ──┘                                          │
                                                         │
                                          05-frontend ◄──┘
                                                         │
                                            07-fetch-plan (deferred)
```

You can build 01 and 02 in parallel; 03 depends on both; 04 lights
up MCP / REST; 05 is independent of 04 until the demo step; 06
requires 04 + 05; 07 is a v2 milestone.

## Non-goals (in this scope)

- **No new agent runtime.** Goal 1 is a flow rooted at an
  `ai-agent` node, like every other rubix goal (R8, see
  `docs/design/agent/README.md`).
- **No second IR / second binding grammar.** Everything renders
  via [`starter-ui-ir`](../../../../crates/starter-ui-ir/). No
  bespoke JSX, no hand-rolled widget code in dashboard pages.
- **No client-side template language.** The frontend renders a
  resolved tree; bindings live and die on the server.
- **No per-tenant page partitioning** in v1. Page rows carry a
  `tenant_id`; the listing query filters on the caller's
  principal. Multi-tenant migration to schema-per-tenant is a
  later concern.
- **No widget marketplace** in v1. Extensions can contribute
  `Component::Custom` renderer ids (already supported by the IR
  + capability handshake), but the AI builder limits itself to
  the built-in catalogue until Phase 3.

## How this maps to SCOPE.md

- Goal 1 — `dashboard.{create,update,list,page_set}` tools and
  the `com.rubix.dashboard-assistant` flow.
- R7 — Every dashboard tool auto-surfaces as an MCP tool via
  `FlowAsTool::from_registry` (no wiring code, see
  `docs/design/agent/README.md`).
- R12 — MCP resource URIs (`rubix://dashboard/pages`) are
  distinct from SDUI page ids (`dashboard.<slug>`). Already
  noted in [`docs/design/sdui/README.md`](../../design/sdui/README.md).

## How to use this scope in an AI session

Each numbered file is sized to be one codeless job:

1. Read `NEW-SESSION.md` + `HOW-TO-CODE.md` + this README.
2. Pick the next undelivered file in the dependency order above.
3. Read **only that file** plus any `docs/design/` it cites.
4. Implement; produce tests in the same diff.
5. When the file's work ships, promote the present-tense parts
   into `docs/design/sdui/<area>/README.md` and delete the scope
   file (or leave a one-line back-reference).

Source comments **never** link back here — only to the design
docs that will exist after the work lands.

## A note on cited line numbers

The scope files cite approximate line numbers into `starter-*`
crates (e.g. *"`Page.default_row_gap` at
[`component.rs:259`](../../../../crates/starter-ui-ir/src/component.rs)"*).
These are anchors for the codeless implementer to *find the right
symbol fast* — they were captured during scope drafting and the
files will keep evolving. **The implementer must re-grep for the
symbol** before quoting any line number in a doc-comment or PR
description, and treat the citation as "this exists in that
file" not "this is at exactly that line."
