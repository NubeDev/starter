// Scripted `BuilderAdapter` for the flow-agent Page Builder demo.
//
// Per SCOPE R3 / D2 this file is deliberately deterministic: a fixed
// `createFixtureBuilderAdapter` (from `@nube/starter-ui-ai-builder`)
// is wired with five prefix-keyed scripts (`sales`, `dashboard`,
// `onboard`, `report`, and a fallback `default`) and a pinned uniform
// `delayMs`.
//
// ── Why uniform delayMs ─────────────────────────────────────────────
// The reference fixture adapter ships with a single `delayMs` between
// yields. D2's pinned wall-clock numbers (`t=0/50/60/80 ms`) are
// illustrative — what is load-bearing is the *ordering* invariant:
//
//   1. one `patch` event MUST be emitted before its parent
//      `full-render`, so the R1 buffer counts at least one held
//      patch ("buffered" badge visible);
//   2. a second `patch` follows, also pre-parent, so the count
//      visibly reaches 2 before draining;
//   3. the parent `full-render` lands well inside `useBuilder`'s
//      default 2000ms buffer window so the buffer drains cleanly
//      (no R1 "stale patch dropped" warning);
//   4. the whole turn (status → tree → status done) finishes inside
//      the 2s acceptance budget.
//
// With `DELAY_MS = 80`, the sales script lands events at
// t = 80, 160, 240, 320, 400, 480, 560, 640, 720, 800, 880 ms.
// Two pre-parent patches at t=160/240 are buffered until the parent
// `full-render` at t=320, then drained — well within 2000ms — and
// `status: done` lands at t≈880ms, comfortably inside the 2s budget.
// The exact 50/60/80ms numbers in D2 cannot be reproduced verbatim
// without per-event timings; the invariants above are what the
// acceptance check pins.

import {
  createFixtureBuilderAdapter,
  fixtureTree,
} from "@nube/starter-ui-ai-builder";
import type {
  BuilderAdapter,
  BuilderEvent,
} from "@nube/starter-ui-ai-builder";
import type { UiComponent } from "@nube/starter-sdui-react";

/** Pinned per SCOPE R3 / D2 — see file header for the invariants. */
export const FIXTURE_DELAY_MS = 80;

// ── Tree builders ──────────────────────────────────────────────────
//
// Each fixture emits:
//   1. a `full-render` *skeleton* whose `children` are placeholders
//      with stable IDs (`root.children.<n>`),
//   2. zero or more `patch` events that fill those placeholders.
//
// The buffered-patch demo emits two `patch`es BEFORE the parent
// `full-render` so the R1 buffer holds them, then the skeleton lands
// and drains the buffer in one tick. IDs match across pre-parent
// patches and the skeleton so `applyPatchIfPossible` resolves them
// on the flush pass.

function placeholder(id: string, label: string): UiComponent {
  return {
    type: "container",
    id,
    style: { className: "rounded-lg border border-border/60 p-4" },
    children: [{ type: "text", text: label }],
  };
}

function kpiCard(id: string, label: string, value: string): UiComponent {
  return {
    type: "container",
    id,
    style: {
      className:
        "rounded-xl border border-border/60 bg-card p-4 shadow-sm flex flex-col gap-1",
    },
    children: [
      {
        type: "text",
        text: label,
        style: { className: "text-xs uppercase text-muted-foreground" },
      },
      {
        type: "text",
        text: value,
        style: { className: "text-2xl font-semibold" },
      },
    ],
  };
}

function salesSkeleton(): UiComponent {
  return {
    type: "container",
    id: "root",
    style: { className: "flex flex-col gap-6 p-6" },
    children: [
      {
        type: "container",
        id: "root.title",
        children: [
          {
            type: "text",
            text: "Sales · Q2",
            style: { className: "text-xl font-semibold" },
          },
        ],
      },
      placeholder("root.kpis", "Loading KPIs…"),
      placeholder("root.pipeline", "Loading pipeline…"),
    ],
  };
}

