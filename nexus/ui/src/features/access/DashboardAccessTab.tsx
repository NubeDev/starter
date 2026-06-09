// Per-dashboard sharing — lists every dashboard with its share scope and grant
// count, each with a Manage button opening the permissions drawer (the same one
// the dashboard toolbar's Share button uses). Data from the authz instances
// endpoint, which lives outside `/api/v1`, so it uses the StarterClient method.

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Shield } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import type { ResourceInstance, ShareScope } from "@nube/starter-client-ts";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { Button } from "@nube/starter-ui-kit/components/button";
import { PageDetailDrawer } from "@nube/starter-ui-authz";

import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

const DASHBOARD_KIND = "nexus.dashboard";

const SCOPE_LABEL: Record<ShareScope, string> = {
  private: "Private",
  tenant: "Anyone in tenant",
  specific: "Specific people",
};

export function DashboardAccessTab({ tenantId }: { tenantId: string | null }) {
  const client = useStarterClient();
  const tenantParam = tenantId ?? undefined;
  const [selected, setSelected] = useState<ResourceInstance | null>(null);

  const instances = useQuery({
    queryKey: ["nexus", "access", "dashboards", tenantParam],
    queryFn: () =>
      client.listResourceInstances(DASHBOARD_KIND, { tenant: tenantParam }),
  });

  return (
    <div className="flex h-full flex-col gap-4">
      {instances.isPending ? (
        <Loading label="Loading dashboards…" />
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
          title="No dashboards"
          description="Create a dashboard to manage who can access it."
        />
      ) : (
        <ul className="flex flex-col gap-2">
          {instances.data.items.map((d) => (
            <AccessRow
              key={d.id}
              instance={d}
              onManage={() => setSelected(d)}
            />
          ))}
        </ul>
      )}

      <PageDetailDrawer
        page={selected}
        kind={DASHBOARD_KIND}
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
          <Shield className="size-4" />
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
