// Shown while a query is in flight. Pure presentation; no timers, no
// data — a screen renders this from a query's `isPending`.
export function Loading({ label = "Loading…" }: { label?: string }) {
  return (
    <div
      role="status"
      aria-live="polite"
      className="flex h-full min-h-48 items-center justify-center text-sm text-muted-foreground"
    >
      {label}
    </div>
  );
}
