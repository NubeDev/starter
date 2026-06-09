import { createBrowserRouter, Navigate } from "react-router-dom";

import { AppShell } from "@/app/AppShell";
import { DashboardsLanding } from "@/features/dashboards/DashboardsLanding";
import { DashboardPage } from "@/features/dashboards/DashboardPage";
import { Explore } from "@/features/query-editor/Explore";

// Routing is a host-only concern (F4): extensions contribute to named
// slots, never routes. React Router — not TanStack Router — owns the
// route table; only TanStack Query is the shared federation singleton.
export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <DashboardsLanding /> },
      { path: "d/:slug", element: <DashboardPage /> },
      { path: "explore", element: <Explore /> },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);
