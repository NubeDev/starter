// Wire types mirroring `starter-flow-spi` on the TS side. Kept small —
// only the surface the UI needs. Backend may carry more fields; we
// preserve unknown keys via the `data` bag so round-trips don't lose
// information.

export type NodeId = string;
export type FlowId = string;
export type SlotName = string;

/**
 * Slot value kinds the UI knows how to colour and validate.
 * Backend may extend this; unknown kinds render as the `any` style.
 */
export type SlotKind =
  | "any"
  | "string"
  | "number"
  | "boolean"
  | "json"
  | "bytes"
  | "event"
  | "trigger"
  | "stream";

export interface SlotSpec {
  name: SlotName;
  kind: SlotKind;
  /** Optional human label. Falls back to `name`. */
  label?: string;
  /** Marks a required input — UI badges it. */
  required?: boolean;
  /** Free-form description shown in tooltips. */
  description?: string;
}

/**
 * Static description of a node kind. Mirrors what the backend
 * `NodeKindRegistry` exposes. Visuals are looked up by `kind`.
 */
export interface NodeKindSpec {
  /** Stable id: e.g. `ai-agent`, `tool-call`, `branch`. */
  kind: string;
  /** Human label for the palette. */
  label: string;
  /** Category for grouping in the palette. */
  category?: string;
  /** Optional accent colour. Hex or CSS var. */
  color?: string;
  /** Optional icon name. Host app resolves to a component. */
  icon?: string;
  inputs: SlotSpec[];
  outputs: SlotSpec[];
}

/** Runtime instance of a node placed on a canvas. */
export interface FlowNode {
  id: NodeId;
  kind: string;
  position: { x: number; y: number };
  /** User-set label. Falls back to the kind label. */
  label?: string;
  /** Arbitrary config. The kind owns the schema. */
  data?: Record<string, unknown>;
}

/** A typed connection between two slots. */
export interface FlowEdge {
  id: string;
  source: NodeId;
  sourceSlot: SlotName;
  target: NodeId;
  targetSlot: SlotName;
}

export interface FlowGraph {
  nodes: FlowNode[];
  edges: FlowEdge[];
}

/**
 * Live status overlay for runtime visualisation. Optional —
 * authoring-only canvases pass nothing.
 */
export type NodeRunState =
  | "idle"
  | "ready"
  | "running"
  | "ok"
  | "error"
  | "cancelled"
  | "skipped";

export interface RunOverlay {
  nodes: Record<NodeId, NodeRunState>;
  /** Active edges (currently propagating). */
  activeEdges?: string[];
}
