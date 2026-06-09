import { Search } from "lucide-react";

// A thin glass topbar over the content region. The aurora backdrop shows
// through, so the bar carries a faint bottom border for separation
// rather than a solid fill — keeping the OLED depth the mock established.
export function Topbar({ title }: { title: string }) {
  return (
    <header className="sticky top-0 z-20 flex h-14 items-center justify-between gap-4 border-b border-border/60 bg-background/40 px-6 backdrop-blur-xl">
      <h1 className="text-balance text-lg font-semibold tracking-tight">
        {title}
      </h1>
      <label className="glass flex h-9 w-72 items-center gap-2 rounded-lg px-3 text-sm text-muted-foreground">
        <Search className="size-4 shrink-0" />
        <input
          type="search"
          placeholder="Search dashboards…"
          aria-label="Search dashboards"
          className="w-full bg-transparent outline-none placeholder:text-muted-foreground/70"
        />
      </label>
    </header>
  );
}
