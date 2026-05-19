// Live preview pane. Renders a representative slice of the app — a
// sidebar replica, a sample card, typography specimen, chart palette —
// scoped to a single `<div>` with the editor's tokens stamped on it
// via `applyThemeToElement`. No iframe, no separate renderer; the
// preview is just primitive markup styled by the same CSS the rest of
// the app uses.

import { useEffect, useRef } from "react";

import { applyThemeToElement, useThemeEditorStore } from "@nube/starter-ui-core/theme-editor";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export interface LivePreviewProps {
  className?: string;
}

export function LivePreview({ className }: LivePreviewProps) {
  const styles = useThemeEditorStore((s) => s.styles);
  const mode = useThemeEditorStore((s) => s.mode);
  const navTitle = useThemeEditorStore((s) => s.shell.nav_title);
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!ref.current) return;
    applyThemeToElement(ref.current, styles[mode], mode);
  }, [styles, mode]);

  return (
    <div
      ref={ref}
      data-theme-preview
      className={cn(
        "flex h-full overflow-hidden rounded-lg border border-border bg-background text-foreground",
        className,
      )}
    >
      <aside className="flex w-48 flex-col gap-1 border-r border-sidebar-border bg-sidebar p-3 text-sidebar-foreground">
        <div className="mb-2 truncate text-sm font-semibold">{navTitle || "Preview"}</div>
        {["Home", "Inbox", "Settings"].map((label, i) => (
          <button
            key={label}
            type="button"
            className={cn(
              "rounded-md px-2 py-1.5 text-left text-sm",
              i === 0
                ? "bg-sidebar-accent text-sidebar-accent-foreground"
                : "hover:bg-sidebar-accent/60",
            )}
          >
            {label}
          </button>
        ))}
      </aside>

      <main className="flex flex-1 flex-col gap-4 overflow-auto p-4">
        <header className="flex items-baseline gap-3">
          <h1 className="text-xl font-semibold">Welcome back</h1>
          <Badge variant="secondary">Beta</Badge>
        </header>

        <Card>
          <CardHeader>
            <CardTitle>Sample card</CardTitle>
            <CardDescription>Primary, secondary, and destructive buttons render against your token map.</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <div className="flex gap-2">
              <Button>Primary</Button>
              <Button variant="secondary">Secondary</Button>
              <Button variant="destructive">Destructive</Button>
              <Button variant="outline">Outline</Button>
            </div>
            <Input placeholder="Type something…" />
            <p className="text-sm text-muted-foreground">
              Muted text inherits the foreground token via reduced opacity. Code spans use{" "}
              <code className="rounded bg-muted px-1 font-mono text-xs">font-mono</code>.
            </p>
          </CardContent>
        </Card>

        <div className="flex gap-2">
          {(["chart-1", "chart-2", "chart-3", "chart-4", "chart-5"] as const).map((key) => (
            <div
              key={key}
              className="h-12 flex-1 rounded"
              style={{ backgroundColor: styles[mode][key] }}
              title={key}
            />
          ))}
        </div>
      </main>
    </div>
  );
}
