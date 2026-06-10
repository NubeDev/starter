import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import { useResourceHistory } from "@/features/audit/useResourceHistory";
import { ChangeList } from "@/features/audit/ChangeList";

// A "History" tab for one resource: its change timeline, newest first. Drop into
// a dashboard/datasource detail view as `<ResourceHistoryTab kind={...} id={...}/>`.
// `enabled` defers the fetch until the tab is actually shown.
export function ResourceHistoryTab({
  kind,
  id,
  enabled = true,
}: {
  kind: string;
  id: string;
  enabled?: boolean;
}) {
  const query = useResourceHistory(kind, id, {}, enabled);

  if (query.isPending) return <Loading label="Loading history…" />;
  if (query.isError) {
    return (
      <ErrorState
        message={query.error instanceof Error ? query.error.message : undefined}
      />
    );
  }
  return <ChangeList changes={query.data.items} />;
}
