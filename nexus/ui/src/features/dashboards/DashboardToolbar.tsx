import { useState, type ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import {
  Braces,
  Check,
  Download,
  Eye,
  MoreHorizontal,
  Pencil,
  Plus,
  Redo2,
  Save,
  Share2,
  Sparkles,
  Undo2,
  Upload,
} from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Separator } from "@nube/starter-ui-kit/components/separator";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@nube/starter-ui-kit/components/dropdown-menu";

import type { Dashboard } from "@/data/types";
import { useUiStore } from "@/store/ui";
import { useRedo, useUndo } from "@/features/audit/useUndoRedo";
import { useSaveDashboard } from "@/features/dashboards/useSaveDashboard";
import { AddWidgetDialog } from "@/features/canvas/AddWidgetDialog";
import { AiBuildDialog } from "@/features/canvas/AiBuildDialog";
import { ShareDashboardDialog } from "@/features/dashboards/ShareDashboardDialog";
import { DashboardStarButton } from "@/features/dashboards/DashboardStarButton";
import { DashboardTagsButton } from "@/features/dashboards/DashboardTagsButton";
import { TimeRangePicker } from "@/features/time/TimeRangePicker";
import { RefreshControl } from "@/features/time/RefreshControl";
import { VariableEditorDialog } from "@/features/variables/VariableEditorDialog";

// The dashboard's header strip. The controls are organised into clear groups
// rather than a flat row of equal-weight pills, because edit mode otherwise
// crowds a dozen buttons together with no hierarchy:
//
//   [ viewing ] | [ history ] | [ add content ] | [ ⋯ ] | [ commit ]
//
// Viewing controls (time range + refresh) are always shown. The history (undo/
// redo) and add-content (add panel / AI / variables) clusters appear only in
// edit mode and use icon-only buttons with tooltips for the secondary actions —
// only the two things a user reaches for most (Add panel, Save) keep a label.
// Occasional, non-editing actions (Export / Import / Share) fold into one "⋯"
// overflow menu so they don't compete with the editing tools.
export function DashboardToolbar({ dashboard }: { dashboard: Dashboard }) {
  const navigate = useNavigate();
  const editing = useUiStore((s) => s.editMode);
  const toggle = useUiStore((s) => s.toggleEditMode);
  const setEditMode = useUiStore((s) => s.setEditMode);
  const [adding, setAdding] = useState(false);
  const [aiBuilding, setAiBuilding] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [editingVars, setEditingVars] = useState(false);
  // Undo/redo target the caller's most recent change group (per-actor, bodyless)
  // and invalidate the whole nexus query tree on success, so the canvas
  // refreshes with the reverted/re-applied state. The same hooks back the
  // global Cmd/Ctrl+Z shortcut (AppShell); these buttons make the action
  // discoverable while editing.
  const undo = useUndo();
  const redo = useRedo();
  // Explicit Save: the dashboard autosaves, so this flushes pending writes and
  // confirms — see useSaveDashboard. "Done" exits edit mode (a separate idea);
  // Save lets a user commit without leaving the editor.
  const { save, state: saveState, isBusy: saving } = useSaveDashboard(
    dashboard.slug,
  );

  return (
    <div className="flex items-center justify-between gap-3">
      <h2 className="text-balance text-base font-semibold tracking-tight">
        {dashboard.name}
      </h2>
      <div className="flex items-center gap-1.5">
        {/* Personal / metadata — always present. Star is per-user (the caller's
            own favourites); tags reuse the generic tagging system. */}
        <DashboardStarButton dashboardId={dashboard.id} />
        <DashboardTagsButton dashboardId={dashboard.id} />
        <ToolbarDivider />
        {/* Viewing context — always present. */}
        <TimeRangePicker />
        <RefreshControl />

        {editing ? (
          <>
            <ToolbarDivider />
            {/* History */}
            <IconButton
              label="Undo (Ctrl/Cmd+Z)"
              onClick={() => undo.mutate()}
              disabled={undo.isPending}
            >
              <Undo2 className="size-4" />
            </IconButton>
            <IconButton
              label="Redo (Ctrl/Cmd+Shift+Z)"
              onClick={() => redo.mutate()}
              disabled={redo.isPending}
            >
              <Redo2 className="size-4" />
            </IconButton>

            <ToolbarDivider />
            {/* Add content — Add panel keeps its label (the primary edit
                action); AI build and Variables are icon-only. */}
            <Button
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => setAdding(true)}
            >
              <Plus className="size-4" />
              Add panel
            </Button>
            <IconButton label="AI build" onClick={() => setAiBuilding(true)}>
              <Sparkles className="size-4" />
            </IconButton>
            <IconButton
              label="Variables"
              onClick={() => setEditingVars(true)}
            >
              <Braces className="size-4" />
            </IconButton>
          </>
        ) : null}

        <ToolbarDivider />
        {/* Occasional actions: Export / Import / Share behind one overflow
            menu, so they don't crowd the editing tools. */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5"
              aria-label="More actions"
              title="More actions"
            >
              <MoreHorizontal className="size-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-44">
            <DropdownMenuItem
              onClick={() => navigate(`/d/${dashboard.slug}/export`)}
            >
              <Download className="size-4" />
              Export…
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => navigate(`/d/${dashboard.slug}/import`)}
            >
              <Upload className="size-4" />
              Import…
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setSharing(true)}>
              <Share2 className="size-4" />
              Share…
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        {/* Commit / mode */}
        {editing ? (
          <Button
            variant="default"
            size="sm"
            className="gap-2"
            onClick={() => save()}
            disabled={saving}
            title="Save changes"
          >
            {saveState === "saved" ? (
              <Check className="size-4" />
            ) : (
              <Save className="size-4" />
            )}
            {saveState === "saved"
              ? "Saved"
              : saveState === "saving"
                ? "Saving…"
                : "Save"}
          </Button>
        ) : null}
        <Button
          variant={editing ? "outline" : "default"}
          size="sm"
          className="gap-2"
          onClick={() => {
            // Leaving edit mode: flush a final save so "Done" never drops an
            // in-flight change, then exit. Entering needs no save.
            if (editing) {
              void save();
              setEditMode(false);
            } else {
              toggle();
            }
          }}
        >
          {editing ? <Eye className="size-4" /> : <Pencil className="size-4" />}
          {editing ? "Done" : "Edit"}
        </Button>
      </div>

      <AddWidgetDialog
        dashboard={dashboard}
        open={adding}
        onOpenChange={setAdding}
      />
      <AiBuildDialog
        slug={dashboard.slug}
        existingWidgets={dashboard.widgets}
        open={aiBuilding}
        onOpenChange={setAiBuilding}
      />
      <ShareDashboardDialog
        dashboardId={dashboard.id}
        open={sharing}
        onOpenChange={setSharing}
      />
      <VariableEditorDialog
        slug={dashboard.slug}
        open={editingVars}
        onOpenChange={setEditingVars}
      />
    </div>
  );
}

// A square, icon-only toolbar button with an accessible label (shown as a
// native tooltip and exposed to assistive tech). Used for the secondary edit
// actions so the bar stays compact and the labelled buttons (Add panel, Save)
// read as primary.
function IconButton({
  label,
  onClick,
  disabled,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  children: ReactNode;
}) {
  return (
    <Button
      variant="outline"
      size="sm"
      className="px-2"
      onClick={onClick}
      disabled={disabled}
      title={label}
      aria-label={label}
    >
      {children}
    </Button>
  );
}

// A thin vertical rule separating control groups.
function ToolbarDivider() {
  return <Separator orientation="vertical" className="mx-0.5 h-6" />;
}
