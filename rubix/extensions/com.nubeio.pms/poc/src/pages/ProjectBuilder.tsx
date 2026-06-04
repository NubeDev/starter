import { useState } from "react";
import { useParams, Link } from "react-router-dom";
import {
  Trash2,
  Server,
  Cpu,
  ChevronDown,
  ChevronRight,
  FileText,
  FileSpreadsheet,
  FileJson,
  LayoutList,
  Network,
  Plus,
} from "lucide-react";
import { useStore } from "@/store/store";
import { Card, Field, Select, TextInput, SectionTitle, Empty, Chip } from "@/components/ui";
import { SettingsEditor } from "@/components/SettingsEditor";
import { BulkAddPanel } from "@/components/BulkAddPanel";
import { NetworkCanvas } from "@/pages/NetworkCanvas";
import { NETWORK_META } from "@/types";
import type {
  Project,
  GatewayInstance,
  NetworkBus,
  EndDeviceInstance,
  DeviceTemplate,
  NetworkType,
} from "@/types";
import { projectToProvision, downloadJSON } from "@/lib/provision";
import { exportProjectExcel, slug } from "@/lib/exportExcel";
import { exportProjectPdf } from "@/lib/exportPdf";
import * as edit from "@/lib/projectEdits";
import { freeSlots, isFull } from "@/lib/networks";

export function ProjectBuilder() {
  const { projectId } = useParams();
  const { state, dispatch, newId } = useStore();
  const project = state.projects.find((p) => p.id === projectId);
  const [view, setView] = useState<"form" | "canvas">("canvas");

  if (!project)
    return (
      <Empty>
        Project not found. <Link to="/projects" className="text-accent">Back</Link>
      </Empty>
    );

  const save = (next: Project) => dispatch({ type: "UPSERT_PROJECT", project: next });
  const client = state.clients.find((c) => c.id === project.clientId);
  const site = state.sites.find((s) => s.id === project.siteId);
  const gatewayTpls = state.templates.filter((t) => t.role === "gateway");

  const totalDevices = project.gateways.reduce(
    (n, g) => n + (g.buses ?? []).reduce((m, b) => m + b.devices.length, 0),
    0,
  );

  return (
    <>
      <SectionTitle
        title={project.name}
        sub={`${client?.name} · ${site?.name}${site?.address ? " · " + site.address : ""}`}
        action={
          <div className="flex gap-2">
            <div className="flex rounded-lg border border-border overflow-hidden mr-1">
              <button
                className={`px-3 py-2 text-sm flex items-center gap-1.5 ${view === "canvas" ? "bg-accent text-white" : "bg-panel-2 text-muted"}`}
                onClick={() => setView("canvas")}
              >
                <Network size={15} /> Canvas
              </button>
              <button
                className={`px-3 py-2 text-sm flex items-center gap-1.5 ${view === "form" ? "bg-accent text-white" : "bg-panel-2 text-muted"}`}
                onClick={() => setView("form")}
              >
                <LayoutList size={15} /> Form
              </button>
            </div>
            <button className="btn" onClick={() => exportProjectPdf(project, state)}>
              <FileText size={15} /> PDF
            </button>
            <button className="btn" onClick={() => exportProjectExcel(project, state)}>
              <FileSpreadsheet size={15} /> Excel
            </button>
            <button
              className="btn btn-primary"
              onClick={() => downloadJSON(`${slug(project.name)}.provision.json`, projectToProvision(project, state))}
            >
              <FileJson size={15} /> Provision JSON
            </button>
          </div>
        }
      />

      <div className="flex gap-2 mb-5 text-sm text-muted">
        <Chip>{project.gateways.length} gateways</Chip>
        <Chip>{project.gateways.reduce((n, g) => n + g.buses.length, 0)} buses</Chip>
        <Chip>{totalDevices} end devices</Chip>
      </div>

      {view === "canvas" ? (
        <NetworkCanvas project={project} templates={state.templates} newId={newId} onChange={save} />
      ) : (
        <FormView
          project={project}
          gatewayTpls={gatewayTpls}
          templates={state.templates}
          newId={newId}
          save={save}
        />
      )}
    </>
  );
}

// --------------------------------------------------------------------------
// Form view
// --------------------------------------------------------------------------

