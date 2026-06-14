import { useState } from "react";
import { useParams } from "react-router-dom";

import type { WidgetLayout, WidgetType } from "@/data/types";
import { useUiStore } from "@/store/ui";
import { AddWidgetDialog } from "@/features/canvas/AddWidgetDialog";
import { DashboardGrid } from "@/features/canvas/DashboardGrid";
import { PanelEditor } from "@/features/canvas/PanelEditor/PanelEditor";
import { VizPalette } from "@/features/canvas/VizPalette";
import { DashboardToolbar } from "@/features/dashboards/DashboardToolbar";
import { useDashboard } from "@/features/dashboards/useDashboard";
import { useDuplicateWidget } from "@/features/dashboards/useDuplicateWidget";
import { useRemovePanel } from "@/features/dashboards/useRemovePanel";
import { useSaveLayout } from "@/features/dashboards/useSaveLayout";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import { useAutoRefresh } from "@/features/time/useAutoRefresh";
import { useTimeUrlSync } from "@/features/time/useTimeUrlSync";
import { VariableBar } from "@/features/variables/VariableBar";

// A single dashboard: loads it by slug, renders the toolbar (view/edit
// toggle) and the canvas. The page is a thin shell — data lives in
// `useDashboard`, layout in the canvas, edit-mode in the UI store. With no
// panels yet it shows an empty state rather than a blank grid (F0).
export function DashboardPage() {
  const { slug } = useParams();
  const { data: dashboard, isPending, isError, error } = useDashboard(slug);
  const editing = useUiStore((s) => s.editMode);
  const selectedId = useUiStore((s) => s.selectedWidgetId);
  const selectWidget = useUiStore((s) => s.selectWidget);
  const removePanel = useRemovePanel(slug ?? "");
  const saveLayout = useSaveLayout(slug ?? "");
  const duplicateWidget = useDuplicateWidget(slug ?? "");

  // Global time-range concerns (WS-01): restore/reflect the range + refresh
  // in the URL for shareable deep links, and drive the auto-refresh loop. Both
  // read the shared time store the panel queries resolve against.
  useTimeUrlSync();
  useAutoRefresh();

  // Drag-to-add state: the type currently being dragged from the palette,
  // and the draft (type + dropped cell) that opens the config dialog once
  // a tile is dropped on the canvas.
  const [dragType, setDragType] = useState<WidgetType | null>(null);
  const [draft, setDraft] = useState<{
    type: WidgetType;
    position: WidgetLayout;
  } | null>(null);

  if (isPending) return <Loading label="Loading dashboard…" />;
  if (isError) {
    return (
      <ErrorState
        title="Couldn't load this dashboard"
        message={error instanceof Error ? error.message : undefined}
      />
    );
  }

  // In edit mode the grid is always mounted (even with no panels) so the
  // palette has a drop target; in view mode an empty board shows the
  // empty state instead of a blank grid (F0).
  const showGrid = editing || dashboard.widgets.length > 0;

  // Resolve the selected panel from the (possibly refreshed) dashboard.
  // Selecting a panel (its gear button) opens the full-page Panel Editor
  // for it; the editor always edits live config off this resolved widget.
  // A stale id (panel removed/refreshed away) resolves to undefined, which
  // simply closes the editor and returns to the board.
  const selected = editing
    ? dashboard.widgets.find((w) => w.id === selectedId)
    : undefined;

  return (
    <div className="flex h-full flex-col gap-4">
      <DashboardToolbar dashboard={dashboard} />
      {/* The variable bar (WS-02): one control per dashboard variable, above
          the canvas. Renders nothing when the dashboard has no variables, so
          a plain board is visually unchanged. */}
      <VariableBar slug={slug ?? ""} />
      <div className="flex min-h-0 flex-1 gap-4">
        <div className="min-h-0 flex-1">
          {showGrid ? (
            <DashboardGrid
              dashboard={dashboard}
              editing={editing}
              dropType={dragType}
              selectedId={selected?.id ?? null}
              onRemovePanel={(id) => {
                if (id === selectedId) selectWidget(null);
                removePanel.mutate(id);
              }}
              onLayoutChange={(moved) => saveLayout.mutate(moved)}
              onSelectPanel={(id) => selectWidget(id)}
              onDuplicatePanel={(id) => {
                const source = dashboard.widgets.find((w) => w.id === id);
                if (source) {
                  duplicateWidget.mutate({
                    source,
                    widgets: dashboard.widgets,
                  });
                }
              }}
              onDropWidget={(position) => {
                if (!dragType) return;
                setDraft({ type: dragType, position: { ...position, w: 0, h: 0 } });
                setDragType(null);
              }}
            />
          ) : (
            <Empty
              title="This dashboard is empty"
              description="Switch to edit mode to add your first panel."
            />
          )}
        </div>
        {/* The edit-mode side slot is the add-panel palette. Editing a
            panel is a full-page takeover (the Panel Editor), not an inline
            form, so there's a single editor and the palette always stays
            available here for adding more panels. */}
        {editing ? <VizPalette onPick={setDragType} /> : null}
      </div>

      {/* Drop-driven config dialog: opens pre-seeded with the dropped type
          and cell. Distinct from the toolbar's button-driven dialog. */}
      <AddWidgetDialog
        dashboard={dashboard}
        open={draft !== null}
        onOpenChange={(open) => {
          if (!open) setDraft(null);
        }}
        initial={
          draft
            ? { type: draft.type, position: draft.position }
            : undefined
        }
      />

      {/* The full-page Panel Editor, open for the selected panel. Selecting
          a panel (its gear button) sets the selection and opens the editor;
          Back/Discard/Save clears the selection and returns to the board.
          Keyed on the panel id so switching panels re-seeds the draft. */}
      {selected ? (
        <PanelEditor
          key={selected.id}
          widget={selected}
          slug={slug ?? ""}
          onClose={() => selectWidget(null)}
        />
      ) : null}
    </div>
  );
}
