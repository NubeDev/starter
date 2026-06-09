import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { updatePanel } from "@/api/dashboards/updatePanel";
import { widgetToLayoutPatch } from "@/api/dashboards/panelAdapter";
import type { Widget } from "@/data/types";
import { dashboardKey } from "@/features/dashboards/useDashboard";

// Persists a canvas layout change: one `PATCH /panels/{id}` per moved
// widget (the diff is already computed upstream by `applyGridLayout`, so
// only changed panels are passed). Refreshes the dashboard once after the
// batch so the saved positions are authoritative. Failures surface via the
// mutation; the in-session move still stands until the next load.
export function useSaveLayout(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, Widget[]>({
    mutationFn: async (moved) => {
      await Promise.all(
        moved.map((w) => updatePanel(client, w.id, widgetToLayoutPatch(w))),
      );
    },
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: dashboardKey(slug) }),
  });
}
