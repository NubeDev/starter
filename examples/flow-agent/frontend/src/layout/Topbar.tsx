import { useTheme } from "@nube/starter-ui-kit";
import { Button } from "@nube/starter-ui-kit";

export function Topbar() {
  const { theme, setTheme } = useTheme();

  function cycle() {
    setTheme(theme === "light" ? "dark" : theme === "dark" ? "system" : "light");
  }

  return (
    <header className="sticky top-0 z-10 flex h-14 items-center justify-between border-b border-border/60 bg-background/70 px-4 backdrop-blur">
      <div className="flex items-center gap-2">
        <div className="size-6 rounded-md bg-primary/90" aria-hidden />
        <span className="text-sm font-semibold tracking-tight">flow-agent</span>
      </div>
      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          size="sm"
          onClick={cycle}
          className="text-xs capitalize"
        >
          {theme}
        </Button>
      </div>
    </header>
  );
}
