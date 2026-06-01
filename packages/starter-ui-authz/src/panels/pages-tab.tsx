// Pages tab — operator-first view of dashboard pages with their
// effective access. Read-only in G2; the drawer's mutations land
// in G3.

import { useEffect, useMemo, useState } from "react";
import {
  Badge,
  Button,
  Input,
  TableCell,
  TableRow,
} from "@nube/starter-ui-kit";
import { Globe, Lock, Search, ShieldAlert } from "lucide-react";
import type {
  EffectiveAcl,
  ResourceInstance,
} from "@nube/starter-client-ts";
import { useResourceInstances } from "../hooks/index.js";
import { DataTable, StateRow } from "./_common.js";
import { PageDetailDrawer, subjectLabel } from "./page-detail-drawer.js";

const PAGE_KIND = "rubix.dashboard.page";

export interface PagesTabProps {
  tenantId: string;
}

export function PagesTab({ tenantId }: PagesTabProps) {
  const [searchInput, setSearchInput] = useState("");
  const [search, setSearch] = useState("");
  const [cursor, setCursor] = useState<string | undefined>(undefined);
  const [selected, setSelected] = useState<ResourceInstance | null>(null);

  // 300ms debounce on the search box → server query.
  useEffect(() => {
    const id = setTimeout(() => {
      setSearch(searchInput.trim());
      setCursor(undefined);
    }, 300);
    return () => clearTimeout(id);
  }, [searchInput]);

  const query = useResourceInstances(PAGE_KIND, {
    tenant: tenantId,
    search: search || undefined,
    cursor,
  });

  const items = query.data?.items ?? [];
  const headers = ["Page", "Access", "Owner", "Updated"];

  const rows = useMemo(
    () =>
      items.map((item) => (
        <TableRow
          key={item.id}
          className="cursor-pointer"
          onClick={() => setSelected(item)}
        >
          <TableCell>
            <div className="font-medium">{item.label}</div>
            <code className="text-xs text-muted-foreground">{item.id}</code>
          </TableCell>
          <TableCell>
            <AccessCell acl={item.effective_acl} />
          </TableCell>
          <TableCell>
            {item.owner ? subjectLabel(item.owner) : (
              <span className="text-muted-foreground">—</span>
            )}
          </TableCell>
          <TableCell>
            {item.updated_at ? (
              <span className="text-sm text-muted-foreground">
                {item.updated_at}
              </span>
            ) : (
              <span className="text-muted-foreground">—</span>
            )}
          </TableCell>
        </TableRow>
      )),
    [items],
  );

  return (
    <section className="grid gap-4">
      <div className="flex items-center gap-2">
        <div className="relative max-w-sm flex-1">
          <Search
            className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden
          />
          <Input
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            placeholder="Search pages…"
            aria-label="Search pages"
            className="pl-9"
          />
        </div>
      </div>

      {query.isLoading ? (
        <StateRow variant="loading">Loading pages…</StateRow>
      ) : query.error ? (
        <StateRow variant="error">{query.error.message}</StateRow>
      ) : items.length === 0 ? (
        <StateRow variant="empty">
          {search ? "No pages match that search." : "No pages in this tenant yet."}
        </StateRow>
      ) : (
        <DataTable headers={headers} rows={rows} label="Pages" />
      )}

      {query.data?.next_cursor ? (
        <div className="flex justify-end">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setCursor(query.data?.next_cursor)}
          >
            Load more
          </Button>
        </div>
      ) : null}

      <PageDetailDrawer
        page={selected}
        tenantId={tenantId}
        onClose={() => setSelected(null)}
      />
    </section>
  );
}

function AccessCell({ acl }: { acl: EffectiveAcl }) {
  const legacy = acl.has_legacy_rules ? (
    <Badge
      variant="outline"
      className="border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300"
    >
      <ShieldAlert className="mr-1 size-3" aria-hidden /> Legacy rules
    </Badge>
  ) : null;

  if (acl.share_scope === "private") {
    return (
      <div className="flex flex-wrap items-center gap-1">
        <Badge variant="secondary">
          <Lock className="mr-1 size-3" aria-hidden /> Private
        </Badge>
        {legacy}
      </div>
    );
  }
  if (acl.share_scope === "tenant") {
    return (
      <div className="flex flex-wrap items-center gap-1">
        <Badge variant="secondary">
          <Globe className="mr-1 size-3" aria-hidden /> Tenant (view)
        </Badge>
        {legacy}
      </div>
    );
  }
  // specific
  const first = acl.grants[0];
  const rest = acl.grants.length - 1;
  return (
    <div className="flex flex-wrap items-center gap-1">
      {first ? (
        <Badge variant="secondary">
          {subjectLabel(first.subject)} ({first.tier})
        </Badge>
      ) : (
        <Badge variant="outline">No grants</Badge>
      )}
      {rest > 0 ? (
        <Badge variant="outline">+{rest} more</Badge>
      ) : null}
      {legacy}
    </div>
  );
}
