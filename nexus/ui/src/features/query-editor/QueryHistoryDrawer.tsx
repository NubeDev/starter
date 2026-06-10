import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { History, Play, Star, Pencil } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";

import { listQueryHistory } from "@/api/query-history/list";
import { starQueryHistory } from "@/api/query-history/star";
import type { QueryHistoryEntry } from "@/api/types";

// The recall drawer for Explore: a user's recent query runs, starred first.
// Each row recalls into the editor (Pencil), re-runs immediately (Play), or
// pins as a favourite (Star). The list refetches whenever a query runs so a
// just-run query shows up without a manual refresh — the parent bumps the
// shared query key via TanStack invalidation.
const HISTORY_KEY = ["query-history"] as const;

/** Refetch the history list — call after a run records a new entry. */
export function useRefreshQueryHistory() {
  const queryClient = useQueryClient();
  return () => queryClient.invalidateQueries({ queryKey: HISTORY_KEY });
}

// Collapse repeated runs of the same SQL to a single row, keyed by the query
// text. The list arrives starred-first then most-recent, so keeping the first
// occurrence of each unique SQL preserves that priority while dropping dupes.
function dedupeBySql(entries: QueryHistoryEntry[]): QueryHistoryEntry[] {
  const seen = new Set<string>();
  return entries.filter((e) => {
    const key = e.sql.trim();
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

export function QueryHistoryDrawer({
  onRecall,
  onRerun,
}: {
  /** Load a past query's SQL into the editor without running it. */
  onRecall: (sql: string) => void;
  /** Load and immediately run a past query. */
  onRerun: (sql: string) => void;
}) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const { data } = useQuery({
    queryKey: HISTORY_KEY,
    queryFn: () => listQueryHistory(client),
  });

  const star = useMutation({
    mutationFn: ({ id, starred }: { id: string; starred: boolean }) =>
      starQueryHistory(client, id, { starred }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: HISTORY_KEY }),
  });

  const entries = dedupeBySql(data?.entries ?? []);
  if (entries.length === 0) {
    return (
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <History className="size-3.5" />
        No query history yet — run a query to start recording.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
        <History className="size-3.5" />
        Recent queries
      </div>
      <ul className="flex max-h-56 flex-col gap-1 overflow-auto">
        {entries.map((e) => (
          <HistoryRow
            key={e.sql}
            entry={e}
            onRecall={() => onRecall(e.sql)}
            onRerun={() => onRerun(e.sql)}
            onToggleStar={() => star.mutate({ id: e.id, starred: !e.starred })}
          />
        ))}
      </ul>
    </div>
  );
}

function HistoryRow({
  entry,
  onRecall,
  onRerun,
  onToggleStar,
}: {
  entry: QueryHistoryEntry;
  onRecall: () => void;
  onRerun: () => void;
  onToggleStar: () => void;
}) {
  // One-line preview of the SQL; the full text is on the title for hover.
  const preview = entry.sql.replace(/\s+/g, " ").trim();
  return (
    <li className="group flex items-center gap-2 rounded-md px-2 py-1 hover:bg-primary/5">
      <button
        type="button"
        onClick={onToggleStar}
        title={entry.starred ? "Unstar" : "Star"}
        className={entry.starred ? "text-primary" : "text-muted-foreground/50 hover:text-primary"}
      >
        <Star className="size-3.5" fill={entry.starred ? "currentColor" : "none"} />
      </button>
      <span
        className={`flex-1 truncate font-mono text-xs ${entry.error ? "text-destructive" : "text-foreground/80"}`}
        title={entry.sql}
      >
        {preview}
      </span>
      {typeof entry.row_count === "number" ? (
        <span className="shrink-0 text-[0.7rem] text-muted-foreground">
          {entry.row_count} rows
        </span>
      ) : null}
      <button
        type="button"
        onClick={onRecall}
        title="Load into editor"
        className="text-muted-foreground/60 opacity-0 transition-opacity hover:text-foreground group-hover:opacity-100"
      >
        <Pencil className="size-3.5" />
      </button>
      <button
        type="button"
        onClick={onRerun}
        title="Run again"
        className="text-muted-foreground/60 opacity-0 transition-opacity hover:text-foreground group-hover:opacity-100"
      >
        <Play className="size-3.5" />
      </button>
    </li>
  );
}
