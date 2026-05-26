// Harness entry — boots a single <PuckBuilder> with a small
// hand-authored ComponentTree so reviewers can drag a widget from
// the palette onto the canvas. No save, no SSE, no auth — purely a
// visual smoke test of the generated config.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@measured/puck/puck.css";

import { PuckBuilder } from "../src/builder.js";

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
    />
  </StrictMode>,
);
