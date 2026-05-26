// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
//
// Schema view — ERD diagram. Rewritten from the upstream
// `routes/schema.tsx`:
//   * No `createFileRoute` / loader; data via `useChErd()`.
//   * Visible strings via `useExplorerMessages()`.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { GitBranch } from "lucide-react";

import { Skeleton } from "../components/ui/skeleton.js";
import { ErdDiagram } from "../components/erd/erd-diagram.js";
import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card.js";
import { useChErd } from "../hooks/index.js";
import { useExplorerMessages } from "../i18n/index.js";

export function ExplorerSchema() {
  const m = useExplorerMessages();
  const { data, isPending } = useChErd();

  if (isPending || !data) {
    return (
      <div className="flex flex-col gap-4">
        <Skeleton className="w-full h-[calc(100vh-12rem)]" />
      </div>
    );
  }

  if (data.tables.length === 0) {
    return (
      <Card>
        <CardHeader className="flex items-center">
          <GitBranch className="mb-4 h-12 w-12 text-muted-foreground" />
          <CardTitle>{m.schema.emptyTitle}</CardTitle>
          <CardDescription>{m.schema.emptyDescription}</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  return <ErdDiagram data={data} />;
}
