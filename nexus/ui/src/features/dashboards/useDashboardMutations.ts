import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { removeDashboard } from "@/api/dashboards/remove";
import { updateDashboard } from "@/api/dashboards/update";
import type { DashboardSummary, UpdateDashboardRequest } from "@/api/types";
import { DASHBOARDS_KEY } from "@/features/dashboards/useDashboards";

// Rename / re-slug a dashboard. On success it invalidates the sidebar +
// management list so the new name/slug shows everywhere.
export function useUpdateDashboard() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<
    DashboardSummary,
    Error,
    { slug: string; patch: UpdateDashboardRequest }
  >({
    mutationFn: ({ slug, patch }) => updateDashboard(client, slug, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: DASHBOARDS_KEY });
    },
  });
}

// Delete a dashboard (and its panels, server-side). Invalidates the list so
// the row disappears.
export function useDeleteDashboard() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: (slug) => removeDashboard(client, slug),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: DASHBOARDS_KEY });
    },
  });
}
