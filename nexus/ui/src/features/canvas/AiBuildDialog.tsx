import { useState } from "react";
import { Sparkles } from "lucide-react";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Checkbox } from "@nube/starter-ui-kit/components/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
import { Label } from "@nube/starter-ui-kit/components/label";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import type { Widget } from "@/data/types";
import {
  resultDashboard,
  useAssist,
  type SuggestedPanel,
} from "@/features/ai/useAssist";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { nextSlot } from "@/features/canvas/placement";
import { useAddPanel } from "@/features/dashboards/useAddPanel";
import { toWidgetType, WIDGET_CATALOG } from "@/features/widgets/catalog";

// AI dashboard builder: the user describes the dashboard they want in plain
// English and the model (POST /ai/assist, task "dashboard") proposes a set of
// panels grounded on the picked datasource's real schema. Suggestions render
// as a checklist the user vets before any are committed — "Add selected" turns
// each checked SuggestedPanel into a draft Widget and adds it via the same
// `useAddPanel` path the manual Add-panel dialog uses, so layout/placement and
// the wire mapping stay in one place.
export function AiBuildDialog({
  slug,
  existingWidgets,
  open,
  onOpenChange,
}: {
  slug: string;
  existingWidgets: ReadonlyArray<Widget>;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const assist = useAssist();
  const add = useAddPanel(slug);
  const [datasourceId, setDatasourceId] = useState<string | undefined>();
  const [prompt, setPrompt] = useState("");
  // Which suggestions are checked, keyed by their index in the result list.
  const [checked, setChecked] = useState<Record<number, boolean>>({});
  const [addError, setAddError] = useState(false);

  const suggestion = assist.data ? resultDashboard(assist.data) : null;
  const panels = suggestion?.panels ?? [];

  const canSuggest = prompt.trim().length > 0 && !assist.isPending;
  const selectedCount = panels.filter((_, i) => checked[i]).length;

  const suggest = () => {
    if (!canSuggest) return;
    setAddError(false);
    assist.mutate(
      {
        task: "dashboard",
        prompt: prompt.trim(),
        datasource_id: datasourceId,
      },
      {
        // Default every returned panel to checked so "Add selected" is a
        // one-click accept; the user unchecks anything they don't want.
        onSuccess: (res) => {
          const next = resultDashboard(res);
          const all: Record<number, boolean> = {};
          (next?.panels ?? []).forEach((_, i) => {
            all[i] = true;
          });
          setChecked(all);
        },
      },
    );
  };

  // Turn one suggestion into a draft Widget, mirroring AddWidgetDialog: the
  // type comes from the catalog's viz→type mapping, the footprint from the
  // catalog default, and placement from `nextSlot` against the live widgets.
  const toWidget = (panel: SuggestedPanel): Widget => {
    const type = toWidgetType(panel.viz);
    const size = WIDGET_CATALOG[type].defaultSize;
    return {
      id: "",
      type,
      title: panel.title,
      layout: nextSlot(existingWidgets, size.w, size.h),
      config: {
        query: { datasourceId: datasourceId ?? "", sql: panel.sql },
        fields: {
          x: panel.x ?? undefined,
          series: [{ value: panel.value }],
        },
      },
    };
  };

  const addSelected = async () => {
    const picked = panels.filter((_, i) => checked[i]);
    if (picked.length === 0) return;
    setAddError(false);
    try {
      // Sequential so each draft places below the previous one — `nextSlot`
      // reads `existingWidgets`, which only refreshes after invalidation, so
      // building one at a time keeps panels from stacking on the same cell.
      const placed: Widget[] = [];
      for (const panel of picked) {
        const draft = toWidget(panel);
        draft.layout = nextSlot([...existingWidgets, ...placed], draft.layout.w, draft.layout.h);
        await add.mutateAsync(draft);
        placed.push(draft);
      }
      onOpenChange(false);
    } catch {
      setAddError(true);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass flex max-h-[85vh] max-w-lg flex-col overflow-hidden">
        <DialogHeader>
          <DialogTitle>AI dashboard builder</DialogTitle>
          <DialogDescription>
            Describe what you want and pick which suggested panels to add.
          </DialogDescription>
        </DialogHeader>
        {/* min-w-0 lets the long mono SQL previews truncate instead of forcing
            the dialog wider; overflow-y-auto keeps a long suggestion list
            scrollable rather than spilling past the dialog. */}
        <div className="flex min-w-0 flex-1 flex-col gap-4 overflow-y-auto">
          <div className="space-y-2">
            <Label>Datasource</Label>
            <DatasourcePicker value={datasourceId} onChange={setDatasourceId} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="ai-build-prompt">Describe the dashboard</Label>
            <Textarea
              id="ai-build-prompt"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="e.g. an overview of energy usage by site over the last 24h"
              rows={3}
              aria-label="Describe the dashboard in plain English"
              onKeyDown={(e) => {
                if ((e.metaKey || e.ctrlKey) && e.key === "Enter") suggest();
              }}
            />
          </div>
          <Button
            type="button"
            size="sm"
            className="w-full gap-2"
            disabled={!canSuggest}
            onClick={suggest}
          >
            <Sparkles className="size-4" />
            {assist.isPending ? "Thinking…" : "Suggest panels"}
          </Button>

          {assist.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {assist.error instanceof Error
                ? assist.error.message
                : "Couldn't suggest panels."}
            </p>
          ) : null}

          {/* Structured suggestions: a vettable checklist. */}
          {suggestion && panels.length > 0 ? (
            <ul className="min-w-0 space-y-2">
              {panels.map((panel, i) => (
                <li
                  key={i}
                  className="flex min-w-0 items-start gap-3 rounded-md border border-border bg-card p-3"
                >
                  <Checkbox
                    checked={checked[i] ?? false}
                    onCheckedChange={(c) =>
                      setChecked((prev) => ({ ...prev, [i]: c === true }))
                    }
                    aria-label={`Add ${panel.title}`}
                    className="mt-0.5"
                  />
                  <div className="min-w-0 flex-1 space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium text-foreground">
                        {panel.title}
                      </span>
                      <Badge variant="secondary" className="shrink-0">
                        {toWidgetType(panel.viz)}
                      </Badge>
                    </div>
                    <p
                      className="truncate font-mono text-xs text-muted-foreground"
                      title={panel.sql}
                    >
                      {panel.sql}
                    </p>
                  </div>
                </li>
              ))}
            </ul>
          ) : null}

          {/* The model replied but we couldn't parse a structured dashboard:
              fall back to the raw text so the user still sees the output. */}
          {assist.data && !suggestion ? (
            <div className="space-y-1">
              <p role="alert" className="text-sm text-destructive">
                Couldn't read the suggested panels.
              </p>
              {typeof assist.data.raw === "string" && assist.data.raw ? (
                <pre className="max-h-40 overflow-auto whitespace-pre-wrap rounded-md bg-muted p-3 text-xs text-muted-foreground">
                  {assist.data.raw}
                </pre>
              ) : null}
            </div>
          ) : null}

          {assist.data && suggestion && panels.length === 0 ? (
            <p className="text-sm text-muted-foreground">No panels suggested.</p>
          ) : null}

          {addError ? (
            <p role="alert" className="text-sm text-destructive">
              Couldn't add the selected panels.
            </p>
          ) : null}
        </div>
        <DialogFooter>
          <Button
            type="button"
            disabled={selectedCount === 0 || add.isPending}
            onClick={addSelected}
          >
            {add.isPending
              ? "Adding…"
              : `Add selected${selectedCount > 0 ? ` (${selectedCount})` : ""}`}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
