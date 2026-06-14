import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import {
  duplicateDashboard,
  exportDashboard,
  importDashboard,
} from "@/api/dashboards/export";
import type { DashboardExport, DashboardSummary } from "@/api/types";
import { DASHBOARDS_KEY } from "@/features/dashboards/useDashboards";

// Fetch a dashboard's portable JSON model on demand (the caller saves it to a
// file). A mutation, not a query, because it is a user-triggered one-shot.
export function useExportDashboard() {
  const client = useStarterClient();
  return useMutation<DashboardExport, Error, string>({
    mutationFn: (slug) => exportDashboard(client, slug),
  });
}

// Re-create a dashboard from a previously exported model. Invalidates the list
// so the imported dashboard appears.
export function useImportDashboard() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<DashboardSummary, Error, DashboardExport>({
    mutationFn: (model) => importDashboard(client, model),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: DASHBOARDS_KEY });
    },
  });
}

// Duplicate a dashboard with its panels and variables under a fresh id.
// Invalidates the list so the copy appears.
export function useDuplicateDashboard() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<DashboardSummary, Error, string>({
    mutationFn: (slug) => duplicateDashboard(client, slug),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: DASHBOARDS_KEY });
    },
  });
}