function salesKpis(): UiComponent {
  return {
    type: "container",
    id: "root.kpis",
    style: { className: "grid grid-cols-4 gap-3" },
    children: [
      kpiCard("root.kpis.mrr", "MRR", "$42k"),
      kpiCard("root.kpis.arr", "ARR", "$508k"),
      kpiCard("root.kpis.win", "Win rate", "31%"),
      kpiCard("root.kpis.nps", "NPS", "62"),
    ],
  };
}

function salesPipeline(): UiComponent {
  return {
    type: "container",
    id: "root.pipeline",
    style: {
      className:
        "rounded-xl border border-border/60 bg-card p-4 shadow-sm flex flex-col gap-2",
    },
    children: [
      {
        type: "text",
        text: "Pipeline",
        style: { className: "text-sm font-medium" },
      },
      {
        type: "table",
        id: "root.pipeline.table",
        columns: [
          { key: "stage", label: "Stage" },
          { key: "deals", label: "Deals" },
          { key: "value", label: "Value" },
        ],
        rows: [
          { stage: "Qualified", deals: 12, value: "$84k" },
          { stage: "Demo", deals: 8, value: "$112k" },
          { stage: "Close", deals: 3, value: "$96k" },
        ],
      } as UiComponent,
    ],
  };
}

function onboardSkeleton(): UiComponent {
  return {
    type: "container",
    id: "root",
    style: { className: "flex flex-col gap-6 p-6" },
    children: [
      {
        type: "text",
        text: "Onboarding",
        style: { className: "text-xl font-semibold" },
      },
      placeholder("root.form", "Loading form…"),
      placeholder("root.checklist", "Loading checklist…"),
      placeholder("root.tabs", "Loading tabs…"),
    ],
  };
}

function reportSkeleton(): UiComponent {
  return {
    type: "container",
    id: "root",
    style: { className: "flex flex-col gap-6 p-6" },
    children: [
      {
        type: "text",
        text: "Daily report",
        style: { className: "text-xl font-semibold" },
      },
      placeholder("root.summary", "Loading summary…"),
      placeholder("root.chart", "Loading chart…"),
      placeholder("root.table", "Loading table…"),
    ],
  };
}

function helloTree(): UiComponent {
  return {
    type: "container",
    id: "root",
    style: { className: "p-6" },
    children: [
      {
        type: "container",
        id: "root.card",
        style: {
          className:
            "rounded-xl border border-border/60 bg-card p-6 shadow-sm",
        },
        children: [
          {
            type: "text",
            text: "Hello — describe a dashboard to get started.",
            style: { className: "text-base" },
          },
          {
            type: "text",
            text: "Try: sales · dashboard · onboard · report",
            style: { className: "text-sm text-muted-foreground mt-2" },
          },
        ],
      },
    ],
  };
}

// ── Scripts ────────────────────────────────────────────────────────
//
// `sales` and `dashboard` share the buffered-patch demo script (D2);
// `onboard` / `report` / `default` use the same opening beat
// (status → 2 pre-parent patches → full-render) so the R1 badge demo
// is reproducible regardless of prompt prefix, then diverge in the
// section payloads they fill.

function salesScript(): BuilderEvent[] {
  return [
    { type: "status", phase: "thinking" },
    // Two patches BEFORE the parent `full-render` (R3 / D2):
    {
      type: "patch",
      targetComponentId: "root.kpis",
      subtree: salesKpis(),
    },
    {
      type: "patch",
      targetComponentId: "root.pipeline",
      subtree: salesPipeline(),
    },
    // Parent skeleton — drains both buffered patches in one tick.
    { type: "full-render", tree: fixtureTree(salesSkeleton()) },
    { type: "status", phase: "writing", message: "Writing layout…" },
    // Refinement patches, streamed normally (post-parent → applied
    // immediately). Re-emit the same nodes to demonstrate streamed
    // refinement; identical payload keeps the snapshot stable.
    {
      type: "patch",
      targetComponentId: "root.kpis",
      subtree: salesKpis(),
    },
    {
      type: "patch",
      targetComponentId: "root.pipeline",
      subtree: salesPipeline(),
    },
    { type: "status", phase: "done", message: "Done" },
  ];
}

