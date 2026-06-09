import { useState } from "react";
import { Outlet } from "react-router-dom";
import { Bell, Command, Menu, Plug, Search } from "lucide-react";
import { Sidebar } from "./Sidebar";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export function Layout() {
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <div className="flex h-screen w-full overflow-hidden bg-background">
      {/* Sidebar — fixed on desktop, drawer on mobile */}
      <div className="hidden lg:block">
        <Sidebar />
      </div>
      {mobileOpen && (
        <div className="fixed inset-0 z-40 lg:hidden">
          <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={() => setMobileOpen(false)} />
          <div className="absolute left-0 top-0 h-full animate-fade-in">
            <Sidebar onCollapse={() => setMobileOpen(false)} />
          </div>
        </div>
      )}

      {/* Main column */}
      <div className="flex min-w-0 flex-1 flex-col">
        {/* Topbar */}
        <header className="flex h-14 shrink-0 items-center justify-between gap-3 border-b border-white/[0.06] px-4 sm:px-6">
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="icon-sm" className="lg:hidden" onClick={() => setMobileOpen(true)}>
              <Menu className="h-4 w-4" />
            </Button>
            <button className="group flex items-center gap-2 rounded-lg border border-white/8 bg-white/[0.02] px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:border-white/15 hover:text-foreground cursor-pointer">
              <Search className="h-4 w-4" />
              <span className="hidden sm:inline">Search devices, metrics…</span>
              <kbd className="ml-2 hidden items-center gap-0.5 rounded border border-white/10 bg-white/5 px-1.5 py-0.5 text-[0.65rem] text-muted-foreground sm:flex">
                <Command className="h-3 w-3" /> K
              </kbd>
            </button>
          </div>

          <div className="flex items-center gap-1.5">
            <Badge variant="success" className="mr-1 hidden gap-1.5 md:flex">
              <Plug className="h-3 w-3" /> 1,284 devices
            </Badge>
            <Button variant="ghost" size="icon-sm" aria-label="Notifications" className="relative">
              <Bell className="h-4 w-4" />
              <span className="absolute right-1.5 top-1.5 h-1.5 w-1.5 rounded-full bg-destructive ring-2 ring-background" />
            </Button>
            <div className={cn("ml-1 h-7 w-7 rounded-full bg-gradient-to-br from-primary to-info ring-1 ring-white/10")} />
          </div>
        </header>

        {/* Routed content */}
        <main className="min-h-0 flex-1">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
