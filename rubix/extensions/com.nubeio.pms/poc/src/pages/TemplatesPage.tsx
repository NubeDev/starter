import { useState } from "react";
import * as Icons from "lucide-react";
import { Trash2, Upload, Download, Server, Cpu } from "lucide-react";
import { useStore } from "@/store/store";
import { Card, SectionTitle, Chip, Empty } from "@/components/ui";
import type { DeviceTemplate } from "@/types";
import { downloadJSON } from "@/lib/provision";

function TemplateIcon({ name }: { name?: string }) {
  const key = (name ?? "box")
    .split("-")
    .map((p) => p[0]?.toUpperCase() + p.slice(1))
    .join("") as keyof typeof Icons;
  const Cmp = (Icons[key] as React.ComponentType<{ size?: number; className?: string }>) ?? Icons.Box;
  return <Cmp size={18} className="text-accent" />;
}

export function TemplatesPage() {
  const { state, dispatch } = useStore();
  const [filter, setFilter] = useState<"all" | "gateway" | "end_device">("all");

  const list = state.templates.filter((t) => filter === "all" || t.role === filter);

  const importTemplates = async () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "application/json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      try {
        const parsed = JSON.parse(await file.text());
        const arr: DeviceTemplate[] = Array.isArray(parsed) ? parsed : [parsed];
        for (const t of arr) {
          const { id: _id, ...rest } = t;
          void _id;
          dispatch({ type: "ADD_TEMPLATE", template: rest as Omit<DeviceTemplate, "id"> });
        }
        alert(`Imported ${arr.length} template(s).`);
      } catch (e) {
        alert("Invalid template JSON: " + (e as Error).message);
      }
    };
    input.click();
  };

  return (
    <>
      <SectionTitle
        title="Device Templates"
        sub="Gateway and end-device blueprints. Loaded by admin; clients instantiate them into projects."
        action={
          <div className="flex gap-2">
            <button className="btn" onClick={importTemplates}>
              <Upload size={15} /> Import JSON
            </button>
            <button className="btn" onClick={() => downloadJSON("templates.json", state.templates)}>
              <Download size={15} /> Export all
            </button>
          </div>
        }
      />

      <div className="flex gap-2 mb-4">
        {(["all", "gateway", "end_device"] as const).map((f) => (
          <button
            key={f}
            className={`btn ${filter === f ? "btn-primary" : "btn-ghost"}`}
            onClick={() => setFilter(f)}
          >
            {f === "all" ? "All" : f === "gateway" ? "Gateways" : "End Devices"}
          </button>
        ))}
      </div>

      <div className="grid grid-cols-2 gap-4">
        {list.length === 0 && <Empty>No templates.</Empty>}
        {list.map((t) => (
          <Card key={t.id}>
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <TemplateIcon name={t.icon} />
                <div>
                  <div className="font-medium flex items-center gap-2">
                    {t.name}
                    {t.role === "gateway" ? (
                      <Chip>
                        <Server size={11} /> gateway
                      </Chip>
                    ) : (
                      <Chip>
                        <Cpu size={11} /> end device
                      </Chip>
                    )}
                  </div>
                  <div className="text-xs text-muted">
                    {t.vendor} · {t.model} · {t.category}
                  </div>
                </div>
              </div>
              <button
                className="btn btn-ghost btn-danger"
                onClick={() => dispatch({ type: "DELETE_TEMPLATE", id: t.id })}
              >
                <Trash2 size={14} />
              </button>
            </div>

            <div className="mt-3 flex flex-wrap gap-1.5">
              {t.networks.map((n) => (
                <Chip key={n}>{n}</Chip>
              ))}
            </div>

            <div className="mt-3 grid grid-cols-2 gap-3 text-sm">
              <div>
                <div className="label">Settings ({t.settings.length})</div>
                <div className="text-muted text-xs">
                  {t.settings.map((s) => s.label).join(", ") || "—"}
                </div>
              </div>
              <div>
                <div className="label">Points ({t.points.reduce((n, p) => n + (p.repeat ?? 1), 0)})</div>
                <div className="text-muted text-xs">
                  {t.points.map((p) => (p.repeat ? `${p.name}×${p.repeat}` : p.name)).join(", ") || "—"}
                </div>
              </div>
            </div>
          </Card>
        ))}
      </div>
    </>
  );
}
