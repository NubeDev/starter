import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { updatePanel } from "@/api/dashboards/updatePanel";
import { widgetToUpdatePanel } from "@/api/dashboards/panelAdapter";
import type { PanelDetail } from "@/api/types";
import type { Widget } from "@/data/types";
import { dashboardKey } from "@/features/dashboards/useDashboard";

// Persists an edited panel from the properties panel: a full
// `PATCH /panels/{id}` carrying title/sql/datasource/viz and the
// re-stashed layout+field-mapping. On success it invalidates the
// dashboard so the edited panel re-renders with its new config.
export function useUpdatePanel(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<PanelDetail, Error, Widget>({
    mutationFn: (widget) =>
      updatePanel(client, widget.id, widgetToUpdatePanel(widget)),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dashboardKey(slug) });
    },
  });
}
