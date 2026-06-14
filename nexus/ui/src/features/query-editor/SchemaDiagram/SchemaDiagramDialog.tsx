import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";

import { NEXUS_DB_DATASOURCE_ID } from "@/api/nexus-db/query";
import { useDatasourceSchema } from "@/features/sql-editor";
import { SchemaDiagram } from "@/features/query-editor/SchemaDiagram/SchemaDiagram";

// Opens the ER diagram for a datasource (or the Nexus DB, via the sentinel id)
// in a large overlay. It reads the *same* cached schema the sidebar and editor
// autocomplete use (`useDatasourceSchema`), so opening it costs no extra request
// once the schema has loaded for browsing.
export function SchemaDiagramDialog({
  datasourceId,
  open,
  onClose,
}: {
  datasourceId: string | undefined;
  open: boolean;
  onClose: () => void;
}) {
  const { data: schema, isLoading, isError } = useDatasourceSchema(datasourceId);
  const isNexusDb = datasourceId === NEXUS_DB_DATASOURCE_ID;

  return (
    <Dialog open={open} onOpenChange={(o) => (o ? undefined : onClose())}>
      <DialogContent className="flex h-[85vh] max-w-[90vw] flex-col gap-0 p-0 sm:max-w-[90vw]">
        <DialogHeader className="border-b border-border/60 px-4 py-3">
          <DialogTitle>
            Schema diagram
            <span className="ms-2 text-sm font-normal text-muted-foreground">
              {isNexusDb ? "Nexus DB (control-plane)" : "datasource"}
            </span>
          </DialogTitle>
        </DialogHeader>
        <div className="min-h-0 flex-1">
          {!datasourceId ? (
            <Centered>Pick a datasource to diagram its schema.</Centered>
          ) : isLoading ? (
            <Centered>Loading schema…</Centered>
          ) : isError ? (
            <Centered>Schema unavailable for this datasource.</Centered>
          ) : schema ? (
            <SchemaDiagram schema={schema} />
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
      {children}
    </div>
  );
}
