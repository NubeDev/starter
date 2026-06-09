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

import type { CreateDatasourceRequest } from "@/api/types";
import { useCreateDatasource } from "@/features/datasources/useDatasourceMutations";

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
  const [form, setForm] = useState({
    name: "",
    host: "",
    port: "5432",
    database: "",
    user: "",
    password: "",
  });

  const set = (k: keyof typeof form) => (v: string) =>
    setForm((f) => ({ ...f, [k]: v }));

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const body: CreateDatasourceRequest = {
      name: form.name.trim(),
      kind: "postgres",
      host: form.host.trim(),
      port: Number(form.port) || 5432,
      database: form.database.trim(),
      user: form.user.trim(),
      password: form.password,
    };
    create.mutate(body, {
      onSuccess: () => {
        onOpenChange(false);
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
          <DialogFooter>
            <Button type="submit" disabled={create.isPending}>
              {create.isPending ? "Connecting…" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
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
