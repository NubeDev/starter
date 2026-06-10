import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { addPanel } from "@/api/dashboards/addPanel";
import { createVariable } from "@/api/variables/create";
import type { DashboardExport } from "@/api/types";
import { dashboardKey } from "@/features/dashboards/useDashboard";
import { variablesKey } from "@/features/variables/useDashboardVariables";
import {
  exportPanelToCreate,
  exportVariableToCreate,
  filterExport,
  type PortableSelection,
} from "@/features/portability/model";

/** Outcome of importing a selection into an existing dashboard: how many of
 *  each kind landed, and any per-item failures (a name clash on a variable, a
 *  panel the server rejected) so the page can report partial success honestly
 *  rather than claiming a clean import. */
export interface ImportReport {
  panelsAdded: number;
  variablesAdded: number;
  failures: string[];
}

/**
 * Add the selected panels and variables from an export model into an existing
 * dashboard (identified by slug). Each panel is a `POST /panels`, each variable
 * a `POST /variables`; they run sequentially so a variable name-clash reports
 * cleanly without racing. Panels are added before variables so a variable that
 * a panel references exists in the same final state (order within each group
 * follows the export). Partial failures are collected, not thrown — importing 9
 * of 10 panels should keep the 9, not roll back.
 */
export function useImportIntoDashboard(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<
    ImportReport,
    Error,
    { model: DashboardExport; selection: PortableSelection }
  >({
    mutationFn: async ({ model, selection }) => {
      const chosen = filterExport(model, selection);
      const report: ImportReport = {
        panelsAdded: 0,
        variablesAdded: 0,
        failures: [],
      };

      for (const panel of chosen.panels) {
        try {
          await addPanel(client, slug, exportPanelToCreate(panel));
          report.panelsAdded += 1;
        } catch (e) {
          report.failures.push(
            `Widget “${panel.title || "Untitled"}”: ${messageOf(e)}`,
          );
        }
      }

      for (const variable of chosen.variables ?? []) {
        try {
          await createVariable(client, slug, exportVariableToCreate(variable));
          report.variablesAdded += 1;
        } catch (e) {
          report.failures.push(
            `Variable “$${variable.name}”: ${messageOf(e)}`,
          );
        }
      }

      return report;
    },
    onSuccess: () => {
      // Refresh the target dashboard + its variables so the imported items
      // appear on the canvas and in the variable bar.
      queryClient.invalidateQueries({ queryKey: dashboardKey(slug) });
      queryClient.invalidateQueries({ queryKey: variablesKey(slug) });
    },
  });
}

function messageOf(e: unknown): string {
  return e instanceof Error ? e.message : "failed";
}
