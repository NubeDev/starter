import { useEffect, useMemo, useState, type FormEvent } from "react";
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type {
  CreateDatasourceRequest,
  DatasourceKind,
  DatasourceKindSummary,
  TestConnectionRequest,
  TestDatasourceResponse,
} from "@/api/types";
import { NodeConfigForm } from "@/features/flows/builder/NodeConfigForm";
import {
  useCreateDatasource,
  useTestConnection,
} from "@/features/datasources/useDatasourceMutations";
import { useDatasourceKinds } from "@/features/datasources/useDatasources";

// Connection form for a new datasource. The form is schema-driven: it lists the
// connector kinds from `GET /datasources/kinds` and renders each kind's config
// fields from its JSON Schema (the same renderer the flow builder uses for node
// configs). Secret fields (`secret_fields`) render as write-only password
// inputs. The "Test connection" probe is shown only for kinds that declare a
// probe (every kind today does, via `query` or `connect`).
//
// Both create and probe carry the same shape: `postgres` fills the flat
// `host`/`port`/`database`/`user`/`password` fields, while non-SQL kinds
// (`mqtt`/`zenoh`) and file kinds (`parquet`/`csv`) carry their parameters in
// the generic `config` blob — the create and test DTOs both accept it, so every
// kind the catalogue declares can be created here.
export function DatasourceFormDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const kinds = useDatasourceKinds();
  const create = useCreateDatasource();
  const test = useTestConnection();

  const [name, setName] = useState("");
  const [kindName, setKindName] = useState<string>("");
  // Config values keyed by the kind's schema property names.
  const [config, setConfig] = useState<Record<string, unknown>>({});

  const kindList = kinds.data ?? [];
  const selected: DatasourceKindSummary | undefined = useMemo(
    () => kindList.find((k) => k.name === kindName),
    [kindList, kindName],
  );

  // Auto-select the sole/first kind once the catalogue loads so the form is
  // never blank when there's an obvious choice.
  useEffect(() => {
    if (!kindName && kindList.length > 0) setKindName(kindList[0].name);
  }, [kindName, kindList]);

  function reset() {
    setName("");
    setConfig({});
    test.reset();
  }

  function onSelectKind(next: string) {
    setKindName(next);
    setConfig({});
    test.reset();
  }

  function onConfigChange(next: Record<string, unknown>) {
    // A field edit invalidates the last probe so a stale green tick can't imply
    // freshly-changed credentials were tested.
    test.reset();
    setConfig(next);
  }

  // Whether this kind can be probed before save. Every declared kind has a
  // `test_mode` today, but guard so a future probe-less kind hides the button.
  const canProbe = Boolean(selected?.test_mode);
  // Every declared kind can be created: postgres via the flat fields, others via
  // the generic `config` blob (the create DTO accepts both).
  const canCreate = Boolean(selected);

  // Build the connection payload for the selected kind: postgres lifts its
  // schema fields to the top level; every other kind carries them under `config`.
  // Shared by both the probe and the create call so the two never diverge.
  function connectionFields():
    | { host: string; port: number; database: string; user: string; password: string }
    | { config: Record<string, unknown> } {
    const kind = selected?.name ?? "postgres";
    if (kind === "postgres") {
      return {
        host: str(config.host),
        port: num(config.port) ?? 5432,
        database: str(config.database),
        user: str(config.user),
        password: str(config.password),
      };
    }
    return { config };
  }

  function probeBody(): TestConnectionRequest {
    const kind = (selected?.name ?? "postgres") as DatasourceKind;
    return { kind, ...connectionFields() };
  }

  function onTest() {
    test.mutate(probeBody());
  }

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!canCreate || !selected) return;
    const body: CreateDatasourceRequest = {
      name: name.trim(),
      kind: selected.name as DatasourceKind,
      ...connectionFields(),
    };
    create.mutate(body, {
      onSuccess: () => {
        onOpenChange(false);
        reset();
      },
    });
  }

  const description = selected?.description
    ? selected.description
    : selected
      ? `Connect a ${selected.name} datasource.`
      : "Connect a datasource.";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass max-w-md">
        <DialogHeader>
          <DialogTitle>New datasource</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={onSubmit}>
          <div className="space-y-1.5">
            <Label htmlFor="ds-name">Name</Label>
            <Input
              id="ds-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoComplete="off"
              required
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="ds-kind">Kind</Label>
            {kinds.isLoading ? (
              <p className="text-sm text-muted-foreground">Loading kinds…</p>
            ) : kinds.isError ? (
              <p role="alert" className="text-sm text-destructive">
                Couldn't load datasource kinds.
              </p>
            ) : (
              <Select value={kindName} onValueChange={onSelectKind}>
                <SelectTrigger id="ds-kind">
                  <SelectValue placeholder="Select a kind…" />
                </SelectTrigger>
                <SelectContent>
                  {kindList.map((k) => (
                    <SelectItem key={k.name} value={k.name}>
                      {k.name}
                      {k.surface ? ` · ${k.surface}` : ""}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>

          {selected ? (
            <NodeConfigForm
              schema={selected.config_schema}
              config={config}
              onChange={onConfigChange}
              secretFields={selected.secret_fields}
              emptyHint="This kind has no configuration."
            />
          ) : null}

          {create.isError ? (
            <p role="alert" className="text-sm text-destructive">
              Couldn't create the datasource.
            </p>
          ) : null}

          <ProbeResult
            pending={test.isPending}
            failed={test.isError}
            result={test.data}
          />

          <DialogFooter>
            {canProbe ? (
              <Button
                type="button"
                variant="outline"
                onClick={onTest}
                disabled={test.isPending || !selected}
              >
                {test.isPending ? "Testing…" : "Test connection"}
              </Button>
            ) : null}
            <Button type="submit" disabled={create.isPending || !canCreate || !name.trim()}>
              {create.isPending ? "Connecting…" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// Read a config value as a trimmed string for the postgres DTO's required
// string fields.
function str(v: unknown): string {
  return v == null ? "" : String(v).trim();
}

// Read a config value as a number, or undefined if it isn't one (so a default
// can apply).
function num(v: unknown): number | undefined {
  if (v == null || v === "") return undefined;
  const n = Number(v);
  return Number.isFinite(n) ? n : undefined;
}

// Shows the pre-save probe outcome below the form. A transport failure and a
// failed probe (`ok:false`) both read as "couldn't connect"; a successful probe
// reports latency so the user sees the connection is live before saving.
export function ProbeResult({
  pending,
  failed,
  result,
}: {
  pending: boolean;
  failed: boolean;
  result: TestDatasourceResponse | undefined;
}) {
  if (pending) return null;
  if (failed) {
    return (
      <p role="status" className="text-sm text-destructive">
        Couldn't reach the database.
      </p>
    );
  }
  if (!result) return null;
  if (result.ok) {
    return (
      <p role="status" className="text-sm text-emerald-600 dark:text-emerald-400">
        Connected{result.latency_ms != null ? ` in ${result.latency_ms}ms` : ""}.
      </p>
    );
  }
  return (
    <p role="status" className="text-sm text-destructive">
      {result.message ?? "Connection failed."}
    </p>
  );
}
