// Navigation Manager (Access → Navigation Manager) — a single bird's-eye view
// of the whole sidebar AND who can reach each part of it. One screen answers
// "what is the nav?" and "which teams see what, at what level?":
//
//   • The left column is the nav tree rendered like the sidebar (indented, real
//     icons), so the admin reads it as the user will.
//   • Each team is a column; each cell is a tier picker (None / View / Edit /
//     Manage) granting that team that level on that node. Flip one, flip a
//     whole team (column header → grant/revoke all), or push a node's row of
//     grants down to every descendant ("apply to children").
//   • Drag the handle to reorder among siblings; add / edit / delete from the
//     row.
//
// Access here is the same per-node grant the rest of Access uses
// (`nexus.nav_node`), so a grant made here shows up everywhere else and is
// gated by the same authz. Cascade is an *explicit* action, not stored
// inheritance: each node's access stays exactly what's persisted (auditable,
// no ambiguous "inherited vs set" state) — "apply to children" just fans the
// parent's grants out on demand.

import { useMemo, useState } from "react";
import {
  Check,
  ChevronDown,
  CornerDownRight,
  GripVertical,
  Pencil,
  Plus,
  SlidersHorizontal,
  Trash2,
} from "lucide-react";
import {
  useCreateGrant,
  useDeleteGrant,
  useGrants,
  usePatchGrant,
  useTeams,
} from "@nube/starter-ui-authz";
import type { Grant, PermissionTier } from "@nube/starter-client-ts";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@nube/starter-ui-kit/components/dropdown-menu";

import type { CreateNavNodeRequest, NavNodeDetail } from "@/api/types";
import { dashboardIcon } from "@/features/dashboards/appearance";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import type { NavTreeNode } from "@/features/nav/navTree";
import { NavNodeFormDialog } from "@/features/nav/NavNodeFormDialog";
import { GROUP_ICON, ROUTE_META } from "@/features/nav/routeMeta";
import {
  useCreateNavNode,
  useRemoveNavNode,
  useUpdateNavNode,
} from "@/features/nav/useNavMutations";
import { useNavTree } from "@/features/nav/useNavTree";

const NAV_NODE_KIND = "nexus.nav_node";

// The tier ladder, weakest → strongest. `null` is "no access" (no grant).
const TIERS: PermissionTier[] = ["View", "Edit", "Manage"];
type TierOrNone = PermissionTier | null;

const TIER_LABEL: Record<PermissionTier, string> = {
  View: "View",
  Edit: "Edit",
  Manage: "Manage",
};

type Editing =
  | { mode: "create"; parentId: string | null; sortOrder: number }
  | { mode: "edit"; node: NavNodeDetail };

/** One render row — a node flattened with its depth and sibling group so a
 *  drag reorders only among siblings. */
interface FlatRow {
  node: NavTreeNode;
  depth: number;
  siblings: NavTreeNode[];
  index: number;
}

function flatten(nodes: NavTreeNode[], depth: number, out: FlatRow[]): FlatRow[] {
  nodes.forEach((node, index) => {
    out.push({ node, depth, siblings: nodes, index });
    if (node.children.length > 0) flatten(node.children, depth + 1, out);
  });
  return out;
}

/** All descendant ids of a node (not including itself) — for cascade. */
function descendantIds(node: NavTreeNode): string[] {
  const out: string[] = [];
  const walk = (n: NavTreeNode) => {
    for (const c of n.children) {
      out.push(c.id);
      walk(c);
    }
  };
  walk(node);
  return out;
}

