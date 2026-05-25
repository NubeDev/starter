// Rubix-side specs + renderer for the built-in `starter.flow.*`
// node kinds (counter, log, trigger.schedule).
//
// Lives rubix-side because upstream `@nube/starter-ui-flow` only
// ships generic kinds (ai-agent, tool-call, trigger, branch, …)
// plus a single `starter.flow.counter` example. The other rubix
// runtime kinds (`starter.flow.log`, `starter.flow.trigger.schedule`)
// are real kinds in the agent's `NodeKindRegistry` but had no UI
// spec — without one xyflow renders them as a plain default node
// with no slot handles, which silently breaks the connect-by-drag
// surface and the per-slot value badges.

import type { NodeProps } from "@xyflow/react";
import {
  BaseNode,
  type NodeKindEntry,
  type NodeKindSpec,
  type SlotName,
} from "@nube/starter-ui-flow";
import type { NodeRunState } from "@nube/starter-ui-flow";

export const STARTER_FLOW_LOG_SPEC: NodeKindSpec = {
  kind: "starter.flow.log",
  label: "Log",
  category: "actions",
  color: "#64748b",
  icon: "scroll-text",
  inputs: [{ name: "value", kind: "any", label: "value" }],
  outputs: [],
};

export const STARTER_FLOW_TRIGGER_SCHEDULE_SPEC: NodeKindSpec = {
  kind: "starter.flow.trigger.schedule",
  label: "Schedule",
  category: "triggers",
  color: "#ef4444",
  icon: "clock",
  inputs: [],
  outputs: [{ name: "fire", kind: "trigger", label: "fire" }],
};

interface GenericNodeData {
  kindSpec?: NodeKindSpec;
  label?: string;
  state?: NodeRunState;
  slotValues?: Record<SlotName, unknown>;
}

function makeGenericRenderer(fallback: NodeKindSpec) {
  return function GenericNode(props: NodeProps) {
    const data = (props.data ?? {}) as GenericNodeData;
    return (
      <BaseNode
        spec={data.kindSpec ?? fallback}
        label={data.label}
        state={data.state}
        selected={props.selected}
        slotValues={data.slotValues}
      />
    );
  };
}

export const STARTER_FLOW_LOG_ENTRY: NodeKindEntry = {
  spec: STARTER_FLOW_LOG_SPEC,
  component: makeGenericRenderer(STARTER_FLOW_LOG_SPEC),
};

export const STARTER_FLOW_TRIGGER_SCHEDULE_ENTRY: NodeKindEntry = {
  spec: STARTER_FLOW_TRIGGER_SCHEDULE_SPEC,
  component: makeGenericRenderer(STARTER_FLOW_TRIGGER_SCHEDULE_SPEC),
};
