import { Search } from "lucide-react";
import { Separator } from "@nube/starter-ui-kit/components/separator";

import { SidebarTrigger } from "@/components/ui/sidebar";
import { LayoutSwitcher } from "@/app/LayoutSwitcher";

// Content header: the sidebar minimise trigger, the page title, a
// search field, and the layout switcher. Sticky and glassy so the aurora
// backdrop reads through as the content scrolls beneath it.
export function Header({ title }: { title: string }) {
  return (
    <header className="sticky top-0 z-20 flex h-14 items-center gap-3 border-b border-border/60 bg-background/40 px-4 backdrop-blur-xl">
      <SidebarTrigger variant="outline" className="size-8" />
      <Separator orientation="vertical" className="h-6" />
      <h1 className="text-balance text-lg font-semibold tracking-tight">
        {title}
      </h1>
      <div className="ms-auto flex items-center gap-2">
        <label className="glass flex h-9 w-64 items-center gap-2 rounded-lg px-3 text-sm text-muted-foreground max-md:hidden">
          <Search className="size-4 shrink-0" />
          <input
            type="search"
            placeholder="Search dashboards…"
            aria-label="Search dashboards"
            className="w-full bg-transparent outline-none placeholder:text-muted-foreground/70"
          />
        </label>
        <LayoutSwitcher />
      </div>
    </header>
  );
}
