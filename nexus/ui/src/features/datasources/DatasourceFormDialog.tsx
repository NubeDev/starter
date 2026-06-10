import { useState, type FormEvent } from "react";
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

import type {
  CreateDatasourceRequest,
  TestConnectionRequest,
  TestDatasourceResponse,
} from "@/api/types";
import {
  useCreateDatasource,
  useTestConnection,
} from "@/features/datasources/useDatasourceMutations";

// Connection form for a new datasource. v1 ships Postgres only (the kind
// enum has one value), so kind is fixed rather than a picker. The password
// is write-only — submitted here, never read back.
export function DatasourceFormDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const create = useCreateDatasource();
  const test = useTestConnection();
  const [form, setForm] = useState({
    name: "",
    host: "",
    port: "5432",
    database: "",
    user: "",
    password: "",
  });

  const set = (k: keyof typeof form) => (v: string) => {
    // A field edit invalidates the last probe result so a stale green tick can't
    // imply the freshly-changed credentials were tested.
    test.reset();
    setForm((f) => ({ ...f, [k]: v }));
  };

  // The concrete Postgres connection fields the probe and the create call both
  // submit. Built with required (non-null) fields so it satisfies the create body;
  // the probe request accepts the same shape (its SQL fields are now optional).
  function connectionBody() {
    return {
      kind: "postgres" as const,
      host: form.host.trim(),
      port: Number(form.port) || 5432,
      database: form.database.trim(),
      user: form.user.trim(),
      password: form.password,
    };
  }

  function onTest() {
    const probe: TestConnectionRequest = connectionBody();
    test.mutate(probe);
  }

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const body: CreateDatasourceRequest = {
      name: form.name.trim(),
      ...connectionBody(),
    };
    create.mutate(body, {
      onSuccess: () => {
        onOpenChange(false);
        test.reset();
        setForm({ name: "", host: "", port: "5432", database: "", user: "", password: "" });
      },
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="glass max-w-md">
        <DialogHeader>
          <DialogTitle>New datasource</DialogTitle>
          <DialogDescription>Connect a Postgres database.</DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={onSubmit}>
          <Field id="ds-name" label="Name" value={form.name} onChange={set("name")} required />
          <div className="grid grid-cols-[1fr_6rem] gap-3">
            <Field id="ds-host" label="Host" value={form.host} onChange={set("host")} required />
            <Field id="ds-port" label="Port" value={form.port} onChange={set("port")} />
          </div>
          <Field id="ds-db" label="Database" value={form.database} onChange={set("database")} required />
          <div className="grid grid-cols-2 gap-3">
            <Field id="ds-user" label="User" value={form.user} onChange={set("user")} required />
            <Field
              id="ds-pass"
              label="Password"
              type="password"
              value={form.password}
              onChange={set("password")}
              required
            />
          </div>
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
            <Button
              type="button"
              variant="outline"
              onClick={onTest}
              disabled={test.isPending || !form.host || !form.user}
            >
              {test.isPending ? "Testing…" : "Test connection"}
            </Button>
            <Button type="submit" disabled={create.isPending}>
              {create.isPending ? "Connecting…" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
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

function Field({
  id,
  label,
  value,
  onChange,
  type = "text",
  required,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  required?: boolean;
}) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        autoComplete={type === "password" ? "new-password" : "off"}
        required={required}
      />
    </div>
  );
}
