import { useState } from "react";
import { Plus } from "lucide-react";
import type { DeviceTemplate, NetworkBus, EndDeviceInstance } from "@/types";
import { Field, Select, TextInput } from "@/components/ui";
import { compatibleTemplates, freeSlots, parseRange } from "@/lib/networks";
import { bulkByCount, bulkByRange } from "@/lib/bulkAdd";

// Bulk-add control for one bus. Offers both modes:
//  - count + start address (sequential, skips taken)
//  - explicit address range like "1-32"
// Only compatible templates appear; cap is enforced in the lib.

export function BulkAddPanel({
  bus,
  templates,
  newId,
  onAdd,
}: {
  bus: NetworkBus;
  templates: DeviceTemplate[];
  newId: (p: string) => string;
  onAdd: (devices: EndDeviceInstance[]) => void;
}) {
  const compat = compatibleTemplates(bus, templates);
  const [tplId, setTplId] = useState(compat[0]?.id ?? "");
  const [mode, setMode] = useState<"count" | "range">("count");
  const [count, setCount] = useState(1);
  const [start, setStart] = useState(1);
  const [range, setRange] = useState("1-8");
  const [msg, setMsg] = useState<string>("");

  const free = freeSlots(bus);
  const tpl = compat.find((t) => t.id === tplId);

  if (compat.length === 0) {
    return (
      <p className="text-xs text-warn">
        No loaded device template is compatible with this {bus.network} bus.
      </p>
    );
  }

  const run = () => {
    if (!tpl) return;
    const res =
      mode === "count"
        ? bulkByCount(tpl, bus, count, start, newId)
        : bulkByRange(tpl, bus, parseRange(range), newId);
    if (res.added === 0) {
      setMsg(res.reason ?? "Nothing added (bus full or addresses taken).");
      return;
    }
    onAdd(res.devices);
    const skipNote = res.skipped.length ? ` · skipped ${res.skipped.length} (taken/over-cap)` : "";
    setMsg(`Added ${res.added}${skipNote}.`);
  };

  return (
    <div className="rounded-lg border border-border bg-bg/40 p-3 space-y-3">
      <div className="flex items-center justify-between">
        <span className="label !mb-0">Bulk add</span>
        <span className="text-xs text-muted">{free} slot(s) free</span>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <Field label="Template">
          <Select value={tplId} onChange={(e) => setTplId(e.target.value)}>
            {compat.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Mode">
          <Select value={mode} onChange={(e) => setMode(e.target.value as "count" | "range")}>
            <option value="count">Count + start address</option>
            <option value="range">Address range</option>
          </Select>
        </Field>
      </div>

      {mode === "count" ? (
        <div className="grid grid-cols-2 gap-3">
          <Field label="Quantity">
            <TextInput
              type="number"
              min={1}
              value={count}
              onChange={(e) => setCount(Math.max(1, Number(e.target.value)))}
            />
          </Field>
          <Field label="Start address">
            <TextInput
              type="number"
              min={1}
              value={start}
              onChange={(e) => setStart(Math.max(1, Number(e.target.value)))}
            />
          </Field>
        </div>
      ) : (
        <Field label="Address range (e.g. 1-32)">
          <TextInput value={range} onChange={(e) => setRange(e.target.value)} placeholder="1-32" />
        </Field>
      )}

      <div className="flex items-center justify-between">
        <button className="btn btn-primary" onClick={run} disabled={free === 0}>
          <Plus size={14} /> Add devices
        </button>
        {msg && <span className="text-xs text-muted">{msg}</span>}
      </div>
    </div>
  );
}
