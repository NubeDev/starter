import { Empty } from "@/features/state/Empty";

// P0 landing. The dashboard list binding is codegen'd from the nexus-api
// OpenAPI contract; until that contract is published the screen renders
// an honest empty state rather than inventing rows (F0). When the client
// lands, this becomes a `useDashboards()` query rendering loading /
// empty / error / list — never fabricated data.
export function DashboardsLanding() {
  return (
    <Empty
      title="No dashboards yet"
      description="Dashboards load from nexus-api. Connect the backend to begin."
    />
  );
}
