import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { removePanel } from "@/api/dashboards/removePanel";
import { dashboardKey } from "@/features/dashboards/useDashboard";

// Removes a panel by id and refreshes the dashboard it belonged to.
export function useRemovePanel(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: (panelId) => removePanel(client, panelId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dashboardKey(slug) });
    },
  });
}
