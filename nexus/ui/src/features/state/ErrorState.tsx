import type { ReactNode } from "react";

// Shown when a query fails. The screen passes the error's message; a
// `retry` (typically the query's `refetch`) is offered when recovery is
// possible. Never falls back to fabricated data on error (F0).
export function ErrorState({
  title = "Something went wrong",
  message,
  retry,
}: {
  title?: string;
  message?: string;
  retry?: ReactNode;
}) {
  return (
    <div
      role="alert"
      className="flex h-full min-h-48 flex-col items-center justify-center gap-2 text-center"
    >
      <p className="text-sm font-medium text-destructive">{title}</p>
      {message ? (
        <p className="max-w-sm text-sm text-muted-foreground">{message}</p>
      ) : null}
      {retry}
    </div>
  );
}
