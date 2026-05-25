import type { NodeProps } from "@xyflow/react";
import type { NodeKindSpec, NodeRunState, SlotName } from "../types.js";
import { BaseNode } from "./BaseNode.js";
import type { NodeKindEntry } from "./NodeRegistry.js";

/**
 * Built-in node kinds matching `DOCS/flow/scope/SCOPE.md`. The visuals
 * are deliberately plain; consumers can override any kind by
 * registering their own component under the same `kind` id.
 */

export const AI_AGENT_SPEC: NodeKindSpec = {
  kind: "ai-agent",
  label: "AI Agent",
  category: "ai",
  color: "#7c3aed",
  icon: "sparkles",
  inputs: [
    { name: "in", kind: "any", label: "input" },
    { name: "tools", kind: "json", label: "tools" },
  ],
  outputs: [
    { name: "out", kind: "any", label: "output" },
    { name: "events", kind: "stream", label: "events" },
  ],
};

export const TOOL_CALL_SPEC: NodeKindSpec = {
  kind: "tool-call",
  label: "Tool Call",
  category: "actions",
  color: "#0ea5e9",
  icon: "wrench",
  inputs: [{ name: "args", kind: "json", label: "args", required: true }],
  outputs: [
    { name: "result", kind: "json", label: "result" },
    { name: "error", kind: "json", label: "error" },
  ],
};

export const TRIGGER_SPEC: NodeKindSpec = {
  kind: "trigger",
  label: "Trigger",
  category: "triggers",
  color: "#ef4444",
  icon: "play",
  inputs: [],
  outputs: [{ name: "fire", kind: "trigger", label: "fire" }],
};

export const BRANCH_SPEC: NodeKindSpec = {
  kind: "branch",
  label: "Branch",
  category: "control",
  color: "#f59e0b",
  icon: "git-branch",
  inputs: [
    { name: "in", kind: "any", required: true },
    { name: "cond", kind: "boolean", label: "if", required: true },
  ],
  outputs: [
    { name: "then", kind: "any" },
    { name: "else", kind: "any" },
  ],
};

export const TRANSFORM_SPEC: NodeKindSpec = {
  kind: "transform",
  label: "Transform",
  category: "data",
  color: "#10b981",
  icon: "function",
  inputs: [{ name: "in", kind: "any", required: true }],
  outputs: [{ name: "out", kind: "any" }],
};

export const COUNTER_SPEC: NodeKindSpec = {
  kind: "starter.flow.counter",
  label: "Counter",
  category: "data",
  color: "#22c55e",
  icon: "hash",
  inputs: [{ name: "in", kind: "any" }],
  // SlotKind has no `i64`; the runtime `out` carries `SlotValue::Int`
  // (i64). Surface it as `number` on the UI palette.
  outputs: [{ name: "out", kind: "number", label: "i64" }],
};

export const SUBFLOW_SPEC: NodeKindSpec = {
  kind: "subflow",
  label: "Subflow",
  category: "control",
  color: "#64748b",
  icon: "box",
  inputs: [{ name: "in", kind: "any" }],
  outputs: [{ name: "out", kind: "any" }],
};

interface FlowNodeData {
  kindSpec: NodeKindSpec;
  label?: string;
  state?: NodeRunState;
  preview?: string;
  slotValues?: Record<SlotName, unknown>;
}

function genericRenderer(props: NodeProps) {
  const data = props.data as unknown as FlowNodeData;
  return (
    <BaseNode
      spec={data.kindSpec}
      label={data.label}
      state={data.state}
      selected={props.selected}
      slotValues={data.slotValues}
    >
      {data.preview ? <span>{data.preview}</span> : null}
    </BaseNode>
  );
}

export const BUILTIN_NODE_KINDS: NodeKindEntry[] = [
  { spec: AI_AGENT_SPEC, component: genericRenderer },
  { spec: TOOL_CALL_SPEC, component: genericRenderer },
  { spec: TRIGGER_SPEC, component: genericRenderer },
  { spec: BRANCH_SPEC, component: genericRenderer },
  { spec: TRANSFORM_SPEC, component: genericRenderer },
  { spec: SUBFLOW_SPEC, component: genericRenderer },
  { spec: COUNTER_SPEC, component: genericRenderer },
];
