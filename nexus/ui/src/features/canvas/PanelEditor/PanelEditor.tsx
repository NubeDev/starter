import { useEffect } from "react";
import { ArrowLeft } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { ScrollArea } from "@nube/starter-ui-kit/components/scroll-area";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";

import type { Widget } from "@/data/types";
import { useUpdatePanel } from "@/features/dashboards/useUpdatePanel";
import { useEditorDraft } from "@/features/canvas/PanelEditor/useEditorDraft";
import { PreviewPane } from "@/features/canvas/PanelEditor/PreviewPane";
import { QueryTab } from "@/features/canvas/PanelEditor/QueryTab";
import { VizTab } from "@/features/canvas/PanelEditor/VizTab";
import { FieldTab } from "@/features/canvas/PanelEditor/FieldTab";
import { OverridesTab } from "@/features/canvas/PanelEditor/OverridesTab";
import { LegendAxesTab } from "@/features/canvas/PanelEditor/LegendAxesTab";
import { TransformsTab } from "@/features/canvas/PanelEditor/TransformsTab";

// The Panel Editor: a full-page takeover (à la Grafana) — a live preview
// beside a tabbed inspector (Query / Visualization / Field / Overrides /
// Legend & Axes / Transforms). It replaces the whole dashboard view while
// open, so editing a panel never depends on where the panel sits in a long
// scrolling board. Edits accumulate in a local draft (`useEditorDraft`) so
// the saved panel is untouched until Save PATCHes it; Discard (or Esc)
// throws the draft away by unmounting (re-seeded from `widget` on each open
// via the parent `key`). The preview re-renders from cached rows on any
// config change — only a query change refetches.
export function PanelEditor({
  widget,
  slug,
  onClose,
}: {
  widget: Widget;
  slug: string;
  onClose: () => void;
}) {
  const draft = useEditorDraft(widget);
  const update = useUpdatePanel(slug);

  function save() {
    update.mutate(draft.widget, { onSuccess: onClose });
  }

  // Esc discards and returns to the board, matching the Discard button.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex flex-col gap-3 bg-background p-4">
      <header className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="gap-2"
            onClick={onClose}
          >
            <ArrowLeft className="size-4" />
            Back to dashboard
          </Button>
          <h2 className="text-sm font-medium text-foreground">
            Edit panel · {draft.widget.title}
          </h2>
        </div>
        <div className="flex items-center gap-2">
          {update.isError ? (
            <p role="alert" className="text-sm text-destructive">
              Couldn't save the panel.
            </p>
          ) : null}
          <Button type="button" variant="ghost" onClick={onClose}>
            Discard
          </Button>
          <Button type="button" onClick={save} disabled={update.isPending}>
            {update.isPending ? "Saving…" : "Save"}
          </Button>
        </div>
      </header>

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_28rem] gap-4 xl:grid-cols-[minmax(0,1fr)_32rem]">
        <div className="min-h-0">
          <PreviewPane widget={draft.widget} />
        </div>

        <Tabs defaultValue="query" className="flex min-h-0 flex-col">
          <TabsList className="grid h-auto w-full grid-cols-3 gap-1">
            <TabsTrigger value="query">Query</TabsTrigger>
            <TabsTrigger value="viz">Visualize</TabsTrigger>
            <TabsTrigger value="field">Field</TabsTrigger>
            <TabsTrigger value="overrides">Overrides</TabsTrigger>
            <TabsTrigger value="legend">Legend</TabsTrigger>
            <TabsTrigger value="transforms">Transforms</TabsTrigger>
          </TabsList>
          <ScrollArea className="min-h-0 flex-1 pr-3">
            <TabsContent value="query">
              <QueryTab draft={draft} />
            </TabsContent>
            <TabsContent value="viz">
              <VizTab draft={draft} />
            </TabsContent>
            <TabsContent value="field">
              <FieldTab draft={draft} />
            </TabsContent>
            <TabsContent value="overrides">
              <OverridesTab draft={draft} />
            </TabsContent>
            <TabsContent value="legend">
              <LegendAxesTab draft={draft} />
            </TabsContent>
            <TabsContent value="transforms">
              <TransformsTab draft={draft} />
            </TabsContent>
          </ScrollArea>
        </Tabs>
      </div>
    </div>
  );
}
