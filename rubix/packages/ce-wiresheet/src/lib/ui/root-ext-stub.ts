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
// Each scheduler kind is its own tabbed editor: a pinned "All" index of that
// kind + a tab per opened instance. The sidebar (UiTabHost strip) switches
// between the three kinds. `fullType` powers the global index; in-folder
// instances show even if it's wrong. TODO: verify the timer/cron full types
// against the schedule extension manifest (only `schedule` instances exist on
// the test engine to confirm against).
const SCHEDULE_UI: UiEntry = {
  id: "schedule",
  label: "Schedule",
  icon: "calendar",
  selection: "ignore",
  view: {
    type: "layout",
    children: [{
      type: "tabbedEditor",
      fullType: "NubeIO-schedule::schedule",
      indexLabel: "schedules",
      inner: { type: "schedule", bind: { prop: "config" }, action: { name: "setSchedule", label: "Save", target: "component" } },
    }],
  },
};
const TIMER_UI: UiEntry = {
  id: "timer",
  label: "Timer",
  icon: "timer",
  selection: "ignore",
  view: {
    type: "layout",
    children: [{
      type: "tabbedEditor",
      fullType: "NubeIO-schedule::timer",
      indexLabel: "timers",
      inner: { type: "timer" },
    }],
  },
};
const CRON_UI: UiEntry = {
  id: "cron",
  label: "Cron",
  icon: "cron",
  selection: "ignore",
  view: {
    type: "layout",
    children: [{
      type: "tabbedEditor",
      fullType: "NubeIO-schedule::cron",
      indexLabel: "crons",
      inner: { type: "cron", bind: { prop: "expr" }, action: { name: "setCron", label: "Set", target: "component" } },
    }],
  },
};

/** "JS" extension (NubeIO-js manifest). Submenu = three tab-open UIs over the
 *  singleton `jsScriptStore`:
 *    - Components (`jsComponents`): jsLogic instances — assign a script, edit its
 *      source inline, live `log`, Save & Assign (setScript). Shows how many
 *      components share the open script.
 *    - Scripts (`jsScripts`): the script library — edit source directly, create.
 *    - Examples (`jsExamples`): read-only example library (getExamples).
 *  All action/field names come from this shared config, so the served /ui/list
 *  entries can replace it verbatim. */
const JS_CFG = {
  fullType: "NubeIO-js::jsLogic",
  serviceType: "jsScriptStore",
  loadAction: "getScript",
  sourceKey: "source",
  scriptIdProp: "scriptId",
  scriptIdParam: "scriptId",
  scriptIdSetAction: "setScript",
  availableScriptsProp: "availableScripts",
  listAction: "listScripts",
  apiAction: "getApi",
  exampleAction: "getExamples",
  bind: { prop: "log" },
  action: { name: "putScript", label: "Save", target: "component" as const },
};
const JS_COMPONENTS_UI: UiEntry = {
  id: "js-components", label: "Components", icon: "boxes", selection: "ignore",
  view: { type: "layout", children: [{ type: "jsComponents", ...JS_CFG }] },
};
const JS_SCRIPTS_UI: UiEntry = {
  id: "js-scripts", label: "Scripts", icon: "code", selection: "ignore",
  view: { type: "layout", children: [{ type: "jsScripts", ...JS_CFG }] },
};
const JS_EXAMPLES_UI: UiEntry = {
  id: "js-examples", label: "Examples", icon: "book", selection: "ignore",
  view: { type: "layout", children: [{ type: "jsExamples", ...JS_CFG }] },
};

export const EXTENSIONS_STUB: ExtensionUi[] = [
  { id: "ce", label: "Control Engine", icon: "cpu", uis: [CE_TABLE, CE_TREE, CE_DEMO] },
  { id: "scheduler", label: "Scheduler", icon: "calendar", uis: [SCHEDULE_UI, TIMER_UI, CRON_UI] },
  { id: "alarms", label: "Alarms", icon: "bell", uis: [ALARMS_HOME, ALARMS_HISTORY] },
  { id: "js", label: "JavaScript", icon: "code", uis: [JS_COMPONENTS_UI, JS_SCRIPTS_UI, JS_EXAMPLES_UI] },
];

/**
 * Resolve the loaded extensions and their UIs. Stubbed today; replace with a
 * fetch (per loaded extension's `GET /api/v0/ui/list`) when the engine ships it.
 */
export async function getExtensions(): Promise<ExtensionUi[]> {
  return EXTENSIONS_STUB;
}
