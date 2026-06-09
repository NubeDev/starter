import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { addPanel } from "@/api/dashboards/addPanel";
import { widgetToCreatePanel } from "@/api/dashboards/panelAdapter";
import type { PanelDetail } from "@/api/types";
import type { Widget } from "@/data/types";
import { dashboardKey } from "@/features/dashboards/useDashboard";

// Adds a panel to a dashboard from a draft `Widget` (the dialog builds the
// draft; this packs it to the wire shape via `widgetToCreatePanel`). On
// success it invalidates the dashboard so the new panel appears.
export function useAddPanel(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<PanelDetail, Error, Widget>({
    mutationFn: (draft) => addPanel(client, slug, widgetToCreatePanel(draft)),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dashboardKey(slug) });
    },
  });
}
