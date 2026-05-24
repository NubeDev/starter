import { Handle, Position, type HandleProps } from "@xyflow/react";
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
 *
 * The connector gets its colour from the slot's `kind` (see
 * `colorForKind`). Everything else is class-driven so hosts can
 * restyle by overriding the `--sf-*` variables or by targeting
 * `.sf-slot`, `.sf-slot__handle`, and `.sf-slot__label`.
 */
export function SlotHandle({ spec, side, id }: SlotHandleProps) {
  const position = side === "input" ? Position.Left : Position.Right;
  const type: HandleProps["type"] = side === "input" ? "target" : "source";
  const handleId = id ?? spec.name;
  const color = colorForKind(spec.kind);
  return (
    <div
      className={`sf-slot sf-slot--${side}`}
      data-slot-kind={spec.kind}
      data-slot-required={spec.required ? "" : undefined}
    >
      <Handle
        id={handleId}
        type={type}
        position={position}
        className="sf-slot__handle"
        data-slot-kind={spec.kind}
        style={{ background: color }}
      />
      <span className="sf-slot__label">
        {spec.label ?? spec.name}
        {spec.required ? <span className="sf-slot__required" aria-hidden="true"> *</span> : null}
      </span>
    </div>
  );
}
