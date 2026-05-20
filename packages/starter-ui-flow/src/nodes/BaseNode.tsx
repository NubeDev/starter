import type { CSSProperties, ReactNode } from "react";
import type { NodeKindSpec, NodeRunState } from "../types.js";
import { SlotHandle } from "../slots/SlotHandle.js";
import { cn } from "../lib/cn.js";

export interface BaseNodeProps {
  spec: NodeKindSpec;
  label?: string;
  state?: NodeRunState;
  selected?: boolean;
  /** Optional body slot for kind-specific config preview. */
  children?: ReactNode;
}

const STATE_BORDER: Record<NodeRunState, string> = {
  idle: "#cbd5e1",
  ready: "#3b82f6",
  running: "#f59e0b",
  ok: "#10b981",
  error: "#ef4444",
  cancelled: "#64748b",
  skipped: "#94a3b8",
};

/**
 * Visual frame for every node kind. Renders header + input column +
 * output column. Kind-specific bodies plug into `children`.
 *
 * Hosts can fully restyle by targeting `.sf-node` and
 * `[data-node-kind="…"]` in their own CSS.
 */
export function BaseNode({
  spec,
  label,
  state = "idle",
  selected,
  children,
}: BaseNodeProps) {
  const accent = spec.color ?? "#0ea5e9";
  const border = selected
    ? "#0ea5e9"
    : state !== "idle"
      ? STATE_BORDER[state]
      : "var(--sf-node-border, #e2e8f0)";

  const wrap: CSSProperties = {
    background: "var(--sf-node-bg, #ffffff)",
    border: `2px solid ${border}`,
    borderRadius: 8,
    minWidth: 200,
    boxShadow: selected
      ? "0 0 0 2px rgba(14,165,233,0.25)"
      : "0 1px 2px rgba(15,23,42,0.06)",
    fontFamily: "var(--sf-font, ui-sans-serif, system-ui, sans-serif)",
  };

  const header: CSSProperties = {
    background: accent,
    color: "#ffffff",
    padding: "6px 10px",
    borderTopLeftRadius: 6,
    borderTopRightRadius: 6,
    fontSize: 12,
    fontWeight: 600,
    display: "flex",
    justifyContent: "space-between",
    gap: 8,
  };

  const body: CSSProperties = {
    display: "grid",
    gridTemplateColumns: "1fr 1fr",
    padding: "6px 4px",
    gap: 4,
  };

  return (
    <div className={cn("sf-node", `sf-node--${state}`)} data-node-kind={spec.kind} style={wrap}>
      <div className="sf-node__header" style={header}>
        <span>{label ?? spec.label}</span>
        <span style={{ opacity: 0.85, fontWeight: 400 }}>{spec.kind}</span>
      </div>
      <div className="sf-node__body" style={body}>
        <div className="sf-node__inputs" style={{ display: "flex", flexDirection: "column" }}>
          {spec.inputs.map((s) => (
            <SlotHandle key={`in-${s.name}`} spec={s} side="input" />
          ))}
        </div>
        <div
          className="sf-node__outputs"
          style={{ display: "flex", flexDirection: "column", alignItems: "flex-end" }}
        >
          {spec.outputs.map((s) => (
            <SlotHandle key={`out-${s.name}`} spec={s} side="output" />
          ))}
        </div>
      </div>
      {children ? (
        <div
          className="sf-node__extra"
          style={{
            borderTop: "1px solid var(--sf-node-divider, #f1f5f9)",
            padding: "6px 10px",
            fontSize: 11,
            color: "var(--sf-node-extra, #475569)",
          }}
        >
          {children}
        </div>
      ) : null}
    </div>
  );
}
