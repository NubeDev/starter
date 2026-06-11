import { Outlet, useLocation } from "react-router-dom";
import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar";

import { AppSidebar } from "@/app/AppSidebar";
import { Header } from "@/app/Header";
import { LayoutProvider } from "@/app/LayoutProvider";
import { getCookie } from "@/lib/cookie";
import { useUndoRedoShortcuts } from "@/features/audit/useUndoRedoShortcuts";

// App frame on the canonical shadcn sidebar: a floating, minimisable
// sidebar beside a `SidebarInset` content region with a glass header and
// the routed outlet. The aurora backdrop (index.css) sits behind it all.
// Route → header title. Kept here (not per-page) so the shell owns the
// chrome and pages stay focused on their content.
const TITLES: Record<string, string> = {
  "/": "Dashboards",
  "/dashboards": "Dashboards",
  "/explore": "Explore",
  "/datasources": "Datasources",
  "/flows": "Flows",
  "/insights": "Insights",
  "/insights/workbench": "Insights",
  "/detections": "Detections",
  "/findings": "Findings",
  "/agents": "Agents",
  "/access": "Access",
  "/audit": "Audit",
  "/extensions": "Extensions",
};

export function AppShell() {
  const { pathname } = useLocation();
  // Exact-match the static pages; the full-page flow editor (`/flows/<name>`)
  // keeps the "Flows" header so the chrome reads consistently.
  const title =
    TITLES[pathname] ?? (pathname.startsWith("/flows/") ? "Flows" : "Dashboards");
  const defaultOpen = getCookie("sidebar_state") !== "false";
  // Cmd/Ctrl+Z / +Shift+Z drive undo/redo across the app (WS-12).
  useUndoRedoShortcuts();
  return (
    <LayoutProvider>
      <SidebarProvider defaultOpen={defaultOpen}>
        <AppSidebar />
        <SidebarInset className="overflow-hidden rounded-xl md:m-2 md:ms-0">
          <Header title={title} />
          <main className="scrollbar-thin min-h-0 flex-1 overflow-y-auto p-6">
            <Outlet />
          </main>
        </SidebarInset>
      </SidebarProvider>
    </LayoutProvider>
  );
}
