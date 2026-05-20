import type { ComponentType } from "react";
import type { NodeProps } from "@xyflow/react";
import type { NodeKindSpec } from "../types.js";

/**
 * Visual component for a node kind. Receives the @xyflow/react node
 * props with `data.kindSpec` and `data.label` already populated by
 * `FlowCanvas`.
 */
export type NodeKindComponent = ComponentType<NodeProps>;

export interface NodeKindEntry {
  spec: NodeKindSpec;
  component: NodeKindComponent;
}

/**
 * Registry of node kinds. Mirrors `starter-flow::NodeKindRegistry`
 * on the UI side. The host app assembles one of these at boot,
 * registering both built-in kinds (from this package) and
 * extension-contributed kinds.
 */
export class NodeKindRegistry {
  private readonly entries = new Map<string, NodeKindEntry>();

  register(entry: NodeKindEntry): this {
    if (this.entries.has(entry.spec.kind)) {
      throw new Error(`NodeKindRegistry: duplicate kind "${entry.spec.kind}"`);
    }
    this.entries.set(entry.spec.kind, entry);
    return this;
  }

  registerAll(entries: NodeKindEntry[]): this {
    for (const e of entries) this.register(e);
    return this;
  }

  get(kind: string): NodeKindEntry | undefined {
    return this.entries.get(kind);
  }

  has(kind: string): boolean {
    return this.entries.has(kind);
  }

  list(): NodeKindEntry[] {
    return Array.from(this.entries.values());
  }

  /** Build the `nodeTypes` map @xyflow/react expects. */
  toNodeTypes(): Record<string, NodeKindComponent> {
    const out: Record<string, NodeKindComponent> = {};
    for (const [kind, entry] of this.entries) out[kind] = entry.component;
    return out;
  }
}