export function NavManager({ tenantId }: { tenantId: string | null }) {
  const { tree, isPending, isError } = useNavTree();
  const teams = useTeams(tenantId);
  // One grants query feeds the whole matrix: every grant on any nav node in
  // this tenant, indexed below by (node id → team slug → grant).
  const grants = useGrants({
    resource_kind: NAV_NODE_KIND,
    tenant_id: tenantId ?? undefined,
  });

  const createNode = useCreateNavNode();
  const updateNode = useUpdateNavNode();
  const removeNode = useRemoveNavNode();
  const createGrant = useCreateGrant();
  const deleteGrant = useDeleteGrant();
  const patchGrant = usePatchGrant();

  const [editing, setEditing] = useState<Editing | null>(null);
  const [dragId, setDragId] = useState<string | null>(null);
  // Hidden team columns (slugs). Lets the admin focus the matrix on a few teams
  // without losing the rest — purely a view filter.
  const [hidden, setHidden] = useState<Set<string>>(() => new Set());

  const rows = useMemo(() => flatten(tree, 0, []), [tree]);

  const grantIndex = useMemo(() => {
    const map = new Map<string, Map<string, Grant>>();
    for (const g of grants.data?.grants ?? []) {
      if (!g.resource_id || g.subject.kind !== "team") continue;
      const byTeam = map.get(g.resource_id) ?? new Map<string, Grant>();
      byTeam.set(g.subject.slug, g);
      map.set(g.resource_id, byTeam);
    }
    return map;
  }, [grants.data]);

  const allTeams = teams.data ?? [];
  const teamList = allTeams.filter((t) => !hidden.has(t.slug));

  function grantOf(nodeId: string, slug: string): Grant | undefined {
    return grantIndex.get(nodeId)?.get(slug);
  }
  function tierOf(nodeId: string, slug: string): TierOrNone {
    return grantOf(nodeId, slug)?.tier ?? null;
  }

  // Set a (node, team) to an exact tier: create / patch / delete the grant to
  // match. A no-op when already at that tier.
  function setTier(nodeId: string, slug: string, next: TierOrNone) {
    const existing = grantOf(nodeId, slug);
    if (next === null) {
      if (existing) deleteGrant.mutate(existing.id);
      return;
    }
    if (!existing) {
      createGrant.mutate({
        subject: { kind: "team", slug },
        resource_kind: NAV_NODE_KIND,
        resource_id: nodeId,
        tier: next,
        tenant_id: tenantId ?? "",
      });
    } else if (existing.tier !== next) {
      patchGrant.mutate({ id: existing.id, body: { tier: next } });
    }
  }

  // Column header: grant every node `View` (if any node lacks access) or revoke
  // the whole column. The "any missing" rule means the first click fills gaps.
  function toggleTeamColumn(slug: string) {
    const anyMissing = rows.some((r) => tierOf(r.node.id, slug) === null);
    for (const { node } of rows) {
      if (anyMissing) {
        if (tierOf(node.id, slug) === null) setTier(node.id, slug, "View");
      } else {
        setTier(node.id, slug, null);
      }
    }
  }

  // Push one team's tier on a node down onto every descendant — the explicit,
  // per-cell cascade. Children end up mirroring the parent's level for that
  // team (or losing access where the parent has none). Per-cell (not whole-row)
  // so the action matches the column you clicked in.
  function applyToChildren(node: NavTreeNode, slug: string) {
    const tier = tierOf(node.id, slug);
    for (const kidId of descendantIds(node)) {
      if (tierOf(kidId, slug) !== tier) setTier(kidId, slug, tier);
    }
  }

  function onSubmit(
    payload: Pick<CreateNavNodeRequest, "title" | "target" | "context">,
  ) {
    if (!editing) return;
    if (editing.mode === "create") {
      createNode.mutate({
        ...payload,
        parent_id: editing.parentId ?? undefined,
        sort_order: editing.sortOrder,
      });
    } else {
      const clearContext =
        payload.target?.kind !== "dashboard" && !!editing.node.context;
      updateNode.mutate({
        id: editing.node.id,
        patch: { ...payload, clear_context: clearContext || undefined },
      });
    }
    setEditing(null);
  }

  // Reorder within a sibling group by swapping sort_order with the drop target.
  // Cross-group moves (reparenting) stay in the Navigation builder tab.
  function onDrop(target: FlatRow) {
    const dragged = rows.find((r) => r.node.id === dragId);
    setDragId(null);
    if (!dragged || dragged.node.id === target.node.id) return;
    if (dragged.siblings !== target.siblings) return;
    updateNode.mutate({
      id: dragged.node.id,
      patch: { sort_order: target.node.sort_order },
    });
    updateNode.mutate({
      id: target.node.id,
      patch: { sort_order: dragged.node.sort_order },
    });
  }

  if (isPending || teams.isPending) return <Loading label="Loading navigation…" />;
  if (isError) return <ErrorState message="Couldn't load the navigation tree." />;
  if (tree.length === 0) {
    return (
      <Empty
        title="No navigation yet"
        description="Add a group or a page to start building the sidebar."
      />
    );
  }

  const mutating =
    createGrant.isPending || deleteGrant.isPending || patchGrant.isPending;

  return (
    <div className="flex h-full flex-col gap-3">
      <header className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          See the whole sidebar and who can reach each part — set a team's level
          per node, grant a whole team, or push a node's access to its children.
        </p>
        <div className="flex shrink-0 items-center gap-2">
          {allTeams.length > 0 ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm">
                  <SlidersHorizontal className="size-4" /> Teams
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-52">
                <DropdownMenuLabel>Show team columns</DropdownMenuLabel>
                <DropdownMenuSeparator />
                {allTeams.map((t) => (
                  <DropdownMenuCheckboxItem
                    key={t.id}
                    checked={!hidden.has(t.slug)}
                    onCheckedChange={(on) =>
                      setHidden((prev) => {
                        const next = new Set(prev);
                        if (on) next.delete(t.slug);
                        else next.add(t.slug);
                        return next;
                      })
                    }
                  >
                    {t.display_name}
                  </DropdownMenuCheckboxItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          ) : null}
          <Button
            size="sm"
            onClick={() =>
              setEditing({ mode: "create", parentId: null, sortOrder: tree.length })
            }
          >
            <Plus className="size-4" /> Add node
          </Button>
        </div>
      </header>

      {allTeams.length === 0 ? (
        <p className="rounded-lg border border-dashed border-border px-4 py-3 text-sm text-muted-foreground">
          No teams yet — create teams in the Teams tab, then set their navigation
          access here.
        </p>
      ) : null}

      <div
        className={`min-h-0 flex-1 overflow-auto rounded-xl border border-border ${
          mutating ? "opacity-90" : ""
        }`}
      >
        <table className="w-full border-collapse text-sm">
          <thead className="sticky top-0 z-10 bg-card">
            <tr className="border-b border-border">
              <th className="sticky left-0 z-20 min-w-64 bg-card px-3 py-2 text-left font-medium">
                Navigation
              </th>
              {teamList.map((team) => {
                const granted = rows.filter(
                  (r) => tierOf(r.node.id, team.slug) !== null,
                ).length;
                const allOn = granted === rows.length && rows.length > 0;
                return (
                  <th
                    key={team.id}
                    className="min-w-32 px-2 py-2 text-center align-bottom font-medium"
                  >
                    <div className="flex flex-col items-center gap-1">
                      <span className="max-w-32 truncate" title={team.display_name}>
                        {team.display_name}
                      </span>
                      <button
                        type="button"
                        onClick={() => toggleTeamColumn(team.slug)}
                        className="text-xs font-normal text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
                        title={allOn ? "Revoke all nodes" : "Grant all nodes (View)"}
                      >
                        {granted}/{rows.length} · {allOn ? "revoke all" : "grant all"}
                      </button>
                    </div>
                  </th>
                );
              })}
              <th className="w-px px-2 py-2" />
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={row.node.id}
                onDragOver={(e) => {
                  if (dragId) e.preventDefault();
                }}
                onDrop={() => onDrop(row)}
                className={`group border-b border-border/60 last:border-0 hover:bg-muted/40 ${
                  dragId === row.node.id ? "opacity-50" : ""
                }`}
              >
                <td className="sticky left-0 z-10 bg-card px-3 py-1.5 group-hover:bg-muted/40">
                  <NodeCell
                    row={row}
                    onDragStart={() => setDragId(row.node.id)}
                    onDragEnd={() => setDragId(null)}
                  />
                </td>
                {teamList.map((team) => (
                  <td key={team.id} className="px-2 py-1.5 text-center">
                    <TierPicker
                      tier={tierOf(row.node.id, team.slug)}
                      onChange={(t) => setTier(row.node.id, team.slug, t)}
                      onApplyToChildren={
                        row.node.children.length > 0
                          ? () => applyToChildren(row.node, team.slug)
                          : undefined
                      }
                    />
                  </td>
                ))}
                <td className="px-2 py-1.5">
                  <div className="flex items-center gap-0.5 opacity-0 transition group-hover:opacity-100">
                    <IconBtn
                      label="Add child"
                      onClick={() =>
                        setEditing({
                          mode: "create",
                          parentId: row.node.id,
                          sortOrder: row.node.children.length,
                        })
                      }
                    >
                      <Plus className="size-4" />
                    </IconBtn>
                    <IconBtn
                      label="Edit"
                      onClick={() => setEditing({ mode: "edit", node: row.node })}
                    >
                      <Pencil className="size-4" />
                    </IconBtn>
                    <IconBtn
                      label="Delete"
                      onClick={() => removeNode.mutate(row.node.id)}
                    >
                      <Trash2 className="size-4" />
                    </IconBtn>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {editing ? (
        <NavNodeFormDialog
          open
          initial={editing.mode === "edit" ? editing.node : undefined}
          onSubmit={onSubmit}
          onClose={() => setEditing(null)}
        />
      ) : null}
    </div>
  );
}

// The node label cell. Only the grip is draggable — a draggable <tr> swallows
// clicks on the controls inside it, which is why the cells felt dead before.
function NodeCell({
  row,
  onDragStart,
  onDragEnd,
}: {
  row: FlatRow;
  onDragStart: () => void;
  onDragEnd: () => void;
}) {
  const { node, depth } = row;
  const Icon =
    node.target.kind === "group"
      ? GROUP_ICON
      : node.target.kind === "route"
        ? ROUTE_META[node.target.route].icon
        : dashboardIcon(node.icon ?? "Activity");

  return (
    <div className="flex items-center gap-2" style={{ paddingLeft: depth * 18 }}>
      <span
        draggable
        onDragStart={onDragStart}
        onDragEnd={onDragEnd}
        className="cursor-grab text-muted-foreground/50 active:cursor-grabbing"
        title="Drag to reorder"
      >
        <GripVertical className="size-4 shrink-0" />
      </span>
      <Icon className="size-4 shrink-0 text-muted-foreground" />
      <span className="truncate font-medium">{node.title}</span>
      {node.target.kind === "group" ? (
        <Badge variant="secondary" className="ml-1 shrink-0">
          Group
        </Badge>
      ) : null}
    </div>
  );
}

// A None/View/Edit/Manage picker for one (node, team) cell. The trigger shows
// the current tier (or a muted "—" for no access); a cascade row appears when
// the node has children.
function TierPicker({
  tier,
  onChange,
  onApplyToChildren,
}: {
  tier: TierOrNone;
  onChange: (t: TierOrNone) => void;
  onApplyToChildren?: () => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant={tier ? "secondary" : "ghost"}
          size="sm"
          className={`h-7 min-w-20 justify-between gap-1 px-2 ${
            tier ? "" : "text-muted-foreground"
          }`}
        >
          {tier ? TIER_LABEL[tier] : "—"}
          <ChevronDown className="size-3 opacity-60" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="center" className="w-40">
        <DropdownMenuItem onClick={() => onChange(null)}>
          <span className="flex-1 text-muted-foreground">No access</span>
          {tier === null ? <Check className="size-4" /> : null}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        {TIERS.map((t) => (
          <DropdownMenuItem key={t} onClick={() => onChange(t)}>
            <span className="flex-1">{TIER_LABEL[t]}</span>
            {tier === t ? <Check className="size-4" /> : null}
          </DropdownMenuItem>
        ))}
        {onApplyToChildren ? (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={onApplyToChildren}>
              <CornerDownRight className="size-4" />
              <span className="flex-1">Apply to children</span>
            </DropdownMenuItem>
          </>
        ) : null}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function IconBtn({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className="size-7"
      title={label}
      aria-label={label}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}
