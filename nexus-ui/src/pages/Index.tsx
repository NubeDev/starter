import { Navigate } from "react-router-dom";
import { useDashboards } from "@/providers/useStore";

// Land on the first (starred-first) dashboard.
export function Index() {
  const dashboards = useDashboards();
  const first =
    dashboards.find((d) => d.starred) ?? dashboards[0];

  if (!first) {
    return (
      <div className="grid h-full place-items-center text-center text-muted-foreground">
        <div>
          <div className="text-lg font-semibold text-foreground">No dashboards yet</div>
          <div className="text-sm">Create one from the sidebar to get started.</div>
        </div>
      </div>
    );
  }
  return <Navigate to={`/d/${first.slug}`} replace />;
}
