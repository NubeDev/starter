import { Outlet } from "react-router-dom";

import { ExtensionSlot } from "@/extensions/ExtensionSlot";

// App frame: a glass sidebar (host nav + extension-contributed nav) and
// a routed content region. Extensions mount into the `sidebar-nav` and
// `sidebar` slots — the same slot ids the in-repo `com.nubeio.ce` remote
// contributes to, so it appears here unchanged once loaded.
export function AppShell() {
  return (
    <div className="flex min-h-screen">
      <aside className="glass flex w-64 shrink-0 flex-col gap-4 p-4">
        <div className="px-2 text-lg font-semibold tracking-tight">Nexus</div>
        <nav className="flex-1 overflow-y-auto">
          <ExtensionSlot id="sidebar-nav" />
        </nav>
        <ExtensionSlot id="sidebar" />
      </aside>
      <main className="flex-1 overflow-y-auto p-6">
        <Outlet />
      </main>
    </div>
  );
}
