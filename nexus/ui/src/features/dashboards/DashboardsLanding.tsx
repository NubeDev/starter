import { usePrincipal } from "@/auth/usePrincipal";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Landing screen. Dashboard records aren't in the nexus-api contract yet
// (no `/dashboards` paths), so the list itself is still an honest empty
// state (F0). What we *can* show is the real signed-in principal from
// `GET /me`, which proves the auth + data path end-to-end.
export function DashboardsLanding() {
  const { data: principal, isPending, isError, error } = usePrincipal();

  if (isPending) return <Loading label="Loading your workspace…" />;
  if (isError) {
    return (
      <ErrorState
        title="Couldn't load your session"
        message={error instanceof Error ? error.message : undefined}
      />
    );
  }

  return (
    <div className="flex h-full flex-col gap-6">
      <div className="glass rounded-xl p-4">
        <p className="text-sm text-muted-foreground">Signed in as</p>
        <p className="text-lg font-semibold text-foreground">{principal.subject}</p>
        <p className="mt-1 text-sm text-muted-foreground">
          role <span className="text-foreground">{principal.role}</span>
          {principal.tenant_id ? (
            <>
              {" · tenant "}
              <span className="tabular text-foreground">{principal.tenant_id}</span>
            </>
          ) : null}
          {principal.teams.length > 0 ? (
            <>{" · teams "}{principal.teams.join(", ")}</>
          ) : null}
        </p>
      </div>
      <div className="min-h-0 flex-1">
        <Empty
          title="No dashboards yet"
          description="Dashboard records aren't exposed by nexus-api yet. Open Explore to run a query against a datasource."
        />
      </div>
    </div>
  );
}
