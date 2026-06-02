// `wiresheet/kinds.ts` — the node-kind vocabulary the wiresheet
// renders. These are the control-engine block types a flow-based
// runtime (Niagara-kit / Sedona) exposes: points, logic, math, etc.
//
// SCAFFOLD: a small starter set. Extend this to mirror the real
// engine's block palette; the backend's `engine_wiresheet_get` should
// emit `nodes[].kind` values drawn from these ids.

import {
  NodeKindRegistry,
  type NodeKindSpec,
} from "@nube/starter-ui-flow/nodes";

import { EngineNode } from "./engine-node";

export const POINT_IN_SPEC: NodeKindSpec = {
  kind: "point-in",
  label: "Input Point",
  category: "io",
  color: "#0ea5e9",
  icon: "arrow-down",
  inputs: [],
  outputs: [{ name: "out", kind: "number", label: "value" }],
};

export const POINT_OUT_SPEC: NodeKindSpec = {
  kind: "point-out",
  label: "Output Point",
  category: "io",
  color: "#22c55e",
  icon: "arrow-up",
  inputs: [{ name: "in", kind: "number", label: "value" }],
  outputs: [],
};

export const MATH_SPEC: NodeKindSpec = {
  kind: "math",
  label: "Math",
  category: "logic",
  color: "#a855f7",
  icon: "plus",
  inputs: [
    { name: "a", kind: "number" },
    { name: "b", kind: "number" },
  ],
  outputs: [{ name: "out", kind: "number" }],
};

export const LOGIC_SPEC: NodeKindSpec = {
  kind: "logic",
  label: "Logic",
  category: "logic",
  color: "#f59e0b",
  icon: "git-branch",
  inputs: [
    { name: "a", kind: "boolean" },
    { name: "b", kind: "boolean" },
  ],
  outputs: [{ name: "out", kind: "boolean" }],
};

export const WIRESHEET_KINDS: NodeKindSpec[] = [
  POINT_IN_SPEC,
  POINT_OUT_SPEC,
  MATH_SPEC,
  LOGIC_SPEC,
];

/** Registry consumed by `<FlowCanvas registry={…} />`. Every kind
 *  renders with the package's default BaseNode visuals. */
export function buildRegistry(): NodeKindRegistry {
  const reg = new NodeKindRegistry();
  for (const spec of WIRESHEET_KINDS) reg.register({ spec, component: EngineNode });
  return reg;
}
