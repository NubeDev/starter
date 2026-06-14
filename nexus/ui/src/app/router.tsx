import { createBrowserRouter, Navigate } from "react-router-dom";

import { AppShell } from "@/app/AppShell";
import { DashboardsPage } from "@/features/dashboards/DashboardsPage";
import { DashboardPage } from "@/features/dashboards/DashboardPage";
import { FindingsPage } from "@/features/detections/FindingsPage";
import { DatasourcesPage } from "@/features/datasources/DatasourcesPage";
import { FlowsPage } from "@/features/flows/FlowsPage";
import { FlowEditorPage } from "@/features/flows/FlowEditorPage";
import { InsightsListPage } from "@/features/insights/InsightsListPage";
import { InsightsPage } from "@/features/insights/InsightsPage";
import { Explore } from "@/features/query-editor/Explore";
import { AgentsPage } from "@/features/agents";
import {
  AccessPage,
  AccessNavigationTab,
  AccessNavManagerTab,
  AccessTeamsTab,
  AccessMembersTab,
} from "@/features/access/AccessPage";
import { AuditPage } from "@/features/audit/AuditPage";
import { ExtensionsPage } from "@/features/extensions/ExtensionsPage";
import { ExtensionPage } from "@/features/extensions/ExtensionPage";
import { ExportPage } from "@/features/portability/ExportPage";
import { ImportPage } from "@/features/portability/ImportPage";

// Routing is a host-only concern (F4): the host owns the route table;
// extensions contribute to named slots, never to routes directly. The one
// `/x/:extId/*` route is still host-owned — it just mounts an extension's
// `main`-slot contribution and forwards the path tail as the slot route, so
// the extension dispatches its own sub-pages without registering routes.
// React Router — not TanStack Router — owns the table; only TanStack Query is
// the shared federation singleton.
export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <DashboardsPage /> },
      { path: "dashboards", element: <DashboardsPage /> },
      { path: "d/:slug", element: <DashboardPage /> },
      { path: "d/:slug/export", element: <ExportPage /> },
      { path: "d/:slug/import", element: <ImportPage /> },
      { path: "import", element: <ImportPage /> },
      { path: "explore", element: <Explore /> },
      { path: "datasources", element: <DatasourcesPage /> },
      { path: "flows", element: <FlowsPage /> },
      { path: "flows/:flowName", element: <FlowEditorPage /> },
      { path: "insights", element: <InsightsListPage /> },
      { path: "insights/workbench", element: <InsightsPage /> },
      { path: "detections", element: <FindingsPage /> },
      { path: "findings", element: <FindingsPage /> },
      { path: "agents", element: <AgentsPage /> },
      {
        path: "access",
        element: <AccessPage />,
        children: [
          // The four Access surfaces are real routes now (deep-linkable,
          // back/forward works). `/access` lands on Navigation.
          { index: true, element: <Navigate to="navigation" replace /> },
          { path: "navigation", element: <AccessNavigationTab /> },
          { path: "nav-manager", element: <AccessNavManagerTab /> },
          { path: "teams", element: <AccessTeamsTab /> },
          { path: "members", element: <AccessMembersTab /> },
        ],
      },
      { path: "audit", element: <AuditPage /> },
      { path: "extensions", element: <ExtensionsPage /> },
      // Per-extension page surface (rubix `/extensions/$extId/$` mechanism):
      // the extension's `main`-slot contribution renders here, with the path
      // tail forwarded as its route. Lives under `/x/` so it never collides
      // with the `/extensions` admin page above.
      { path: "x/:extId", element: <ExtensionPage /> },
      { path: "x/:extId/*", element: <ExtensionPage /> },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);
