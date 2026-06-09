import { createBrowserRouter, Navigate } from "react-router-dom";

import { AppShell } from "@/app/AppShell";
import { DashboardsPage } from "@/features/dashboards/DashboardsPage";
import { DashboardPage } from "@/features/dashboards/DashboardPage";
import { AlertsPage } from "@/features/alerts/AlertsPage";
import { DatasourcesPage } from "@/features/datasources/DatasourcesPage";
import { FlowsPage } from "@/features/flows/FlowsPage";
import { Explore } from "@/features/query-editor/Explore";
import { AgentsPage } from "@/features/agents";
import { AccessPage } from "@/features/access/AccessPage";
import { AuditPage } from "@/features/audit/AuditPage";

// Routing is a host-only concern (F4): extensions contribute to named
// slots, never routes. React Router — not TanStack Router — owns the
// route table; only TanStack Query is the shared federation singleton.
export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <DashboardsPage /> },
      { path: "dashboards", element: <DashboardsPage /> },
      { path: "d/:slug", element: <DashboardPage /> },
      { path: "explore", element: <Explore /> },
      { path: "datasources", element: <DatasourcesPage /> },
      { path: "flows", element: <FlowsPage /> },
      { path: "alerts", element: <AlertsPage /> },
      { path: "agents", element: <AgentsPage /> },
      { path: "access", element: <AccessPage /> },
      { path: "audit", element: <AuditPage /> },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);
