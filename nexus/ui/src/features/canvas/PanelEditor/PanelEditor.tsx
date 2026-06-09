import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
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

// The full-screen Panel Editor: a live preview beside a tabbed inspector
// (Query / Visualization / Field / Overrides / Legend & Axes / Transforms),
// modelled on Grafana's panel editor. Edits accumulate in a local draft
// (`useEditorDraft`) so the canvas keeps showing the saved panel until
// Save PATCHes the panel. Cancel discards by unmounting (the draft is seed
// from `widget` each open via the parent `key`). The preview re-renders
// from cached rows on any config change — only a query change refetches.
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

  return (
    <Dialog open onOpenChange={(open) => (!open ? onClose() : undefined)}>
      <DialogContent
        className="glass flex h-[90vh] max-h-[90vh] w-[95vw] max-w-[95vw] flex-col gap-3 p-4 sm:max-w-[95vw]"
        showCloseButton={false}
      >
        <DialogHeader className="flex-row items-center justify-between space-y-0">
          <DialogTitle>Edit panel</DialogTitle>
          <div className="flex items-center gap-2">
            <Button type="button" variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button type="button" onClick={save} disabled={update.isPending}>
              {update.isPending ? "Saving…" : "Save"}
            </Button>
          </div>
        </DialogHeader>

        {update.isError ? (
          <p role="alert" className="text-sm text-destructive">
            Couldn't save the panel.
          </p>
        ) : null}

        <div className="grid min-h-0 flex-1 grid-cols-[1fr_24rem] gap-4">
          <div className="min-h-0">
            <PreviewPane widget={draft.widget} />
          </div>

          <Tabs defaultValue="query" className="flex min-h-0 flex-col">
            <TabsList className="w-full justify-start overflow-x-auto">
              <TabsTrigger value="query" className="flex-none">
                Query
              </TabsTrigger>
              <TabsTrigger value="viz" className="flex-none">
                Visualization
              </TabsTrigger>
              <TabsTrigger value="field" className="flex-none">
                Field
              </TabsTrigger>
              <TabsTrigger value="overrides" className="flex-none">
                Overrides
              </TabsTrigger>
              <TabsTrigger value="legend" className="flex-none">
                Legend &amp; Axes
              </TabsTrigger>
              <TabsTrigger value="transforms" className="flex-none">
                Transforms
              </TabsTrigger>
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
      </DialogContent>
    </Dialog>
  );
}
