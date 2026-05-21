/**
 * Core dispatcher — switches on `node.type` and delegates to the
 * matching `ComponentSpec` in `builtinComponentRegistry`. Unknown
 * types degrade to a neutral placeholder so a single unrecognised
 * variant never crashes the whole tree.
 *
 * Size budget: this file is the **only** dispatch surface; per the
 * SCOPE's Phase-4 acceptance criterion it stays ≤ 800 lines TSX (CI
 * gate). Anything that wants to grow the dispatch goes into a
 * `ComponentSpec` in `components/`, not into this file.
 */
import { Fragment } from "react";
import type { UiComponent } from "./types.js";
import { useSdui } from "./context.js";
import { lookupSpec } from "./registry/index.js";
import { evaluateShowWhen } from "./show-when.js";

export function Renderer({ node }: { node: UiComponent }) {
  const { pageState } = useSdui();

  // V1.7 `show_when` gate. Read the predicate from the node's style
  // bag and unmount when it evaluates false. The hook always runs so
  // the rules-of-hooks invariant holds; the predicate is cheap.
  const showWhen = node.style?.show_when;
  if (showWhen && !evaluateShowWhen(showWhen, pageState)) {
    return null;
  }

  const spec = lookupSpec(node.type);
  if (spec) {
    const Comp = spec.Component as React.ComponentType<{ node: unknown }>;
    return <Comp node={node} />;
  }

  return (
    <div className="rounded border border-dashed border-muted-foreground/40 px-3 py-2 text-xs text-muted-foreground">
      Unknown component: {node.type}
    </div>
  );
}

/**
 * Convenience — render a list of children. Layout containers
 * (page, row, col, grid, stack, tabs, card, form) call this for
 * their `children` array. `parentId` / `parentType` are reserved
 * for a future visual-builder pane that wraps children with
 * drop-zones; live-render call sites pass them but they are unused
 * in this phase.
 */
export function RendererList({
  nodes,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  parentId,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  parentType,
}: {
  nodes: UiComponent[];
  parentId?: string;
  parentType?: string;
}) {
  return (
    <Fragment>
      {nodes.map((n, i) => (
        <Renderer key={n.id ?? i} node={n} />
      ))}
    </Fragment>
  );
}
