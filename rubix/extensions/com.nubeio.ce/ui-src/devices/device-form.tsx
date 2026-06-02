// `devices/device-form.tsx` — create / edit one control engine.
//
// SCAFFOLD: renders the fields (IP, port, username, password, …) and
// wires submit to `createDevice` / `updateDevice`. No validation,
// no dirty-tracking — fill those in with the real CRUD logic.

import * as React from "react";

import { createDevice, updateDevice } from "../ce-api";
import type { DeviceInput, DeviceRow } from "../types";

export function DeviceForm({
  initial,
  onDone,
  onCancel,
}: {
  initial: DeviceRow | null;
  onDone: () => void;
  onCancel: () => void;
}): React.ReactElement {
  const isEdit = initial !== null;
  const [form, setForm] = React.useState<DeviceInput>(() => ({
    device_id: initial?.device_id ?? "",
    name: initial?.name ?? "",
    description: initial?.description ?? "",
    engine_kind: initial?.engine_kind ?? "",
    ip: initial?.ip ?? "",
    port: initial?.port ?? 443,
    username: initial?.username ?? "",
    password: "",
  }));

  const set = <K extends keyof DeviceInput>(key: K, value: DeviceInput[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    // TODO: client-side validation (required device_id + ip, port range).
    const op = isEdit ? updateDevice : createDevice;
    op(form).then(onDone).catch(() => onDone());
  };

  return (
    <form onSubmit={submit} className="ext-card flex max-w-xl flex-col gap-4 p-4">
      <h4 className="text-base font-semibold">
        {isEdit ? `Edit ${initial?.name ?? initial?.device_id}` : "Add control engine"}
      </h4>

      <Field label="Device ID">
        <Input
          value={form.device_id}
          disabled={isEdit}
          onChange={(v) => set("device_id", v)}
          placeholder="ce-001"
        />
      </Field>
      <Field label="Name">
        <Input value={form.name ?? ""} onChange={(v) => set("name", v)} />
      </Field>
      <Field label="Engine kind">
        <Input
          value={form.engine_kind ?? ""}
          onChange={(v) => set("engine_kind", v)}
          placeholder="niagara | sedona"
        />
      </Field>
      <div className="grid grid-cols-[1fr_120px] gap-3">
        <Field label="IP / host">
          <Input value={form.ip ?? ""} onChange={(v) => set("ip", v)} placeholder="10.0.0.5" />
        </Field>
        <Field label="Port">
          <Input
            value={String(form.port ?? "")}
            onChange={(v) => set("port", Number(v) || 0)}
            placeholder="443"
          />
        </Field>
      </div>
      <Field label="Username">
        <Input value={form.username ?? ""} onChange={(v) => set("username", v)} />
      </Field>
      <Field label="Password">
        <Input
          value={form.password ?? ""}
          type="password"
          onChange={(v) => set("password", v)}
          placeholder={isEdit ? "(unchanged)" : ""}
        />
      </Field>

      <div className="flex justify-end gap-2 pt-2">
        <button
          type="button"
          onClick={onCancel}
          className="rounded-md border border-border px-3 py-1.5 text-sm"
        >
          Cancel
        </button>
        <button
          type="submit"
          className="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground"
        >
          {isEdit ? "Save" : "Create"}
        </button>
      </div>
    </form>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}): React.ReactElement {
  return (
    <label className="flex flex-col gap-1">
      <span className="ext-eyebrow">{label}</span>
      {children}
    </label>
  );
}

function Input({
  value,
  onChange,
  type = "text",
  placeholder,
  disabled,
}: {
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
  disabled?: boolean;
}): React.ReactElement {
  return (
    <input
      type={type}
      value={value}
      disabled={disabled}
      placeholder={placeholder}
      onChange={(e) => onChange(e.target.value)}
      className="rounded-md border border-border bg-background px-2.5 py-1.5 text-sm disabled:opacity-60"
    />
  );
}
