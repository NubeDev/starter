import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from "@xyflow/react";
import { colorForKind } from "../slots/colors.js";

/**
 * Edge that colours itself by the source-slot kind. Active edges
 * (carried in the runtime overlay) animate via the `sf-edge--active`
 * CSS class.
 */
export function TypedEdge(props: EdgeProps) {
  const {
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    data,
    markerEnd,
    label,
  } = props;

  const slotKind = ((data as { slotKind?: string } | undefined)?.slotKind) ?? "any";
  const active = Boolean((data as { active?: boolean } | undefined)?.active);
  const color = colorForKind(slotKind);

  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        markerEnd={markerEnd}
        style={{
          stroke: color,
          strokeWidth: active ? 2.5 : 1.5,
          strokeDasharray: active ? "6 4" : undefined,
        }}
        className={active ? "sf-edge sf-edge--active" : "sf-edge"}
      />
      {label ? (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              background: "var(--sf-edge-label-bg, #ffffff)",
              border: `1px solid ${color}`,
              color,
              borderRadius: 4,
              padding: "1px 6px",
              fontSize: 10,
              pointerEvents: "all",
            }}
            className="sf-edge__label nodrag nopan"
          >
            {label}
          </div>
        </EdgeLabelRenderer>
      ) : null}
    </>
  );
}
