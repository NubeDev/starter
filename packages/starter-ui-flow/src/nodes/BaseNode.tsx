import type { CSSProperties, ReactNode } from "react";
import type { NodeKindSpec, NodeRunState, SlotName, SlotSpec } from "../types.js";
import { SlotHandle } from "../slots/SlotHandle.js";
import { cn } from "../lib/cn.js";

export interface BaseNodeProps {
  spec: NodeKindSpec;
  label?: string;
  state?: NodeRunState;
  selected?: boolean;
  /**
   * Live per-slot values, keyed by slot name. Renderers show each
   * value as a small monospaced badge adjacent to its slot label.
   * Currently rendered for output slots only — input badges land
   * once the engine emits input-write events.
   */
  slotValues?: Record<SlotName, unknown>;
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
  slotValues,
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
            <SlotRow key={`in-${s.name}`} spec={s} side="input" value={slotValues?.[s.name]} />
          ))}
        </div>
        <div
          className="sf-node__outputs"
          style={{ display: "flex", flexDirection: "column", alignItems: "flex-end" }}
        >
          {spec.outputs.map((s) => (
            <SlotRow key={`out-${s.name}`} spec={s} side="output" value={slotValues?.[s.name]} />
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

const BADGE_MAX = 48;

function SlotRow({
  spec,
  side,
  value,
}: {
  spec: SlotSpec;
  side: "input" | "output";
  value: unknown;
}) {
  const rendered = renderSlotValue(value);
  const wrap: CSSProperties = {
    display: "flex",
    flexDirection: "column",
    alignItems: side === "input" ? "flex-start" : "flex-end",
  };
  const badge: CSSProperties = {
    marginLeft: side === "input" ? 16 : 0,
    marginRight: side === "output" ? 16 : 0,
    marginTop: 1,
    padding: "1px 6px",
    borderRadius: 4,
    background: "var(--sf-slot-value-bg, rgba(15,23,42,0.06))",
    color: "var(--sf-slot-value-fg, #0f172a)",
    fontFamily:
      "var(--sf-mono, ui-monospace, SFMono-Regular, Menlo, monospace)",
    fontSize: 10,
    lineHeight: 1.3,
    maxWidth: 180,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  };
  return (
    <div style={wrap}>
      <SlotHandle spec={spec} side={side} />
      {rendered !== null ? (
        <span
          className="sf-slot__value"
          data-slot-kind={spec.kind}
          title={rendered}
          style={badge}
        >
          {rendered.length > BADGE_MAX
            ? `${rendered.slice(0, BADGE_MAX - 1)}…`
            : rendered}
        </span>
      ) : null}
    </div>
  );
}

/**
 * Project an arbitrary slot value to a compact display string, or
 * `null` if the badge should be hidden entirely.
 *
 * Null/undefined collapse the badge. Strings render verbatim. Numbers
 * and booleans use their canonical `String(...)`. Everything else
 * falls through to `JSON.stringify`, with a fallback to `String(v)`
 * when the value contains a cycle or otherwise refuses to serialize.
 */
function renderSlotValue(v: unknown): string | null {
  if (v === null || v === undefined) return null;
  if (typeof v === "string") return v;
  if (typeof v === "number" || typeof v === "boolean" || typeof v === "bigint") {
    return String(v);
  }
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}
