// `wiresheet/wiresheet-panel.tsx` — remote-programming canvas for one
// control engine.
//
// SCAFFOLD ONLY — no flow logic. Renders an empty `<FlowCanvas />`
// with the kind registry and a toolbar. Wiring it to the engine
// (`engine_wiresheet_get` / `engine_wiresheet_put`, graph mapping,
// dirty-tracking, save) is left as TODO.

import * as React from "react";

import { ArrowLeft, RefreshCw, Save } from "lucide-react";

import { FlowCanvas } from "@nube/starter-ui-flow/canvas";
import type { FlowGraph } from "@nube/starter-ui-flow";

import { EXTENSION_ID } from "../types";
import { buildRegistry } from "./kinds";

const EMPTY: FlowGraph = { nodes: [], edges: [] };

export function WiresheetPanel({ deviceId }: { deviceId: string }): React.ReactElement {
  const registry = React.useMemo(() => buildRegistry(), []);

  // TODO: load the engine's program via `getWiresheet(deviceId)` and
  // render it; wire `onChange` + a save via `putWiresheet`.

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <a
          href={`/extensions/${EXTENSION_ID}/devices`}
          className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="size-4" /> Devices
        </a>
        <div className="flex items-center gap-2">
          <span className="ext-eyebrow">device: {deviceId || "—"}</span>
          <button
            type="button"
            className="inline-flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm"
          >
            <RefreshCw className="size-4" /> Reload
          </button>
          <button
            type="button"
            className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground"
          >
            <Save className="size-4" /> Save to engine
          </button>
        </div>
      </div>

      <div className="ext-wiresheet">
        <FlowCanvas registry={registry} graph={EMPTY} showMiniMap showControls showBackground />
      </div>
    </div>
  );
}
