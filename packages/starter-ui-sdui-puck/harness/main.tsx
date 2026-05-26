// Harness entry — boots a single <PuckBuilder> with a small
// hand-authored ComponentTree so reviewers can drag a widget from
// the palette onto the canvas. No save, no SSE, no auth — purely a
// visual smoke test of the generated config.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@measured/puck/puck.css";

import { PuckBuilder } from "../src/builder.js";
import {
  catalogueFromMap,
  type CatalogueEntry,
} from "../src/data-source-field.js";

// Mock catalogue so the §B3 pickers work without a running rubix
// agent. Each kind returns a hand-rolled list — the real frontend
// wires these to the corresponding rubix verbs (analytics template
// list, /api/v1/tools, rubix.tenant.list, etc).
const mockEntry = (
  value: string,
  label: string,
  hint?: string,
): CatalogueEntry => ({ value, label, hint });

const harnessCatalogue = catalogueFromMap({
  analytics_template: async () => [
    mockEntry("meter_value_24h_1m", "Meter value · 24h (1-min)"),
    mockEntry("kwh_per_day_30d", "kWh per day · 30d"),
  ],
  tool: async () => [
    mockEntry("rubix.dashboard.update", "rubix.dashboard.update"),
    mockEntry("rubix.alert.send", "rubix.alert.send"),
  ],
  tenant: async () => [mockEntry("system", "system")],
  unit_symbol: async () => [
    mockEntry("kWh", "kWh — kilowatt hours"),
    mockEntry("L", "L — litres"),
    mockEntry("°C", "°C — degrees Celsius"),
    mockEntry("%", "% — percent"),
  ],
  page_state_key: async () => [
    mockEntry("$page.range_from", "$page.range_from"),
    mockEntry("$page.range_to", "$page.range_to"),
  ],
});

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("harness: #root missing");

// Puck `Data` shape — `content` is the top-level array of dropped
// components. Matches what the generator emits Puck keys for
// (snake-case IR `type` strings).
const initialData = {
  root: { props: { title: "Harness page" } },
  content: [
    {
      type: "heading",
      props: { id: "hdg-1", content: "Hello from PuckBuilder", level: 2 },
    },
    {
      type: "row",
      props: { id: "row-1", children: [] },
    },
    {
      type: "kpi",
      props: { id: "kpi-1", label: "Demo KPI", value: 123.4, unit_symbol: "kWh" },
    },
    {
      type: "chart",
      props: { id: "chart-1", title: "Demo chart" },
    },
  ],
} as const;

createRoot(rootEl).render(
  <StrictMode>
    <PuckBuilder
      pageRef="dashboard.harness"
      initialData={initialData as unknown as Parameters<typeof PuckBuilder>[0]["initialData"]}
      catalogue={harnessCatalogue}
    />
  </StrictMode>,
);
