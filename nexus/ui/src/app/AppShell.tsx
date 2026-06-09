import { Outlet } from "react-router-dom";
import { SidebarProvider } from "@nube/starter-ui-kit/components/sidebar";

import { AppSidebar } from "@/app/AppSidebar";
import { Topbar } from "@/app/Topbar";

// App frame: the kit's collapsible sidebar beside a content region with
// a glass topbar and the routed outlet. The aurora backdrop (index.css)
// sits behind everything; surfaces are glass so the depth reads through.
export function AppShell() {
  return (
    <SidebarProvider>
      <div className="flex min-h-screen w-full">
        <AppSidebar />
        <div className="flex min-w-0 flex-1 flex-col">
          <Topbar title="Dashboards" />
          <main className="scrollbar-thin min-h-0 flex-1 overflow-y-auto p-6">
            <Outlet />
          </main>
        </div>
      </div>
    </SidebarProvider>
  );
}
