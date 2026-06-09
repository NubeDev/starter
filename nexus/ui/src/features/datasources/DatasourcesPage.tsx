import { useState } from "react";
import { Plus } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { useDatasources } from "@/features/datasources/useDatasources";
import { useRemoveDatasource } from "@/features/datasources/useDatasourceMutations";
import { DatasourceFormDialog } from "@/features/datasources/DatasourceFormDialog";
import { DatasourceRow } from "@/features/datasources/DatasourceRow";
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
              <DatasourceRow
                key={ds.id}
                datasource={ds}
                onRemove={() => remove.mutate(ds.id)}
                removing={remove.isPending}
              />
            ))}
          </ul>
        )}
      </div>

      <DatasourceFormDialog open={adding} onOpenChange={setAdding} />
    </div>
  );
}
