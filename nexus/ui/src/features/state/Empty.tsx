import type { ReactNode } from "react";

// Shown when a query succeeds but returns nothing. Distinct from
// `Loading` and `ErrorState` so a screen can branch on the three
// outcomes a real query has — there is no fourth "fake data" branch.
export function Empty({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex h-full min-h-48 flex-col items-center justify-center gap-2 text-center">
      <p className="text-sm font-medium text-foreground">{title}</p>
      {description ? (
        <p className="max-w-sm text-sm text-muted-foreground">{description}</p>
      ) : null}
      {action}
    </div>
  );
}
