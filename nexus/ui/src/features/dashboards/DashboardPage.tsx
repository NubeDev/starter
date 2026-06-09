import { useParams } from "react-router-dom";

import { useUiStore } from "@/store/ui";
import { DashboardGrid } from "@/features/canvas/DashboardGrid";
import { DashboardToolbar } from "@/features/dashboards/DashboardToolbar";
import { useDashboard } from "@/features/dashboards/useDashboard";
import { useRemovePanel } from "@/features/dashboards/useRemovePanel";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// A single dashboard: loads it by slug, renders the toolbar (view/edit
// toggle) and the canvas. The page is a thin shell — data lives in
// `useDashboard`, layout in the canvas, edit-mode in the UI store. With no
// panels yet it shows an empty state rather than a blank grid (F0).
export function DashboardPage() {
  const { slug } = useParams();
  const { data: dashboard, isPending, isError, error } = useDashboard(slug);
  const editing = useUiStore((s) => s.editMode);
  const removePanel = useRemovePanel(slug ?? "");

  if (isPending) return <Loading label="Loading dashboard…" />;
  if (isError) {
    return (
      <ErrorState
        title="Couldn't load this dashboard"
        message={error instanceof Error ? error.message : undefined}
      />
    );
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <DashboardToolbar dashboard={dashboard} />
      {dashboard.widgets.length === 0 ? (
        <Empty
          title="This dashboard is empty"
          description="Switch to edit mode to add your first panel."
        />
      ) : (
        <div className="min-h-0 flex-1">
          <DashboardGrid
            dashboard={dashboard}
            editing={editing}
            onRemovePanel={(id) => removePanel.mutate(id)}
          />
        </div>
      )}
    </div>
  );
}
