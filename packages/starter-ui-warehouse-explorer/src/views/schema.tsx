// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: 1a0736055a4647c18d0be19347e4325007c7bd52.
// Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.

import { GitBranch, ShieldX } from "lucide-react";

import { useClickhouseErd } from "../hooks/use-warehouse-ch";
import { Skeleton } from "../components/ui/skeleton";
import { ErdDiagram } from "../components/erd/erd-diagram";
import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";

export function Schema() {
  const { data, isLoading, isError, error } = useClickhouseErd();

  if (isLoading) return <SchemaSkeleton />;

  if (isError) {
    return (
      <Card>
        <CardHeader className="flex items-center">
          <ShieldX className="mb-4 h-12 w-12 text-red-400" />
          <CardTitle className="text-red-400">
            Failed to load schema
          </CardTitle>
          <CardDescription className="text-red-400/80">
            {error?.message ?? "The schema endpoint returned an error."}
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  // `data` may still be undefined here if the query has been
  // disabled or returned a non-JSON body. Treat that the same as an
  // empty schema so the page renders a meaningful state instead of
  // a silent blank panel.
  if (!data || data.tables.length === 0) {
    return (
      <Card>
        <CardHeader className="flex items-center">
          <GitBranch className="mb-4 h-12 w-12 text-muted-foreground" />
          <CardTitle>No Tables Found</CardTitle>
          <CardDescription>
            The database has no tables to display in the schema diagram.
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  return <ErdDiagram data={data} />;
}

function SchemaSkeleton() {
  return (
    <div className="flex flex-col gap-4">
      <Skeleton className="w-full h-[calc(100vh-12rem)]" />
    </div>
  );
}
