import { useState } from "react";
import {
  ChevronDown,
  ChevronUp,
  Pencil,
  Plus,
  Shield,
  Trash2,
} from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import { useQuery } from "@tanstack/react-query";
import type { ResourceInstance } from "@nube/starter-client-ts";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { PageDetailDrawer } from "@nube/starter-ui-authz";

import type { CreateNavNodeRequest, NavNodeDetail } from "@/api/types";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import type { NavTreeNode } from "@/features/nav/navTree";
import { NavNodeFormDialog } from "@/features/nav/NavNodeFormDialog";
import { ROUTE_META } from "@/features/nav/routeMeta";
import {
  useCreateNavNode,
  useRemoveNavNode,
  useUpdateNavNode,
} from "@/features/nav/useNavMutations";
import { useNavTree } from "@/features/nav/useNavTree";

const NAV_NODE_KIND = "nexus.nav_node";

type Editing =
  | { mode: "create"; parentId: string | null; sortOrder: number }
  | { mode: "edit"; node: NavNodeDetail };

// The Navigation builder (WS-13 §7) — an admin area to author the nav tree:
// add group/dashboard-mount/route nodes, nest under a parent, reorder among
// siblings, and manage each node's access inline (the same permissions drawer
// the Access section uses). The signed-in admin sees the tree filtered to nodes
// they hold `view` on, like everyone else.
//
// This lives as the Navigation tab of the Access section (nav + access in one
// place), so the body is split out as an embeddable component without the
// standalone page chrome (max-width wrapper / h1).
export function NavBuilder() {
  const client = useStarterClient();
  const { tree, isPending, isError } = useNavTree();
  const create = useCreateNavNode();
  const update = useUpdateNavNode();
  const remove = useRemoveNavNode();
  const [editing, setEditing] = useState<Editing | null>(null);
  // The node id whose access drawer is open; the live ResourceInstance (with its
  // real ACL) is resolved from the instances endpoint so the drawer shows the
  // true share scope rather than a synthesised one.
  const [managingId, setManagingId] = useState<string | null>(null);
  const instances = useQuery({
    queryKey: ["nexus", "access", "nav"],
    queryFn: () => client.listResourceInstances(NAV_NODE_KIND, {}),
  });
  const managing: ResourceInstance | null =
    instances.data?.items.find((i) => i.id === managingId) ?? null;

  function onSubmit(
    payload: Pick<CreateNavNodeRequest, "title" | "target" | "context">,
  ) {
    if (!editing) return;
    if (editing.mode === "create") {
      create.mutate({
        ...payload,
        parent_id: editing.parentId ?? undefined,
        sort_order: editing.sortOrder,
      });
    } else {
      // Retargeting away from a dashboard must clear the now-dangling context.
      const clearContext =
        payload.target?.kind !== "dashboard" && !!editing.node.context;
      update.mutate({
        id: editing.node.id,
        patch: { ...payload, clear_context: clearContext || undefined },
      });
    }
    setEditing(null);
  }

  return (
    <div className="flex h-full w-full flex-col gap-4">
      <header className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          Build the sidebar: mount pages, group them, and grant access per node.
        </p>
        <Button
          onClick={() =>
            setEditing({ mode: "create", parentId: null, sortOrder: tree.length })
          }
        >
          <Plus className="size-4" /> Add node
        </Button>
      </header>

      {isPending ? (
        <Loading label="Loading navigation…" />
      ) : isError ? (
        <ErrorState message="Couldn't load the navigation tree." />
      ) : tree.length === 0 ? (
        <Empty
          title="No navigation yet"
          description="Add a group or a page to start building the sidebar."
        />
      ) : (
        <ul className="flex flex-col gap-1">
          {tree.map((n, i) => (
            <NavRow
              key={n.id}
              node={n}
              depth={0}
              index={i}
              siblings={tree}
              onAddChild={() =>
                setEditing({
                  mode: "create",
                  parentId: n.id,
                  sortOrder: n.children.length,
                })
              }
              onEdit={() => setEditing({ mode: "edit", node: n })}
              onDelete={() => remove.mutate(n.id)}
              onReorder={(dir) => reorder(update, n, tree, i, dir)}
              onManage={() => setManagingId(n.id)}
              renderChild={(child, ci, sibs) => (
                <NavRowRecursive
                  key={child.id}
                  node={child}
                  depth={1}
                  index={ci}
                  siblings={sibs}
                  update={update}
                  remove={remove}
                  setEditing={setEditing}
                  setManagingId={setManagingId}
                />
              )}
            />
          ))}
        </ul>
      )}

      {editing ? (
        <NavNodeFormDialog
          open
          initial={editing.mode === "edit" ? editing.node : undefined}
          onSubmit={onSubmit}
          onClose={() => setEditing(null)}
        />
      ) : null}

      <PageDetailDrawer
        page={managing}
        kind={NAV_NODE_KIND}
        tenantId=""
        onClose={() => setManagingId(null)}
      />
    </div>
  );
}

