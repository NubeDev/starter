// `devices/devices-panel.tsx` — the device catalog (list + CRUD).
//
// Lists registered control engines, lets the operator add / edit /
// delete one, and links each to its wiresheet. SCAFFOLD: data loading
// and the form submit call the real tool wrappers in ce-api.ts, but
// state handling is intentionally minimal — no validation, no
// optimistic updates, no error toasts yet.

import * as React from "react";

import { ExternalLink, Pencil, Plus, Trash2 } from "lucide-react";

import { EXTENSION_ID, type DeviceRow } from "../types";
import { deleteDevice, listDevices } from "../ce-api";
import { DeviceForm } from "./device-form";

export function DevicesPanel(): React.ReactElement {
  const [rows, setRows] = React.useState<ReadonlyArray<DeviceRow>>([]);
  const [editing, setEditing] = React.useState<DeviceRow | "new" | null>(null);

  const reload = React.useCallback(() => {
    // TODO: surface loading + error states.
    listDevices().then(setRows).catch(() => setRows([]));
  }, []);

  React.useEffect(() => reload(), [reload]);

  if (editing) {
    return (
      <DeviceForm
        initial={editing === "new" ? null : editing}
        onDone={() => {
          setEditing(null);
          reload();
        }}
        onCancel={() => setEditing(null)}
      />
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => setEditing("new")}
          className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground"
        >
          <Plus className="size-4" /> Add engine
        </button>
      </div>

      <div className="ext-card overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/40 text-muted-foreground">
            <tr>
              <Th>Name</Th>
              <Th>Kind</Th>
              <Th>Address</Th>
              <Th>Status</Th>
              <Th>{""}</Th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-3 py-6 text-center text-muted-foreground">
                  No engines registered yet.
                </td>
              </tr>
            ) : (
              rows.map((d) => (
                <tr key={d.device_id} className="border-t border-border">
                  <Td>{d.name ?? d.device_id}</Td>
                  <Td>{d.engine_kind ?? "—"}</Td>
                  <Td>
                    {d.ip}:{d.port}
                  </Td>
                  <Td>{d.status ?? "unknown"}</Td>
                  <Td>
                    <div className="flex items-center justify-end gap-2">
                      <a
                        href={`/extensions/${EXTENSION_ID}/wiresheet/${d.device_id}`}
                        className="inline-flex items-center gap-1 text-primary hover:underline"
                        title="Open wiresheet"
                      >
                        <ExternalLink className="size-4" /> Wiresheet
                      </a>
                      <button
                        type="button"
                        onClick={() => setEditing(d)}
                        className="text-muted-foreground hover:text-foreground"
                        title="Edit"
                      >
                        <Pencil className="size-4" />
                      </button>
                      <button
                        type="button"
                        onClick={() => deleteDevice(d.device_id).then(reload)}
                        className="text-muted-foreground hover:text-destructive"
                        title="Delete"
                      >
                        <Trash2 className="size-4" />
                      </button>
                    </div>
                  </Td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function Th({ children }: { children: React.ReactNode }): React.ReactElement {
  return <th className="px-3 py-2 text-left font-medium">{children}</th>;
}
function Td({ children }: { children: React.ReactNode }): React.ReactElement {
  return <td className="px-3 py-2">{children}</td>;
}
