/**
 * `tree` — hierarchical node list. Each node may carry an optional
 * `node_action`; the `$node.id` placeholder is substituted from the
 * clicked node before dispatch.
 *
 * Pure projection — no client-side filtering, no client-side
 * derived state (R9). Expansion state lives in component-local
 * UI state; collapse / expand never round-trips.
 */
import { useCallback, useState } from "react";
import type { ComponentSpec } from "../registry/types.js";
import type { UiComponent } from "../types.js";
import { useSdui } from "../context.js";

export interface TreeItemNode {
  id: string;
  label: string;
  children?: TreeItemNode[];
}
export interface TreeNode extends UiComponent {
  type: "tree";
  nodes?: TreeItemNode[];
  node_action?: { handler: string; args?: Record<string, unknown> };
}

function substituteNodeId(
  args: Record<string, unknown> | undefined,
  nodeId: string,
): Record<string, unknown> | undefined {
  if (!args) return undefined;
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(args)) {
    out[k] = v === "$node.id" ? nodeId : v;
  }
  return out;
}

function TreeItem({
  item,
  onClick,
}: {
  item: TreeItemNode;
  onClick: (id: string) => void;
}) {
  const [open, setOpen] = useState(true);
  const hasChildren = !!item.children?.length;
  return (
    <li>
      <div className="flex items-center gap-1.5 py-0.5 text-sm">
        {hasChildren ? (
          <button
            type="button"
            onClick={() => setOpen((v) => !v)}
            className="h-4 w-4 text-muted-foreground"
            aria-label={open ? "collapse" : "expand"}
          >
            {open ? "▾" : "▸"}
          </button>
        ) : (
          <span className="inline-block h-4 w-4" />
        )}
        <button
          type="button"
          className="hover:underline"
          onClick={() => onClick(item.id)}
        >
          {item.label}
        </button>
      </div>
      {hasChildren && open ? (
        <ul className="ml-4 border-l pl-2">
          {item.children!.map((c) => (
            <TreeItem key={c.id} item={c} onClick={onClick} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

export const treeSpec: ComponentSpec<TreeNode> = {
  kind: "tree" as never,
  Component: ({ node }) => {
    const { dispatchAction } = useSdui();
    const onClick = useCallback(
      (id: string) => {
        const action = node.node_action;
        if (!action) return;
        void dispatchAction(action.handler, substituteNodeId(action.args, id));
      },
      [dispatchAction, node.node_action],
    );
    const nodes = node.nodes ?? [];
    return (
      <ul className={`text-sm ${node.style?.className ?? ""}`}>
        {nodes.map((n) => (
          <TreeItem key={n.id} item={n} onClick={onClick} />
        ))}
      </ul>
    );
  },
};
