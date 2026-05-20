import { Handle, Position, type HandleProps } from "@xyflow/react";
import type { CSSProperties } from "react";
import type { SlotSpec } from "../types.js";
import { colorForKind } from "./colors.js";

interface SlotHandleProps {
  spec: SlotSpec;
  side: "input" | "output";
  /** Override the handle id. Defaults to `spec.name`. */
  id?: string;
}

/**
 * A typed slot handle. Renders a coloured connector on the side of a
 * node with a label adjacent to it. The handle `id` is the slot name,
 * which is what `useTypedConnect` uses to validate connections.
 */
export function SlotHandle({ spec, side, id }: SlotHandleProps) {
  const position = side === "input" ? Position.Left : Position.Right;
  const type: HandleProps["type"] = side === "input" ? "target" : "source";
  const handleId = id ?? spec.name;
  const color = colorForKind(spec.kind);

  const handleStyle: CSSProperties = {
    background: color,
    width: 10,
    height: 10,
    border: "2px solid var(--sf-handle-border, #ffffff)",
  };

  const rowStyle: CSSProperties = {
    display: "flex",
    flexDirection: side === "input" ? "row" : "row-reverse",
    alignItems: "center",
    gap: 6,
    position: "relative",
    padding: "2px 0",
  };

  const labelStyle: CSSProperties = {
    fontSize: 11,
    color: "var(--sf-slot-label, #475569)",
    whiteSpace: "nowrap",
  };

  return (
    <div className="sf-slot" data-slot-kind={spec.kind} style={rowStyle}>
      <Handle
        id={handleId}
        type={type}
        position={position}
        style={handleStyle}
        data-slot-kind={spec.kind}
      />
      <span style={labelStyle}>
        {spec.label ?? spec.name}
        {spec.required ? <span style={{ color: "#ef4444" }}> *</span> : null}
      </span>
    </div>
  );
}
