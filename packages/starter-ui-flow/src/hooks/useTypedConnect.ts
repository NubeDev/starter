import { useCallback } from "react";
import type { Connection, Edge, IsValidConnection } from "@xyflow/react";
import type { NodeKindRegistry } from "../nodes/NodeRegistry.js";
import type { FlowNode, SlotKind } from "../types.js";

/**
 * Slot-kind compatibility. `any` is compatible with everything;
 * otherwise kinds must match. Hosts can override by passing their
 * own `compatible` callback.
 */
export function defaultCompatible(a: SlotKind, b: SlotKind): boolean {
  if (a === "any" || b === "any") return true;
  return a === b;
}

interface UseTypedConnectArgs {
  registry: NodeKindRegistry;
  nodes: FlowNode[];
  compatible?: (a: SlotKind, b: SlotKind) => boolean;
}

/**
 * Returns an `isValidConnection` callback for @xyflow/react's
 * `<ReactFlow isValidConnection={...} />`. Rejects connections whose
 * source-slot kind is incompatible with the target-slot kind.
 */
export function useTypedConnect({
  registry,
  nodes,
  compatible = defaultCompatible,
}: UseTypedConnectArgs): IsValidConnection {
  return useCallback<IsValidConnection>(
    (c: Edge | Connection) => {
      if (!c.source || !c.target || !c.sourceHandle || !c.targetHandle) {
        return false;
      }
      if (c.source === c.target) return false;

      const src = nodes.find((n) => n.id === c.source);
      const dst = nodes.find((n) => n.id === c.target);
      if (!src || !dst) return false;

      const srcSpec = registry.get(src.kind)?.spec;
      const dstSpec = registry.get(dst.kind)?.spec;
      if (!srcSpec || !dstSpec) return false;

      const srcSlot = srcSpec.outputs.find((s) => s.name === c.sourceHandle);
      const dstSlot = dstSpec.inputs.find((s) => s.name === c.targetHandle);
      if (!srcSlot || !dstSlot) return false;

      return compatible(srcSlot.kind, dstSlot.kind);
    },
    [registry, nodes, compatible],
  );
}
