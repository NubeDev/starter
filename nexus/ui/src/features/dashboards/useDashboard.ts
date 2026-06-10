import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { getDashboard } from "@/api/dashboards/get";
import { panelToWidget } from "@/api/dashboards/panelAdapter";
import type { Dashboard } from "@/data/types";

export const dashboardKey = (slug: string) =>
  ["nexus", "dashboard", slug] as const;

// One dashboard by slug, with its panels adapted to the UI's `Widget`
// model (`panelToWidget`). Returns the UI `Dashboard` shape so the canvas
// consumes it directly. The query result carries loading/error so the page
// renders honest states (F0).
export function useDashboard(
  slug: string | undefined,
): UseQueryResult<Dashboard> {
  const client = useStarterClient();
  return useQuery({
    queryKey: dashboardKey(slug ?? ""),
    enabled: !!slug,
    queryFn: async () => {
      const detail = await getDashboard(client, slug!);
      const dashboard: Dashboard = {
        id: detail.id,
        name: detail.name,
        slug: detail.slug,
        // Real per-dashboard appearance now comes from the backend.
        icon: detail.icon,
        accent: detail.accent,
        widgets: detail.panels.map(panelToWidget),
        updatedAt: "",
      };
      return dashboard;
    },
  });
}
