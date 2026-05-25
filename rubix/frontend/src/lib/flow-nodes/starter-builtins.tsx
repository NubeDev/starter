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
  // Two outputs, matching the backend node
  // (crates/starter-flow-nodes/src/trigger_schedule.rs):
  //
  //   * `fire`      — per-tick `SlotValue::Int(unix_ms)`. The signal
  //                   downstream nodes wire to for fan-out. Varies
  //                   per tick by construction, so the engine's R3
  //                   idempotent-write short-circuit never suppresses
  //                   propagation.
  //   * `schedule`  — constant `SlotValue::String(cron_expr)` for
  //                   enumeration / inspection surfaces (FlowAsService).
  //                   Do NOT wire downstream fan-out to this slot —
  //                   R3 will swallow every emit after the first and
  //                   the pipeline goes silent.
  outputs: [
    { name: "fire", kind: "trigger", label: "fire" },
    { name: "schedule", kind: "string", label: "schedule" },
  ],
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
