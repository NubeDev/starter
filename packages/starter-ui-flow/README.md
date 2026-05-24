# @nube/starter-ui-flow

React component library for the [`starter-flow`](../../DOCS/flow/scope/SCOPE.md)
node-graph runtime. Wraps [`@xyflow/react`](https://reactflow.dev) (React
Flow v12) with the shape `starter-flow-spi` uses on the wire: typed
slots, branded ids, a `NodeKindRegistry`, and a `RunOverlay` for
visualising live execution state.

**Zero I/O** (per the starter SCOPE R6 pattern for UI packages). The
host app passes a `FlowGraph` in and receives mutations via
`onChange`. Persistence, transport, and engine wiring are someone
else's job.

## Install

```bash
pnpm add @nube/starter-ui-flow @xyflow/react react react-dom
```

Once, at app entry:

```ts
import "@xyflow/react/dist/style.css";
import "@nube/starter-ui-flow/styles.css";
```

## Quick start

```tsx
import {
  FlowCanvas,
  NodeKindRegistry,
  BUILTIN_NODE_KINDS,
  type FlowGraph,
} from "@nube/starter-ui-flow";

const registry = new NodeKindRegistry().registerAll(BUILTIN_NODE_KINDS);

const graph: FlowGraph = {
  nodes: [
    { id: "t1", kind: "trigger",  position: { x:   0, y: 0 } },
    { id: "a1", kind: "ai-agent", position: { x: 240, y: 0 } },
  ],
  edges: [
    {
      id: "e1",
      source: "t1", sourceSlot: "fire",
      target: "a1", targetSlot: "in",
    },
  ],
};

export function App() {
  return (
    <div style={{ height: 600 }}>
      <FlowCanvas
        registry={registry}
        graph={graph}
        onChange={(g) => console.log("graph", g)}
      />
    </div>
  );
}
```

## What ships

| Surface | Purpose |
| --- | --- |
| `FlowCanvas` | Top-level component. Holds @xyflow/react and wires nodes, edges, typed connections, and the run overlay. |
| `NodeKindRegistry` | TS twin of the backend `NodeKindRegistry`. Hosts assemble it at boot with built-in + extension-contributed kinds. |
| `BUILTIN_NODE_KINDS` | `ai-agent`, `tool-call`, `trigger`, `branch`, `transform`, `subflow` — visuals mirroring the SCOPE. |
| `BaseNode` | Styled node frame. Use it inside custom kind components. |
| `SlotHandle` | Single coloured slot connector + label. |
| `TypedEdge` | Edge coloured by source-slot kind; animates when active. |
| `NodePalette` | Minimal category-grouped palette for inserting nodes. |
| `useFlowGraph` | Bridges `FlowGraph` ↔ @xyflow/react state. Owns change handlers. |
| `useTypedConnect` | `isValidConnection` callback that enforces slot-kind compatibility. |

## Registering a custom node kind

```tsx
import { BaseNode, type NodeKindSpec } from "@nube/starter-ui-flow";
import type { NodeProps } from "@xyflow/react";

const HTTP_OUT_SPEC: NodeKindSpec = {
  kind: "http-out",
  label: "HTTP",
  category: "io",
  color: "#0ea5e9",
  inputs: [
    { name: "url",    kind: "string", required: true },
    { name: "body",   kind: "json" },
  ],
  outputs: [
    { name: "status", kind: "number" },
    { name: "body",   kind: "json" },
  ],
};

function HttpOutNode(props: NodeProps) {
  return <BaseNode spec={HTTP_OUT_SPEC} {...(props.data as any)} selected={props.selected} />;
}

registry.register({ spec: HTTP_OUT_SPEC, component: HttpOutNode });
```

## Live run overlay

```tsx
<FlowCanvas
  registry={registry}
  graph={graph}
  overlay={{
    nodes: { a1: "running", t1: "ok" },
    activeEdges: ["e1"],
  }}
/>
```

Active edges animate; nodes border-shift to match their state.

## Styling

Override these CSS variables to fit the host theme — see
[`src/styles/flow.css`](src/styles/flow.css) for the full list.
Defaults include a light and a dark variant (the latter via
`prefers-color-scheme: dark`).

## Localization

The package ships English defaults and no `react-intl` dependency.
Translate the strings the package owns by passing `i18n` to
`<FlowCanvas>`:

```tsx
import type { FlowMessages } from "@nube/starter-ui-flow";

const i18n: Partial<FlowMessages> = {
  state: { running: t("flow.state.running"), ok: t("flow.state.ok"), /* … */ },
  kindLabels: { "ai-agent": t("flow.kind.ai-agent") /* … */ },
  slotLabels: { "ai-agent.tools": t("flow.slot.ai-agent.tools") },
};

<FlowCanvas registry={registry} graph={graph} i18n={i18n} />
```

Outside `<FlowCanvas>` (e.g. a standalone `<BaseNode>` in a docs page)
wrap with `<FlowI18nProvider value={i18n}>` instead. `DEFAULT_FLOW_MESSAGES`
is exported so consumers can build on top of the English fallback.

## Non-goals

- No HTTP, no websocket, no SSE wiring. Connect the engine yourself
  and pump `RunOverlay` updates into a prop.
- No graph persistence — `onChange` exposes mutations.
- No opinion on which state library (zustand, react-query, …) the
  host uses.
- No bundled icon set. `spec.icon` is a string the host resolves.