// One row + its (recursively rendered) children.
function NavRow({
  node,
  depth,
  index,
  siblings,
  onAddChild,
  onEdit,
  onDelete,
  onReorder,
  onManage,
  renderChild,
}: {
  node: NavTreeNode;
  depth: number;
  index: number;
  siblings: NavTreeNode[];
  onAddChild: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onReorder: (dir: -1 | 1) => void;
  onManage: () => void;
  renderChild: (
    child: NavTreeNode,
    index: number,
    siblings: NavTreeNode[],
  ) => React.ReactNode;
}) {
  return (
    <li>
      <div
        className="flex items-center justify-between gap-2 rounded-lg border border-border bg-card px-3 py-2"
        style={{ marginLeft: depth * 16 }}
      >
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-sm font-medium">{node.title}</span>
          <TargetBadge node={node} />
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <IconBtn label="Move up" disabled={index === 0} onClick={() => onReorder(-1)}>
            <ChevronUp className="size-4" />
          </IconBtn>
          <IconBtn
            label="Move down"
            disabled={index === siblings.length - 1}
            onClick={() => onReorder(1)}
          >
            <ChevronDown className="size-4" />
          </IconBtn>
          <IconBtn label="Add child" onClick={onAddChild}>
            <Plus className="size-4" />
          </IconBtn>
          <IconBtn label="Manage access" onClick={onManage}>
            <Shield className="size-4" />
          </IconBtn>
          <IconBtn label="Edit" onClick={onEdit}>
            <Pencil className="size-4" />
          </IconBtn>
          <IconBtn label="Delete" onClick={onDelete}>
            <Trash2 className="size-4" />
          </IconBtn>
        </div>
      </div>
      {node.children.length > 0 ? (
        <ul className="mt-1 flex flex-col gap-1">
          {node.children.map((c, ci) => renderChild(c, ci, node.children))}
        </ul>
      ) : null}
    </li>
  );
}

// Recursive child renderer, threading the mutation hooks down so nested rows
// edit/delete/reorder/manage like roots.
function NavRowRecursive({
  node,
  depth,
  index,
  siblings,
  update,
  remove,
  setEditing,
  setManagingId,
}: {
  node: NavTreeNode;
  depth: number;
  index: number;
  siblings: NavTreeNode[];
  update: ReturnType<typeof useUpdateNavNode>;
  remove: ReturnType<typeof useRemoveNavNode>;
  setEditing: (e: Editing) => void;
  setManagingId: (id: string) => void;
}) {
  return (
    <NavRow
      node={node}
      depth={depth}
      index={index}
      siblings={siblings}
      onAddChild={() =>
        setEditing({
          mode: "create",
          parentId: node.id,
          sortOrder: node.children.length,
        })
      }
      onEdit={() => setEditing({ mode: "edit", node })}
      onDelete={() => remove.mutate(node.id)}
      onReorder={(dir) => reorder(update, node, siblings, index, dir)}
      onManage={() => setManagingId(node.id)}
      renderChild={(child, ci, sibs) => (
        <NavRowRecursive
          key={child.id}
          node={child}
          depth={depth + 1}
          index={ci}
          siblings={sibs}
          update={update}
          remove={remove}
          setEditing={setEditing}
          setManagingId={setManagingId}
        />
      )}
    />
  );
}

function TargetBadge({ node }: { node: NavTreeNode }) {
  if (node.target.kind === "group")
    return <Badge variant="secondary">Group</Badge>;
  if (node.target.kind === "route")
    return <Badge variant="outline">{ROUTE_META[node.target.route].label}</Badge>;
  return <Badge variant="outline">Dashboard</Badge>;
}

function IconBtn({
  label,
  onClick,
  disabled,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  children: React.ReactNode;
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className="size-7"
      title={label}
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}

// Swap a node with its neighbour by exchanging sort_order — a reorder among
// siblings. Both writes invalidate the tree, so the list re-settles.
function reorder(
  update: ReturnType<typeof useUpdateNavNode>,
  node: NavTreeNode,
  siblings: NavTreeNode[],
  index: number,
  dir: -1 | 1,
) {
  const other = siblings[index + dir];
  if (!other) return;
  update.mutate({ id: node.id, patch: { sort_order: other.sort_order } });
  update.mutate({ id: other.id, patch: { sort_order: node.sort_order } });
}

