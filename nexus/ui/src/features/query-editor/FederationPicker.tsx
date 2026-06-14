import { Plus, X } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";
import { Button } from "@nube/starter-ui-kit/components/button";

import { useDatasources } from "@/features/datasources/useDatasources";
import type { FederatedSourceRef } from "@/api/types";

// Federated-sources editor for the explorer. Each row binds an `alias`
// (the table name the SQL JOINs against, surfaced as `ds_<alias>` server-
// side) to a datasource id and an optional remote table. When the list is
// non-empty the query must run via the unscoped `POST /query`, since it
// spans datasources (RW-05). Kept folded/empty by default — additive only.
export function FederationPicker({
  sources,
  onChange,
}: {
  sources: FederatedSourceRef[];
  onChange: (next: FederatedSourceRef[]) => void;
}) {
  const { data, isPending, isError } = useDatasources();
  const datasources = data ?? [];

  const placeholder = isPending
    ? "Loading…"
    : isError
      ? "Failed to load"
      : datasources.length === 0
        ? "No datasources"
        : "Datasource";

  const addRow = () =>
    onChange([...sources, { alias: "", datasource: "", table: "" }]);
  const removeRow = (i: number) =>
    onChange(sources.filter((_, idx) => idx !== i));
  const patch = (i: number, partial: Partial<FederatedSourceRef>) =>
    onChange(sources.map((s, idx) => (idx === i ? { ...s, ...partial } : s)));

  return (
    <div className="flex flex-col gap-2">
      {sources.map((src, i) => (
        <div key={i} className="flex items-center gap-2">
          <input
            type="text"
            value={src.alias}
            onChange={(e) => patch(i, { alias: e.target.value })}
            placeholder="alias"
            aria-label="Source alias"
            className="h-9 w-32 rounded-md border bg-transparent px-2 text-sm"
          />
          <Select
            value={src.datasource || undefined}
            onValueChange={(v) => patch(i, { datasource: v })}
            disabled={isPending || isError}
          >
            <SelectTrigger className="w-56">
              <SelectValue placeholder={placeholder} />
            </SelectTrigger>
            <SelectContent>
              {datasources.map((ds) => (
                <SelectItem key={ds.id} value={ds.id}>
                  {ds.name}
                  <span className="ms-2 text-xs text-muted-foreground">
                    {ds.kind}
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <input
            type="text"
            value={src.table ?? ""}
            onChange={(e) => patch(i, { table: e.target.value })}
            placeholder="table (optional)"
            aria-label="Remote table"
            className="h-9 w-44 rounded-md border bg-transparent px-2 text-sm"
          />
          {src.alias.trim() ? (
            <span className="text-xs text-muted-foreground">
              → JOIN as <code className="font-mono">ds_{src.alias.trim()}</code>
            </span>
          ) : null}
          <button
            type="button"
            onClick={() => removeRow(i)}
            aria-label="Remove source"
            className="ms-auto rounded-md p-1.5 text-muted-foreground hover:bg-accent"
          >
            <X className="size-4" />
          </button>
        </div>
      ))}
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="w-fit gap-2"
        onClick={addRow}
      >
        <Plus className="size-4" />
        Add source
      </Button>
    </div>
  );
}