function FormView({
  project,
  gatewayTpls,
  templates,
  newId,
  save,
}: {
  project: Project;
  gatewayTpls: DeviceTemplate[];
  templates: DeviceTemplate[];
  newId: (p: string) => string;
  save: (p: Project) => void;
}) {
  return (
    <>
      <Card className="mb-5">
        <span className="label">Add gateway</span>
        <div className="flex flex-wrap gap-2">
          {gatewayTpls.map((t) => (
            <button
              key={t.id}
              className="btn"
              onClick={() => save(edit.addGateway(project, t, newId))}
            >
              <Server size={14} /> {t.name}
            </button>
          ))}
          {gatewayTpls.length === 0 && (
            <span className="text-xs text-muted">No gateway templates — load some in Templates.</span>
          )}
        </div>
      </Card>

      {project.gateways.length === 0 && <Empty>No gateways yet. Add one above.</Empty>}
      <div className="space-y-4">
        {project.gateways.map((gw) => (
          <GatewayCard
            key={gw.id}
            gw={gw}
            templates={templates}
            newId={newId}
            onChange={(g) => save(edit.updateGateway(project, g))}
            onRemove={() => save(edit.removeGateway(project, gw.id))}
          />
        ))}
      </div>
    </>
  );
}

function GatewayCard({
  gw,
  templates,
  newId,
  onChange,
  onRemove,
}: {
  gw: GatewayInstance;
  templates: DeviceTemplate[];
  newId: (p: string) => string;
  onChange: (gw: GatewayInstance) => void;
  onRemove: () => void;
}) {
  const [open, setOpen] = useState(true);
  const tpl = templates.find((t) => t.id === gw.templateId);
  const supported = tpl?.networks ?? [];
  const unusedNetworks = supported.filter((n) => !gw.buses.some((b) => b.network === n));
  const ndev = gw.buses.reduce((m, b) => m + b.devices.length, 0);

  return (
    <Card>
      <div className="flex items-center justify-between">
        <button className="flex items-center gap-2" onClick={() => setOpen(!open)}>
          {open ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          <Server size={16} className="text-accent" />
          <span className="font-medium">{gw.name}</span>
          <Chip>{gw.buses.length} buses</Chip>
          <Chip>{ndev} dev</Chip>
        </button>
        <button className="btn btn-ghost btn-danger" onClick={onRemove}>
          <Trash2 size={14} />
        </button>
      </div>

      {open && (
        <div className="mt-4 space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <Field label="Name">
              <TextInput value={gw.name} onChange={(e) => onChange({ ...gw, name: e.target.value })} />
            </Field>
            <Field label="Uplink address">
              <TextInput
                value={gw.address ?? ""}
                placeholder="IP / host"
                onChange={(e) => onChange({ ...gw, address: e.target.value })}
              />
            </Field>
          </div>

          {tpl && tpl.settings.length > 0 && (
            <div>
              <span className="label">Gateway settings</span>
              <SettingsEditor
                specs={tpl.settings}
                values={gw.settings}
                onChange={(settings) => onChange({ ...gw, settings })}
              />
            </div>
          )}

          {/* Buses */}
          <div className="border-t border-border pt-4 space-y-3">
            <div className="flex items-center justify-between">
              <span className="label !mb-0">Network buses</span>
              {unusedNetworks.length > 0 && (
                <AddBusMenu
                  networks={unusedNetworks}
                  onAdd={(n) => onChange(edit.addBus(gw, n, newId))}
                />
              )}
            </div>
            {gw.buses.map((bus) => (
              <BusCard
                key={bus.id}
                bus={bus}
                templates={templates}
                newId={newId}
                onChange={(b) => onChange(edit.mapBus(gw, bus.id, () => b))}
                onRemove={() => onChange(edit.removeBus(gw, bus.id))}
                onAddDevices={(devs) => onChange(edit.addDevices(gw, bus.id, devs))}
                onUpdateDevice={(d) => onChange(edit.updateDevice(gw, bus.id, d))}
                onRemoveDevice={(id) => onChange(edit.removeDevice(gw, bus.id, id))}
              />
            ))}
          </div>
        </div>
      )}
    </Card>
  );
}

function AddBusMenu({
  networks,
  onAdd,
}: {
  networks: NetworkType[];
  onAdd: (n: NetworkType) => void;
}) {
  const [pick, setPick] = useState("");
  return (
    <div className="flex gap-2 items-center">
      <Select value={pick} onChange={(e) => setPick(e.target.value)} className="w-44">
        <option value="">Add bus…</option>
        {networks.map((n) => (
          <option key={n} value={n}>
            {NETWORK_META[n].label}
          </option>
        ))}
      </Select>
      <button
        className="btn"
        onClick={() => {
          if (pick) {
            onAdd(pick as NetworkType);
            setPick("");
          }
        }}
      >
        <Plus size={14} />
      </button>
    </div>
  );
}

function BusCard({
  bus,
  templates,
  newId,
  onChange,
  onRemove,
  onAddDevices,
  onUpdateDevice,
  onRemoveDevice,
}: {
  bus: NetworkBus;
  templates: DeviceTemplate[];
  newId: (p: string) => string;
  onChange: (b: NetworkBus) => void;
  onRemove: () => void;
  onAddDevices: (d: EndDeviceInstance[]) => void;
  onUpdateDevice: (d: EndDeviceInstance) => void;
  onRemoveDevice: (id: string) => void;
}) {
  const [open, setOpen] = useState(true);
  const meta = NETWORK_META[bus.network];
  const full = isFull(bus);

  return (
    <div className="rounded-lg border" style={{ borderColor: full ? "var(--color-crit)" : "var(--color-border)" }}>
      <div className="flex items-center justify-between px-3 py-2">
        <button className="flex items-center gap-2" onClick={() => setOpen(!open)}>
          {open ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
          <span
            className="w-2.5 h-2.5 rounded-full"
            style={{ background: meta.color }}
          />
          <span className="text-sm font-medium">{meta.label}</span>
          <Chip>
            {bus.devices.length}/{bus.maxDevices}
          </Chip>
          {full && <span className="text-xs text-crit">full</span>}
        </button>
        <div className="flex items-center gap-2">
          <input
            className="input !w-20 !py-1 text-xs"
            type="number"
            title="Max devices on this bus"
            value={bus.maxDevices}
            min={1}
            onChange={(e) => onChange({ ...bus, maxDevices: Math.max(1, Number(e.target.value)) })}
          />
          <button className="btn btn-ghost btn-danger" onClick={onRemove}>
            <Trash2 size={13} />
          </button>
        </div>
      </div>

      {open && (
        <div className="px-3 pb-3 space-y-3">
          <BulkAddPanel bus={bus} templates={templates} newId={newId} onAdd={onAddDevices} />

          {bus.devices.length === 0 ? (
            <p className="text-xs text-muted">No devices on this bus.</p>
          ) : (
            <div className="rounded-lg border border-border overflow-hidden">
              <table className="w-full text-xs">
                <thead className="bg-panel-2 text-muted">
                  <tr>
                    <th className="text-left px-2 py-1">Addr</th>
                    <th className="text-left px-2 py-1">Device</th>
                    <th className="text-left px-2 py-1">Template</th>
                    <th className="px-2 py-1"></th>
                  </tr>
                </thead>
                <tbody>
                  {bus.devices.map((d) => {
                    const dt = templates.find((t) => t.id === d.templateId);
                    return (
                      <tr key={d.id} className="border-t border-border">
                        <td className="px-2 py-1 w-16">
                          {meta.addressed ? (
                            <input
                              className="input !py-0.5 !px-1 !w-14 text-xs"
                              type="number"
                              value={d.address ?? ""}
                              onChange={(e) => onUpdateDevice({ ...d, address: Number(e.target.value) })}
                            />
                          ) : (
                            <input
                              className="input !py-0.5 !px-1 !w-20 text-xs"
                              placeholder="id"
                              value={d.idTag ?? ""}
                              onChange={(e) => onUpdateDevice({ ...d, idTag: e.target.value })}
                            />
                          )}
                        </td>
                        <td className="px-2 py-1">
                          <input
                            className="input !py-0.5 !px-1 text-xs"
                            value={d.name}
                            onChange={(e) => onUpdateDevice({ ...d, name: e.target.value })}
                          />
                        </td>
                        <td className="px-2 py-1 text-muted">{dt?.name ?? "?"}</td>
                        <td className="px-2 py-1 text-right">
                          <button className="btn btn-ghost btn-danger !p-1" onClick={() => onRemoveDevice(d.id)}>
                            <Trash2 size={12} />
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
          <div className="text-[11px] text-muted">
            {freeSlots(bus)} of {bus.maxDevices} slots free ·{" "}
            <Cpu size={11} className="inline -mt-0.5" /> {meta.label} bus
          </div>
        </div>
      )}
    </div>
  );
}
