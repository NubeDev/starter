// Stub of the loaded extensions' UI contributions — stands in for the engine /
// host serving each extension's `GET /api/v0/ui/list` until that lands (see
// ../../SDUI_UNIFIED_DESIGN.md §10, ../../API_REQUESTS.md §5). Two nav levels:
//
//   right-edge tab  = which EXTENSION   (ExtensionUi)
//   inner side-strip = that ext's UIs   (UiEntry[])
//
// The renderer + tab host build against `getExtensions()`; swap the body for a
// real fetch (one list per loaded extension) when the API lands.

import type { ExtensionUi, UiEntry } from "./types";

// --- Control Engine (root) extension UIs ------------------------------------

/** The root ext's default: a full-bleed `collection` of the folder's components,
 *  `selection: "sync"` (canvas ⇄ row). Backed by the built-in table widget;
 *  columns derive from field descriptors (`__facets` + `/schema`). */
const CE_TABLE: UiEntry = {
  id: "components-table",
  label: "Table",
  icon: "table",
  selection: "sync",
  view: { type: "collection", source: "components", fullBleed: true, multiselect: true },
};

/** A `layout` page, `selection: "follow"` — binds live values of the selected
 *  component + an action button. Proves value binding + dispatch in a layout. */
const CE_DEMO: UiEntry = {
  id: "demo-panel",
  label: "Demo",
  icon: "layout",
  selection: "follow",
  view: {
    type: "layout",
    children: [
      { type: "text", text: "Demo panel — follows the selected component." },
      { type: "value", label: "Input 1", bind: { prop: "in1" } },
      { type: "value", label: "Input 2", bind: { prop: "in2" } },
      { type: "value", label: "Output", bind: { prop: "out" } },
      {
        type: "button",
        label: "Log selected",
        action: { name: "ping", label: "Log selected", target: "component" },
      },
    ],
  },
};

// --- A second stub extension, to make the right-edge (outer) tabs real -------

/** "Alarms" extension — a live console over the singleton `alarm.service`. A
 *  standalone surface (`ignore`): the widget auto-resolves the one alarm service,
 *  so there's no node selection. */
const ALARMS_HOME: UiEntry = {
  id: "alarms-home",
  label: "Alarms",
  icon: "bell",
  selection: "ignore",
  view: { type: "layout", children: [{ type: "alarms" }] },
};
const ALARMS_HISTORY: UiEntry = {
  id: "alarms-history",
  label: "History",
  icon: "tree",
  selection: "ignore",
  view: { type: "layout", children: [{ type: "alarmHistory" }] },
};

/** Structural overview: the folder hierarchy as a tree with per-folder counts.
 *  `selection: "sync"` so it tracks the canvas; double-click drills in. */
const CE_TREE: UiEntry = {
  id: "components-tree",
  label: "Tree",
  icon: "tree",
  selection: "sync",
  view: { type: "tree", source: "components", fullBleed: true },
};

/** Stub "Scheduler" extension — now three component types: schedule (weekly
 *  calendar grid), timer, and cron. Each is a `follow` UI bound to its type;
 *  when nothing matching is selected the host shows a 3-column picker (one per
 *  type). Timer/cron editors are placeholders until their manifests land. */
const SCHEDULE_UI: UiEntry = {
  id: "schedule",
  label: "Schedule",
  icon: "calendar",
  selection: "follow",
  appliesTo: "schedule",
  view: {
    type: "layout",
    children: [{ type: "schedule", bind: { prop: "config" }, action: { name: "setSchedule", label: "Save", target: "component" } }],
  },
};
const TIMER_UI: UiEntry = {
  id: "timer",
  label: "Timer",
  icon: "timer",
  selection: "follow",
  appliesTo: "timer",
  view: { type: "layout", children: [{ type: "timer" }] },
};
const CRON_UI: UiEntry = {
  id: "cron",
  label: "Cron",
  icon: "cron",
  selection: "follow",
  appliesTo: "cron",
  view: { type: "layout", children: [{ type: "cron", bind: { prop: "cron" }, action: { name: "setCron", label: "Set", target: "component" } }] },
};

export const EXTENSIONS_STUB: ExtensionUi[] = [
  { id: "ce", label: "Control Engine", icon: "cpu", uis: [CE_TABLE, CE_TREE, CE_DEMO] },
  { id: "scheduler", label: "Scheduler (stub)", icon: "calendar", uis: [SCHEDULE_UI, TIMER_UI, CRON_UI] },
  { id: "alarms", label: "Alarms (stub)", icon: "bell", uis: [ALARMS_HOME, ALARMS_HISTORY] },
];

/**
 * Resolve the loaded extensions and their UIs. Stubbed today; replace with a
 * fetch (per loaded extension's `GET /api/v0/ui/list`) when the engine ships it.
 */
export async function getExtensions(): Promise<ExtensionUi[]> {
  return EXTENSIONS_STUB;
}
