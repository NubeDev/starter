import { useState } from "react";
import { Database, Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { useDatasources } from "@/features/datasources/useDatasources";
import { useRemoveDatasource } from "@/features/datasources/useDatasourceMutations";
import { DatasourceFormDialog } from "@/features/datasources/DatasourceFormDialog";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Datasource management: list the tenant's datasources with a create
// action and per-row delete, all over the real endpoints. Loading / empty
// / error states throughout (F0).
export function DatasourcesPage() {
  const { data, isPending, isError, error } = useDatasources();
  const remove = useRemoveDatasource();
  const [adding, setAdding] = useState(false);

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold tracking-tight">Datasources</h2>
        <Button size="sm" className="gap-2" onClick={() => setAdding(true)}>
          <Plus className="size-4" />
          New datasource
        </Button>
      </div>

      <div className="min-h-0 flex-1">
        {isPending ? (
          <Loading label="Loading datasources…" />
        ) : isError ? (
          <ErrorState message={error instanceof Error ? error.message : undefined} />
        ) : data.length === 0 ? (
          <Empty
            title="No datasources"
            description="Connect a database to start querying."
          />
        ) : (
          <ul className="flex flex-col gap-2">
            {data.map((ds) => (
              <li
                key={ds.id}
                className="glass flex items-center gap-3 rounded-lg px-4 py-3"
              >
                <span className="grid size-9 place-items-center rounded-lg bg-primary/15 text-primary">
                  <Database className="size-4" />
                </span>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium text-foreground">
                    {ds.name}
                  </p>
                  <p className="text-xs text-muted-foreground">{ds.kind}</p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={`Delete ${ds.name}`}
                  disabled={remove.isPending}
                  onClick={() => remove.mutate(ds.id)}
                  className="text-muted-foreground hover:text-destructive"
                >
                  <Trash2 className="size-4" />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <DatasourceFormDialog open={adding} onOpenChange={setAdding} />
    </div>
  );
}
