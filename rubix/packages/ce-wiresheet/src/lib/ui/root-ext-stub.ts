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

/** Stub "Alarms" extension — one `layout` UI, `selection: "ignore"` (a surface
 *  independent of node selection). Proves the outer extension-tab level + the
 *  `ignore` selection mode. */
const ALARMS_HOME: UiEntry = {
  id: "alarms-home",
  label: "Alarms",
  icon: "layout",
  selection: "ignore",
  view: {
    type: "layout",
    children: [
      { type: "text", text: "Alarms (stub extension) — ignores selection." },
      {
        type: "button",
        label: "Generate alarm",
        action: { name: "generate", label: "Generate alarm", target: "collection" },
      },
    ],
  },
};

export const EXTENSIONS_STUB: ExtensionUi[] = [
  { id: "ce", label: "Control Engine", icon: "cpu", uis: [CE_TABLE, CE_DEMO] },
  { id: "alarms", label: "Alarms (stub)", icon: "bell", uis: [ALARMS_HOME] },
];

/**
 * Resolve the loaded extensions and their UIs. Stubbed today; replace with a
 * fetch (per loaded extension's `GET /api/v0/ui/list`) when the engine ships it.
 */
export async function getExtensions(): Promise<ExtensionUi[]> {
  return EXTENSIONS_STUB;
}