function onboardScript(): BuilderEvent[] {
  const form: UiComponent = {
    type: "container",
    id: "root.form",
    style: { className: "rounded-xl border border-border/60 p-4 shadow-sm" },
    children: [
      {
        type: "text",
        text: "Account details",
        style: { className: "text-sm font-medium mb-2" },
      },
      { type: "text", text: "Name · Email · Company" },
    ],
  };
  const checklist: UiComponent = {
    type: "container",
    id: "root.checklist",
    style: { className: "rounded-xl border border-border/60 p-4 shadow-sm" },
    children: [
      { type: "text", text: "✓ Create account" },
      { type: "text", text: "✓ Verify email" },
      { type: "text", text: "○ Invite teammates" },
      { type: "text", text: "○ Connect a data source" },
    ],
  };
  const tabs: UiComponent = {
    type: "container",
    id: "root.tabs",
    style: { className: "rounded-xl border border-border/60 p-4 shadow-sm" },
    children: [
      { type: "text", text: "[ Profile ] [ Team ] [ Billing ]" },
    ],
  };
  return [
    { type: "status", phase: "thinking" },
    { type: "patch", targetComponentId: "root.form", subtree: form },
    {
      type: "patch",
      targetComponentId: "root.checklist",
      subtree: checklist,
    },
    { type: "full-render", tree: fixtureTree(onboardSkeleton()) },
    { type: "status", phase: "writing", message: "Writing layout…" },
    { type: "patch", targetComponentId: "root.tabs", subtree: tabs },
    { type: "status", phase: "done", message: "Done" },
  ];
}

function reportScript(): BuilderEvent[] {
  const summary: UiComponent = {
    type: "markdown",
    id: "root.summary",
    text: "**Today:** 1,284 events · 41 errors · 99.97% uptime.",
  };
  const chart: UiComponent = {
    type: "container",
    id: "root.chart",
    style: {
      className:
        "rounded-xl border border-border/60 p-4 shadow-sm h-40 flex items-center justify-center text-muted-foreground",
    },
    children: [{ type: "text", text: "▁▂▄▆█▆▄▃▅▇█ (chart placeholder)" }],
  };
  const table: UiComponent = {
    type: "table",
    id: "root.table",
    columns: [
      { key: "hour", label: "Hour" },
      { key: "events", label: "Events" },
      { key: "errors", label: "Errors" },
    ],
    rows: [
      { hour: "00:00", events: 81, errors: 0 },
      { hour: "06:00", events: 144, errors: 2 },
      { hour: "12:00", events: 412, errors: 12 },
      { hour: "18:00", events: 647, errors: 27 },
    ],
  } as UiComponent;
  return [
    { type: "status", phase: "thinking" },
    { type: "patch", targetComponentId: "root.summary", subtree: summary },
    { type: "patch", targetComponentId: "root.chart", subtree: chart },
    { type: "full-render", tree: fixtureTree(reportSkeleton()) },
    { type: "status", phase: "writing", message: "Writing layout…" },
    { type: "patch", targetComponentId: "root.table", subtree: table },
    { type: "status", phase: "done", message: "Done" },
  ];
}

function defaultScript(): BuilderEvent[] {
  return [
    { type: "status", phase: "thinking" },
    { type: "full-render", tree: fixtureTree(helloTree()) },
    { type: "status", phase: "done", message: "Done" },
  ];
}

/**
 * Build the flow-agent demo `BuilderAdapter`. Prompts are matched by
 * case-insensitive prefix:
 *
 * | Prefix      | Script                                        |
 * | ----------- | --------------------------------------------- |
 * | `sales`     | KPI grid + pipeline table (buffered-patch demo) |
 * | `dashboard` | Same as `sales`                               |
 * | `onboard`   | Form + checklist + tabs                       |
 * | `report`    | Summary + chart + table                       |
 * | _anything_  | Hello card                                    |
 */
export function createFlowAgentBuilderFixture(): BuilderAdapter {
  const sales = salesScript();
  return createFixtureBuilderAdapter({
    delayMs: FIXTURE_DELAY_MS,
    scripts: {
      sales,
      dashboard: sales,
      onboard: onboardScript(),
      report: reportScript(),
      default: defaultScript(),
    },
  });
}
