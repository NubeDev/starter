// Per-node navigation access (WS-13 §6) — the Access section's Navigation tab,
// replacing the old per-dashboard Dashboards tab. Lists every nav node with its
// share scope + grant count, each with a Manage button opening the same
// permissions drawer the rest of Access uses, scoped to `nexus.nav_node`.
//
// Granting "Building-1" here gives a user that mount — not every building that
// reuses the same template. A clone of DashboardAccessTab over the node kind.

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Compass } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import type { ResourceInstance, ShareScope } from "@nube/starter-client-ts";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { Button } from "@nube/starter-ui-kit/components/button";
import { PageDetailDrawer } from "@nube/starter-ui-authz";

import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

const NAV_NODE_KIND = "nexus.nav_node";

const SCOPE_LABEL: Record<ShareScope, string> = {
  private: "Private",
  tenant: "Anyone in tenant",
  specific: "Specific people",
};

export function NavAccessTab({ tenantId }: { tenantId: string | null }) {
  const client = useStarterClient();
  const tenantParam = tenantId ?? undefined;
  const [selected, setSelected] = useState<ResourceInstance | null>(null);

  const instances = useQuery({
    queryKey: ["nexus", "access", "nav", tenantParam],
    queryFn: () =>
      client.listResourceInstances(NAV_NODE_KIND, { tenant: tenantParam }),
  });

  return (
    <div className="flex h-full flex-col gap-4">
      {instances.isPending ? (
        <Loading label="Loading navigation…" />
      ) : instances.isError ? (
        <ErrorState
          message={
            instances.error instanceof Error
              ? instances.error.message
              : undefined
          }
        />
      ) : instances.data.items.length === 0 ? (
        <Empty
          title="No navigation nodes"
          description="Build the navigation tree to manage who can reach each page."
        />
      ) : (
        <ul className="flex flex-col gap-2">
          {instances.data.items.map((n) => (
            <AccessRow key={n.id} instance={n} onManage={() => setSelected(n)} />
          ))}
        </ul>
      )}

      <PageDetailDrawer
        page={selected}
        kind={NAV_NODE_KIND}
        tenantId={tenantId ?? ""}
        onClose={() => setSelected(null)}
      />
    </div>
  );
}

function AccessRow({
  instance,
  onManage,
}: {
  instance: ResourceInstance;
  onManage: () => void;
}) {
  const acl = instance.effective_acl;
  const count = acl.grants.length;
  return (
    <li className="flex items-center justify-between gap-3 rounded-2xl border border-border bg-card px-4 py-3">
      <div className="flex items-center gap-3">
        <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary">
          <Compass className="size-4" />
        </span>
        <div className="grid">
          <span className="text-sm font-medium">{instance.label}</span>
          <span className="text-xs text-muted-foreground">
            {SCOPE_LABEL[acl.share_scope]}
            {count > 0 ? ` · ${count} grant${count === 1 ? "" : "s"}` : ""}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Badge variant="secondary">{SCOPE_LABEL[acl.share_scope]}</Badge>
        <Button variant="outline" size="sm" onClick={onManage}>
          Manage
        </Button>
      </div>
    </li>
  );
}
