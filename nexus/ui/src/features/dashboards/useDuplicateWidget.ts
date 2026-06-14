import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { addPanel } from "@/api/dashboards/addPanel";
import { widgetToCreatePanel } from "@/api/dashboards/panelAdapter";
import type { PanelDetail } from "@/api/types";
import type { Widget } from "@/data/types";
import { nextSlot } from "@/features/canvas/placement";
import { dashboardKey } from "@/features/dashboards/useDashboard";

/** The mutation input: the panel to copy, plus the board's current widgets so
 *  the copy can be placed in a free slot below them. Passed at call-time (not
 *  hook-time) because the dashboard is only loaded inside the page body. */
export interface DuplicateInput {
  source: Widget;
  widgets: ReadonlyArray<Widget>;
}

/**
 * Duplicate a panel: add a copy of `source` to the same dashboard, carrying its
 * query, viz, and field mapping but with a "(copy)" title and a fresh grid slot
 * below the existing panels (via `nextSlot`) so it never lands on top of the
 * original. It's a plain `POST /panels` — the server mints the new id — so the
 * copy is independent of the source and records its own create in the changelog
 * (undoable like any add).
 */
export function useDuplicateWidget(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<PanelDetail, Error, DuplicateInput>({
    mutationFn: ({ source, widgets }) => {
      const copy: Widget = {
        ...source,
        // A new draft id; the server assigns the real one on create. Title
        // gets a "(copy)" suffix so the duplicate is distinguishable at a
        // glance. Position is the next free slot at the source's footprint.
        id: `${source.id}-copy`,
        title: copyTitle(source.title),
        layout: {
          ...nextSlot(widgets, source.layout.w, source.layout.h),
        },
      };
      return addPanel(client, slug, widgetToCreatePanel(copy));
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dashboardKey(slug) });
    },
  });
}

/** Suffix a title with "(copy)", avoiding "(copy) (copy)" by reusing an
 *  existing suffix — a second duplicate becomes "(copy 2)". Exported for test. */
export function copyTitle(title: string): string {
  const base = title || "Untitled panel";
  const match = base.match(/^(.*?)(?: \(copy(?: (\d+))?\))$/);
  if (match) {
    const n = match[2] ? Number(match[2]) + 1 : 2;
    return `${match[1]} (copy ${n})`;
  }
  return `${base} (copy)`;
}
