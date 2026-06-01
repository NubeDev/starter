// Team Permissions tab (G4) — inverse view of the Pages tab.
// For a given team subject, list every grant whose subject equals
// `team:<slug>`. Each row classifies how the access was granted:
// direct grant (revocable), tenant default (read-only), or legacy
// rule (read-only, advanced-mode hint).
//
// G3 shipped `useGrants(query)` and `useDeleteGrant()` — this
// panel is a thin consumer of both.

import { useMemo } from "react";
import {
  Badge,
  Button,
  TableCell,
  TableRow,
} from "@nube/starter-ui-kit";
import { X } from "lucide-react";
import type { Grant } from "@nube/starter-client-ts";
import { useDeleteGrant, useGrants } from "../hooks/index.js";
import { ActionsCell, DataTable, StateRow } from "./_common.js";

export interface TeamPermissionsTabProps {
  tenantId: string;
  teamSlug: string;
}

type RowClass = "direct" | "tenant-default" | "legacy";

interface ClassifiedGrant {
  grant: Grant;
  cls: RowClass;
}

/** Classify a grant row. The shipped `Grant` shape only contains
 * structured fields — every row served via `?subject=team:<slug>`
 * is by construction a direct, revocable grant. The two read-only
 * branches stay here as forward-compatible safety nets: if a
 * wildcard-subject row ever bleeds through it surfaces as a
 * "tenant default", and any future `source`/`condition`-tagged
 * row will fall into "legacy". */
function classify(g: Grant): RowClass {
  // Forward-compatible escape hatch: if the server ever adds a
  // `condition` or non-"grant" source onto the Grant type, the
  // cast below keeps the runtime check working without a compile
  // error today.
  const loose = g as Grant & {
    source?: string;
    condition?: string | null;
    role?: string;
  };
  if (loose.source && loose.source !== "grant") return "legacy";
  if (loose.condition != null) return "legacy";
  if (g.subject.kind === "wildcard") return "tenant-default";
  if (loose.role === "*") return "tenant-default";
  return "direct";
}

function tierLabel(tier: Grant["tier"]): string {
  // Server tiers already capitalized ("View" | "Edit" | "Manage")
  // but normalize defensively in case a lowercase variant slips in.
  const s = String(tier);
  return s.charAt(0).toUpperCase() + s.slice(1).toLowerCase();
}

function kindLabel(kind: string): string {
  switch (kind) {
    case "rubix.dashboard.page":
      return "Page";
    case "rubix.tool":
      return "Tool";
    default:
      return kind;
  }
}

function resourceCell(g: Grant) {
  if (g.resource_kind === "rubix.dashboard.page") {
    if (!g.resource_id || g.resource_id === "*") {
      return <span>All pages</span>;
    }
    return (
      <span>
        <span aria-hidden>📄 </span>
        <code className="text-sm">{g.resource_id}</code>
      </span>
    );
  }
  if (!g.resource_id || g.resource_id === "*") {
    return <span className="text-muted-foreground">All</span>;
  }
  return <code className="text-sm">{g.resource_id}</code>;
}

function GrantedByCell({ cls }: { cls: RowClass }) {
  if (cls === "direct") {
    return (
      <Badge variant="secondary">direct grant</Badge>
    );
  }
  if (cls === "tenant-default") {
    return (
      <span className="flex flex-wrap items-center gap-2">
        <Badge variant="outline">tenant default</Badge>
        <a
          href="#rules"
          className="text-xs text-muted-foreground underline-offset-2 hover:underline"
          title="Edit in Advanced mode"
        >
          Advanced mode
        </a>
      </span>
    );
  }
  return (
    <span className="flex flex-wrap items-center gap-2">
      <Badge
        variant="outline"
        className="border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300"
      >
        legacy rule
      </Badge>
      <a
        href="#rules"
        className="text-xs text-muted-foreground underline-offset-2 hover:underline"
        title="Edit in Advanced mode"
      >
        Advanced mode
      </a>
    </span>
  );
}

export function TeamPermissionsTab({
  tenantId,
  teamSlug,
}: TeamPermissionsTabProps) {
  const subject = `team:${teamSlug}`;
  const query = useGrants({ subject, tenant_id: tenantId });
  const del = useDeleteGrant();

  const items = query.data?.grants ?? [];
  const classified = useMemo<ClassifiedGrant[]>(
    () => items.map((g) => ({ grant: g, cls: classify(g) })),
    [items],
  );

  const headers = ["Resource", "Kind", "Tier", "Granted by", "Actions"];

  const rows = useMemo(
    () =>
      classified.map(({ grant, cls }) => (
        <TableRow key={grant.id}>
          <TableCell>{resourceCell(grant)}</TableCell>
          <TableCell>{kindLabel(grant.resource_kind)}</TableCell>
          <TableCell>{tierLabel(grant.tier)}</TableCell>
          <TableCell>
            <GrantedByCell cls={cls} />
          </TableCell>
          <ActionsCell>
            {cls === "direct" ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={del.isPending}
                onClick={() => del.mutate(grant.id)}
                aria-label={`Revoke ${grant.resource_kind} ${grant.resource_id ?? ""}`.trim()}
              >
                <X className="mr-1 size-3" aria-hidden /> Revoke
              </Button>
            ) : (
              <span className="text-xs text-muted-foreground">—</span>
            )}
          </ActionsCell>
        </TableRow>
      )),
    [classified, del],
  );

  if (query.isLoading) {
    return <StateRow variant="loading">Loading permissions…</StateRow>;
  }
  if (query.error) {
    return <StateRow variant="error">{query.error.message}</StateRow>;
  }
  if (items.length === 0) {
    return (
      <StateRow variant="empty">
        No permissions granted to this team yet.
      </StateRow>
    );
  }

  return (
    <section className="grid gap-4">
      <DataTable headers={headers} rows={rows} label="Team permissions" />
    </section>
  );
}
