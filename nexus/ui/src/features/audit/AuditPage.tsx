import { useState } from "react";
import { Input } from "@nube/starter-ui-kit/components/input";

import { usePrincipal } from "@/auth/usePrincipal";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import { useAudit } from "@/features/audit/useAudit";
import { ChangeList } from "@/features/audit/ChangeList";

// The admin audit screen: the tenant's change ledger, newest first, with a
// simple actor/kind filter. The server admin-gates the read (a non-admin gets
// 403); we also gate the screen here so a non-admin sees an honest notice
// rather than a failed request.
export function AuditPage() {
  const principal = usePrincipal();

  if (principal.isPending) return <Loading label="Loading…" />;
  if (principal.isError) {
    return (
      <ErrorState
        message={
          principal.error instanceof Error ? principal.error.message : undefined
        }
      />
    );
  }

  if (principal.data?.role !== "admin") {
    return (
      <ErrorState
        title="Admin only"
        message="The audit log is visible to tenant admins."
      />
    );
  }

  return <AuditLog />;
}

function AuditLog() {
  const [actorId, setActorId] = useState("");
  const [resourceKind, setResourceKind] = useState("");
  const query = useAudit({
    actor_id: actorId || undefined,
    resource_kind: resourceKind || undefined,
  });

  return (
    <div className="flex h-full flex-col gap-4">
      <header className="flex flex-col gap-1">
        <h1 className="text-lg font-semibold">Audit log</h1>
        <p className="text-sm text-muted-foreground">
          Every change in this tenant, newest first.
        </p>
      </header>
      <div className="flex flex-wrap gap-2">
        <Input
          value={actorId}
          onChange={(e) => setActorId(e.target.value)}
          placeholder="Filter by actor id"
          className="max-w-xs"
        />
        <Input
          value={resourceKind}
          onChange={(e) => setResourceKind(e.target.value)}
          placeholder="Filter by resource kind"
          className="max-w-xs"
        />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {query.isPending ? (
          <Loading label="Loading audit log…" />
        ) : query.isError ? (
          <ErrorState
            message={
              query.error instanceof Error ? query.error.message : undefined
            }
          />
        ) : (
          <ChangeList changes={query.data.items} />
        )}
      </div>
    </div>
  );
}
