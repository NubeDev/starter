import { Outlet } from "react-router-dom";
import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar";

import { AppSidebar } from "@/app/AppSidebar";
import { Header } from "@/app/Header";
import { LayoutProvider } from "@/app/LayoutProvider";
import { getCookie } from "@/lib/cookie";

// App frame on the canonical shadcn sidebar: a floating, minimisable
// sidebar beside a `SidebarInset` content region with a glass header and
// the routed outlet. The aurora backdrop (index.css) sits behind it all.
export function AppShell() {
  const defaultOpen = getCookie("sidebar_state") !== "false";
  return (
    <LayoutProvider>
      <SidebarProvider defaultOpen={defaultOpen}>
        <AppSidebar />
        <SidebarInset>
          <Header title="Dashboards" />
          <main className="scrollbar-thin min-h-0 flex-1 overflow-y-auto p-6">
            <Outlet />
          </main>
        </SidebarInset>
      </SidebarProvider>
    </LayoutProvider>
  );
}
