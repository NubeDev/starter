// Rubix-side node-kind registry.
//
// Builds a `NodeKindRegistry` at app boot seeded with the built-in
// node kinds shipped by `@nube/starter-ui-flow` (ai-agent, tool-call,
// trigger, branch, transform, subflow), then overrides specific
// entries with rubix-side custom components.
//
// Currently the only rubix override is for `ai-agent`, which renders
// extra config (skill hint + allowed-tools count) supplied by the
// rubix `ai-agent` node body. Add further overrides here as more
// rubix-specific node kinds land — never modify `@nube/starter-ui-flow`
// itself.

import {
  AI_AGENT_SPEC,
  BRANCH_SPEC,
  COUNTER_SPEC,
  NodeKindRegistry,
  TOOL_CALL_SPEC,
  TRANSFORM_SPEC,
  TRIGGER_SPEC,
  SUBFLOW_SPEC,
  type NodeKindEntry,
} from "@nube/starter-ui-flow";
import { BUILTIN_NODE_KINDS } from "@nube/starter-ui-flow";
import { RubixAiAgentNode } from "./flow-nodes/ai-agent-node.js";
import {
  STARTER_FLOW_LOG_ENTRY,
  STARTER_FLOW_TRIGGER_SCHEDULE_ENTRY,
} from "./flow-nodes/starter-builtins.js";

/**
 * The set of built-in entries we re-export through the registry,
 * unchanged from `@nube/starter-ui-flow`. Kept as a named list rather
 * than a blind spread so we notice when upstream adds a new kind.
 */
const BUILTIN_KINDS_KEPT: ReadonlyArray<string> = [
  TOOL_CALL_SPEC.kind,
  TRIGGER_SPEC.kind,
  BRANCH_SPEC.kind,
  TRANSFORM_SPEC.kind,
  SUBFLOW_SPEC.kind,
  COUNTER_SPEC.kind,
];

/** Rubix override for the `ai-agent` node kind. */
const RUBIX_AI_AGENT_ENTRY: NodeKindEntry = {
  spec: AI_AGENT_SPEC,
  component: RubixAiAgentNode,
};

/**
 * Build the per-app NodeKindRegistry. Call once at app boot and pass
 * the result down through `FlowCanvas` consumers.
 *
 * Builtin entries are registered first; rubix overrides are layered
 * on top by removing the upstream entry from the candidate list
 * before re-adding our replacement.
 */
export function buildFlowRegistry(): NodeKindRegistry {
  const registry = new NodeKindRegistry();

  const overriddenKinds = new Set<string>([AI_AGENT_SPEC.kind]);

  for (const entry of BUILTIN_NODE_KINDS) {
    if (overriddenKinds.has(entry.spec.kind)) continue;
    // Sanity-guard: only register builtins we know about. Anything
    // upstream adds will be ignored here until we explicitly opt in.
    if (
      !BUILTIN_KINDS_KEPT.includes(entry.spec.kind) &&
      entry.spec.kind !== AI_AGENT_SPEC.kind
    ) {
      continue;
    }
    registry.register(entry);
  }

  registry.register(RUBIX_AI_AGENT_ENTRY);
  // Rubix-specific built-in kinds the upstream package doesn't ship
  // a spec for. Without these xyflow falls back to its default node
  // renderer, which has no slot handles — so edges can't connect to
  // them and they look like a plain box on the canvas.
  registry.register(STARTER_FLOW_LOG_ENTRY);
  registry.register(STARTER_FLOW_TRIGGER_SCHEDULE_ENTRY);
  return registry;
}

/** Lazily-built singleton for app code that just wants the registry. */
let cached: NodeKindRegistry | undefined;
export function getFlowRegistry(): NodeKindRegistry {
  if (!cached) cached = buildFlowRegistry();
  return cached;
}
