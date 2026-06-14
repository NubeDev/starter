import { useState, type FormEvent } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import {
  createQueryKind,
  type CreateQueryKindRequest,
  type QueryKindDetail,
} from "@/api/query/kinds";

// "Save as kind" dialog: promote the SQL just authored in the Explore editor
// into a reusable, named query-kind via `POST /api/v1/query-kinds`. For v1 we
// save kinds with NO params (empty `params_schema`) — the SQL should use only
// `$caller_tenant_id` (host-bound) and literal values. The server lints on
// save (tenant-isolation + declared params), and any 400/409 message surfaces
// verbatim. The current SQL is shown read-only so the user confirms what
// they're saving; the editor itself never changes here.
export function SaveKindDialog({
  sql,
  open,
  onClose,
  onSaved,
}: {
  sql: string;
  open: boolean;
  onClose: () => void;
  onSaved?: (detail: QueryKindDetail) => void;
}) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const [form, setForm] = useState({
    name: "",
    description: "",
    datasourceKind: "postgres",
    tables: "",
  });

  const set = (k: keyof typeof form) => (v: string) =>
    setForm((f) => ({ ...f, [k]: v }));

  const save = useMutation<QueryKindDetail, Error, CreateQueryKindRequest>({
    mutationFn: (body) => createQueryKind(client, body),
    onSuccess: (detail) => {
      // The kind picker keys its list on ["query-kinds"]; refetch so the new
      // kind shows up immediately for kind-mode runs.
      queryClient.invalidateQueries({ queryKey: ["query-kinds"] });
      onSaved?.(detail);
      save.reset();
      onClose();
    },
  });

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const name = form.name.trim();
    const description = form.description.trim();
    // Comma-separated → trimmed, non-empty table names.
    const tables = form.tables
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);

    const body: CreateQueryKindRequest = {
      name,
      sql,
      datasource_kind: form.datasourceKind.trim() || "postgres",
      // v1: no declared params — empty schema, literals + $caller_tenant_id only.
      params_schema: {},
      ...(tables.length > 0 ? { tables } : {}),
      ...(description ? { description } : {}),
    };
    save.mutate(body);
  }

  const canSubmit =
    form.name.trim().length > 0 && sql.trim().length > 0 && !save.isPending;

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          save.reset();
          onClose();
        }
      }}
    >
      <DialogContent className="glass max-w-md">
        <DialogHeader>
          <DialogTitle>Save as kind</DialogTitle>
          <DialogDescription>
            Promote this SQL into a reusable, named query-kind that panels can
            invoke by name.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={onSubmit}>
          <div className="space-y-1.5">
            <Label htmlFor="kind-name">Name</Label>
            <Input
              id="kind-name"
              value={form.name}
              onChange={(e) => set("name")(e.target.value)}
              placeholder="com.acme.my_query"
              autoComplete="off"
              required
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="kind-description">Description (optional)</Label>
            <Input
              id="kind-description"
              value={form.description}
              onChange={(e) => set("description")(e.target.value)}
              placeholder="What this query returns"
              autoComplete="off"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="kind-datasource">Datasource kind</Label>
            <Input
              id="kind-datasource"
              value={form.datasourceKind}
              onChange={(e) => set("datasourceKind")(e.target.value)}
              placeholder="postgres"
              autoComplete="off"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="kind-tables">Tables (optional)</Label>
            <Input
              id="kind-tables"
              value={form.tables}
              onChange={(e) => set("tables")(e.target.value)}
              placeholder="readings, sites"
              autoComplete="off"
            />
            <p className="text-xs text-muted-foreground">
              Tables this query reads. If set, your SQL must filter by
              $caller_tenant_id.
            </p>
          </div>
          <div className="space-y-1.5">
            <Label>SQL</Label>
            <pre className="max-h-32 overflow-auto rounded-md border bg-muted/40 p-2 text-xs text-muted-foreground whitespace-pre-wrap">
              {sql}
            </pre>
          </div>
          {save.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {save.error.message}
            </p>
          ) : null}
          <DialogFooter>
            <Button type="submit" disabled={!canSubmit}>
              {save.isPending ? "Saving…" : "Save kind"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
