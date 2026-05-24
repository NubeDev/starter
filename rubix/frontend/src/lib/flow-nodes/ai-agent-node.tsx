// Rubix-side custom renderer for the `ai-agent` node kind.
//
// Wraps `@nube/starter-ui-flow`'s `BaseNode` and projects two
// rubix-specific config fields from `node.data` onto the node body:
//
//   - `skill_hint` (string)       — rendered as a small label line
//   - `allowed_tools` (string[])  — rendered as a count badge
//
// This component intentionally lives rubix-side. Upstream
// `@nube/starter-ui-flow` is left untouched; the override is wired in
// at boot by `buildFlowRegistry()` in `../flow-registry.ts`.

import type { NodeProps } from "@xyflow/react";
import {
  AI_AGENT_SPEC,
  BaseNode,
  type NodeKindSpec,
  type SlotName,
} from "@nube/starter-ui-flow";

/**
 * Shape of `node.data` for an `ai-agent` node. All fields optional —
 * any of them may be absent on a freshly-dropped node.
 */
interface AiAgentNodeData {
  kindSpec?: NodeKindSpec;
  label?: string;
  state?: Parameters<typeof BaseNode>[0]["state"];
  slotValues?: Record<SlotName, unknown>;
  /** Rubix-specific config payload. */
  skill_hint?: string;
  allowed_tools?: string[];
}

function isStringArray(v: unknown): v is string[] {
  return Array.isArray(v) && v.every((x) => typeof x === "string");
}

/**
 * Render the rubix `ai-agent` node. The base frame, slot handles, and
 * run-state ring all come from `BaseNode`; we only inject the body
 * extras (skill hint + allowed-tools count) via `children`.
 */
export function RubixAiAgentNode(props: NodeProps) {
  const data = (props.data ?? {}) as AiAgentNodeData;
  const spec = data.kindSpec ?? AI_AGENT_SPEC;
  const skillHint = typeof data.skill_hint === "string" ? data.skill_hint.trim() : "";
  const tools = isStringArray(data.allowed_tools) ? data.allowed_tools : [];
  const toolCount = tools.length;

  return (
    <BaseNode
      spec={spec}
      label={data.label}
      state={data.state}
      selected={props.selected}
      slotValues={data.slotValues}
    >
      <div
        className="sf-node__rubix-ai-agent"
        data-rubix-node="ai-agent"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 4,
          fontSize: 11,
          lineHeight: 1.3,
        }}
      >
        {skillHint ? (
          <span
            className="sf-node__rubix-ai-agent__skill"
            title={skillHint}
            style={{
              opacity: 0.8,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              maxWidth: 200,
            }}
          >
            <span style={{ opacity: 0.6 }}>skill:</span> {skillHint}
          </span>
        ) : (
          <span style={{ opacity: 0.45, fontStyle: "italic" }}>no skill hint</span>
        )}
        <span
          className="sf-node__rubix-ai-agent__tools"
          title={
            toolCount > 0
              ? `Allowed tools: ${tools.join(", ")}`
              : "No allowed tools configured"
          }
          style={{
            alignSelf: "flex-start",
            padding: "1px 6px",
            borderRadius: 999,
            background: "rgba(124, 58, 237, 0.12)",
            color: "var(--sf-accent, #7c3aed)",
            fontWeight: 600,
          }}
        >
          {toolCount} tool{toolCount === 1 ? "" : "s"}
        </span>
      </div>
    </BaseNode>
  );
}
