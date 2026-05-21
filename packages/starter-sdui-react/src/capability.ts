/**
 * IR-version capability handshake — the renderer refuses to project
 * any tree whose `ir_version` exceeds `SUPPORTED_IR_VERSION`. Mirrors
 * the backend constant in `starter-ui-ir`.
 *
 * Per **R2** in `DOCS/frontend/sdui/SCOPE.md`: adding a component
 * variant is a minor bump (back-compat, this constant stays);
 * removing or re-shaping is a major bump and this constant follows.
 * Lower-versioned trees are accepted — that's the server clamping
 * emission to an older IR for an older client (the back-compat path).
 */
import type { UiComponentTree } from "./types.js";

/** Highest `ir_version` this renderer knows how to project. */
export const SUPPORTED_IR_VERSION = 5;

export interface CapabilityMismatch {
  kind: "capability-mismatch";
  supported: number;
  received: number;
}

/**
 * Returns a mismatch descriptor if the tree advertises a higher
 * `ir_version` than the renderer supports; returns `null` otherwise.
 */
export function checkIrVersion(
  tree: UiComponentTree,
): CapabilityMismatch | null {
  if (tree.ir_version > SUPPORTED_IR_VERSION) {
    return {
      kind: "capability-mismatch",
      supported: SUPPORTED_IR_VERSION,
      received: tree.ir_version,
    };
  }
  return null;
}
