import { useCallback, useMemo, useRef, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  type Node,
  type Edge,
  type NodeProps,
  Handle,
  Position,
  ReactFlowProvider,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Server, Cpu, Trash2, AlertTriangle, GripVertical } from "lucide-react";
import { NETWORK_META } from "@/types";
import type { Project, DeviceTemplate, GatewayInstance, NetworkBus } from "@/types";
import { Card } from "@/components/ui";
import { BulkAddPanel } from "@/components/BulkAddPanel";
import * as edit from "@/lib/projectEdits";
import { checkDrop, allocAddresses, freeSlots, isFull } from "@/lib/networks";

// Drag payload carried via dataTransfer when dragging a template from the
// palette onto a bus.
const DND_TYPE = "application/x-pms-template";

// ---- Layout: deterministic columns (gateway → bus → devices) -------------

const GW_X = 40;
const BUS_X = 360;
const DEV_X = 680;
const ROW_H = 64;
const GW_GAP = 60;

interface BuildCtx {
  project: Project;
  templates: DeviceTemplate[];
  selectedBusId: string | null;
  rejection: { busId: string; reason: string } | null;
}

function buildGraph(ctx: BuildCtx): { nodes: Node[]; edges: Edge[] } {
  const { project, templates } = ctx;
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  let cursorY = 20;

  for (const gw of project.gateways) {
    const gwTop = cursorY;
    let busY = cursorY;

    const busBlocks = gw.buses.map((bus) => {
      const top = busY;
      const devCount = bus.devices.length;
      const block = Math.max(ROW_H, ROW_H + devCount * (ROW_H - 24));
      busY += block;
      return { bus, top, block };
    });

    const gwHeight = Math.max(ROW_H, busY - gwTop);
    nodes.push({
      id: gw.id,
      position: { x: GW_X, y: gwTop + gwHeight / 2 - 30 },
      data: { gw, templateName: templates.find((t) => t.id === gw.templateId)?.name },
      type: "gateway",
      draggable: false,
    });

    for (const { bus, top } of busBlocks) {
      const isRej = ctx.rejection?.busId === bus.id;
      nodes.push({
        id: bus.id,
        position: { x: BUS_X, y: top },
        data: {
          bus,
          gwId: gw.id,
          selected: ctx.selectedBusId === bus.id,
          rejection: isRej ? ctx.rejection?.reason : undefined,
        },
        type: "bus",
        draggable: false,
      });
      edges.push({
        id: `${gw.id}->${bus.id}`,
        source: gw.id,
        target: bus.id,
        style: { stroke: NETWORK_META[bus.network].color, strokeWidth: 2 },
      });

      bus.devices.forEach((d, i) => {
        nodes.push({
          id: d.id,
          position: { x: DEV_X, y: top + i * (ROW_H - 24) },
          data: { dev: d, network: bus.network, templateName: templates.find((t) => t.id === d.templateId)?.name },
          type: "device",
          draggable: false,
        });
        edges.push({
          id: `${bus.id}->${d.id}`,
          source: bus.id,
          target: d.id,
          style: { stroke: NETWORK_META[bus.network].color, strokeWidth: 1, opacity: 0.6 },
        });
      });
    }

    cursorY = gwTop + gwHeight + GW_GAP;
  }

  return { nodes, edges };
}

// ---- Custom nodes --------------------------------------------------------

function GatewayNode({ data }: NodeProps) {
  const gw = data.gw as GatewayInstance;
  return (
    <div className="card px-3 py-2 w-[280px]" style={{ borderColor: "var(--color-accent)" }}>
      <div className="flex items-center gap-2">
        <Server size={16} className="text-accent" />
        <div className="font-medium text-sm">{gw.name}</div>
      </div>
      <div className="text-[11px] text-muted mt-0.5">
        {String(data.templateName ?? "")} · {gw.buses.length} bus(es)
      </div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

function BusNode({ data }: NodeProps) {
  const bus = data.bus as NetworkBus;
  const meta = NETWORK_META[bus.network];
  const full = isFull(bus);
  const rejection = data.rejection as string | undefined;
  const onDrop = data.onDrop as (busId: string, gwId: string, tplId: string) => void;
  const onSelect = data.onSelect as (busId: string) => void;
  const gwId = data.gwId as string;
  const [over, setOver] = useState<null | "ok" | "bad">(null);

  return (
    <div
      onClick={() => onSelect(bus.id)}
      onDragOver={(e) => {
        e.preventDefault();
        const tplId = e.dataTransfer.types.includes(DND_TYPE);
        setOver(tplId ? "ok" : null);
      }}
      onDragLeave={() => setOver(null)}
      onDrop={(e) => {
        e.preventDefault();
        setOver(null);
        const tplId = e.dataTransfer.getData(DND_TYPE);
        if (tplId) onDrop(bus.id, gwId, tplId);
      }}
      className="card px-3 py-2 w-[280px] cursor-pointer transition-all"
      style={{
        borderColor: rejection
          ? "var(--color-crit)"
          : over === "ok"
            ? meta.color
            : data.selected
              ? "var(--color-accent)"
              : "var(--color-border)",
        boxShadow: over === "ok" ? `0 0 0 2px ${meta.color}55` : undefined,
        background: data.selected ? "var(--color-panel-2)" : undefined,
      }}
    >
      <Handle type="target" position={Position.Left} />
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="w-2.5 h-2.5 rounded-full" style={{ background: meta.color }} />
          <span className="text-sm font-medium">{meta.label}</span>
        </div>
        <span className={`text-xs ${full ? "text-crit" : "text-muted"}`}>
          {bus.devices.length}/{bus.maxDevices}
        </span>
      </div>
      <div className="mt-1.5 h-1 rounded-full bg-bg overflow-hidden">
        <div
          className="h-full rounded-full"
          style={{
            width: `${Math.min(100, (bus.devices.length / bus.maxDevices) * 100)}%`,
            background: full ? "var(--color-crit)" : meta.color,
          }}
        />
      </div>
      {rejection && (
        <div className="mt-1.5 text-[11px] text-crit flex items-start gap-1">
          <AlertTriangle size={12} className="mt-0.5 shrink-0" />
          {rejection}
        </div>
      )}
      {!rejection && <div className="mt-1 text-[10px] text-muted">drop compatible devices here</div>}
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

function DeviceNode({ data }: NodeProps) {
  const dev = data.dev as { id: string; name: string; address?: number; idTag?: string };
  const network = data.network as keyof typeof NETWORK_META;
  const meta = NETWORK_META[network];
  const addr = meta.addressed ? dev.address : dev.idTag;
  const onRemove = data.onRemove as (id: string) => void;
  return (
    <div className="card px-2.5 py-1.5 w-[220px] flex items-center justify-between group">
      <Handle type="target" position={Position.Left} />
      <div className="flex items-center gap-2 min-w-0">
        <Cpu size={13} className="text-accent-2 shrink-0" />
        <span className="text-xs truncate">{dev.name}</span>
      </div>
      <div className="flex items-center gap-1.5 shrink-0">
        {addr != null && addr !== "" && (
          <span className="chip !py-0 !px-1.5 !text-[10px]">{String(addr)}</span>
        )}
        <button
          className="opacity-0 group-hover:opacity-100 text-muted hover:text-crit"
          onClick={() => onRemove(dev.id)}
        >
          <Trash2 size={12} />
        </button>
      </div>
    </div>
  );
}

const nodeTypes = { gateway: GatewayNode, bus: BusNode, device: DeviceNode };

// ---- Main canvas ---------------------------------------------------------

export function NetworkCanvas({
  project,
  templates,
  newId,
  onChange,
}: {
  project: Project;
  templates: DeviceTemplate[];
  newId: (p: string) => string;
  onChange: (p: Project) => void;
}) {
  const [selectedBusId, setSelectedBusId] = useState<string | null>(null);
  const [rejection, setRejection] = useState<{ busId: string; reason: string } | null>(null);
  const rejectTimer = useRef<number | undefined>(undefined);

  const gatewayTpls = templates.filter((t) => t.role === "gateway");
  const deviceTpls = templates.filter((t) => t.role === "end_device");

  const flash = useCallback((busId: string, reason: string) => {
    setRejection({ busId, reason });
    window.clearTimeout(rejectTimer.current);
    rejectTimer.current = window.setTimeout(() => setRejection(null), 3500);
  }, []);

  // Add one device of a template to a bus (drag/drop or click), enforcing
  // compatibility + cap.
  const addOne = useCallback(
    (busId: string, gwId: string, tplId: string) => {
      const tpl = templates.find((t) => t.id === tplId);
      const gw = project.gateways.find((g) => g.id === gwId);
      const bus = gw?.buses.find((b) => b.id === busId);
      if (!tpl || !gw || !bus) return;
      const gate = checkDrop(tpl, bus, 1);
      if (!gate.ok) {
        flash(busId, gate.reason ?? "Cannot add device.");
        return;
      }
      const [addr] = allocAddresses(bus, 1, 1);
      const dev = edit.makeOneDevice(tpl, bus, addr, newId);
      onChange(edit.updateGateway(project, edit.addDevices(gw, busId, [dev])));
    },
    [project, templates, newId, onChange, flash],
  );

  const selected = useMemo(() => {
    for (const gw of project.gateways) {
      const bus = gw.buses.find((b) => b.id === selectedBusId);
      if (bus) return { gw, bus };
    }
    return null;
  }, [project, selectedBusId]);

  const removeDevice = useCallback(
    (devId: string) => {
      for (const gw of project.gateways) {
        for (const bus of gw.buses) {
          if (bus.devices.some((d) => d.id === devId)) {
            onChange(edit.updateGateway(project, edit.removeDevice(gw, bus.id, devId)));
            return;
          }
        }
      }
    },
    [project, onChange],
  );

  const { nodes, edges } = useMemo(() => {
    const g = buildGraph({ project, templates, selectedBusId, rejection });
    // inject callbacks into node data
    for (const n of g.nodes) {
      if (n.type === "bus") {
        n.data = { ...n.data, onDrop: addOne, onSelect: setSelectedBusId };
      } else if (n.type === "device") {
        n.data = { ...n.data, onRemove: removeDevice };
      }
    }
    return g;
  }, [project, templates, selectedBusId, rejection, addOne, removeDevice]);

  if (project.gateways.length === 0) {
    return (
      <Card className="p-6">
        <div className="text-sm text-muted mb-3">No gateways yet. Add one to start the network:</div>
        <div className="flex flex-wrap gap-2">
          {gatewayTpls.map((t) => (
            <button key={t.id} className="btn" onClick={() => onChange(edit.addGateway(project, t, newId))}>
              <Server size={14} /> {t.name}
            </button>
          ))}
        </div>
      </Card>
    );
  }

  return (
    <div className="grid grid-cols-[220px_1fr] gap-4">
      {/* Palette */}
      <div className="space-y-3">
        <Card className="p-3">
          <span className="label">Add gateway</span>
          <div className="flex flex-col gap-1.5">
            {gatewayTpls.map((t) => (
              <button
                key={t.id}
                className="btn justify-start text-xs"
                onClick={() => onChange(edit.addGateway(project, t, newId))}
              >
                <Server size={13} /> {t.name}
              </button>
            ))}
          </div>
        </Card>

        <Card className="p-3">
          <span className="label">Devices · drag to a bus</span>
          <div className="flex flex-col gap-1.5">
            {deviceTpls.map((t) => (
              <div
                key={t.id}
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData(DND_TYPE, t.id);
                  e.dataTransfer.effectAllowed = "copy";
                }}
                className="flex items-center gap-2 px-2 py-1.5 rounded-lg border border-border bg-panel-2 cursor-grab active:cursor-grabbing text-xs"
                title={`Networks: ${t.networks.map((n) => NETWORK_META[n].label).join(", ")}`}
              >
                <GripVertical size={12} className="text-muted" />
                <Cpu size={13} className="text-accent-2" />
                <span className="truncate flex-1">{t.name}</span>
              </div>
            ))}
          </div>
          <p className="text-[10px] text-muted mt-2">
            A device only drops onto a bus whose network it supports.
          </p>
        </Card>

        {/* Bulk add for the selected bus */}
        {selected && (
          <Card className="p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="label !mb-0">
                {NETWORK_META[selected.bus.network].label} bus
              </span>
              <span className="text-[11px] text-muted">{freeSlots(selected.bus)} free</span>
            </div>
            <BulkAddPanel
              bus={selected.bus}
              templates={templates}
              newId={newId}
              onAdd={(devs) =>
                onChange(edit.updateGateway(project, edit.addDevices(selected.gw, selected.bus.id, devs)))
              }
            />
          </Card>
        )}
      </div>

      {/* Flow canvas */}
      <div className="card overflow-hidden" style={{ height: 620 }}>
        <ReactFlowProvider>
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            fitView
            proOptions={{ hideAttribution: true }}
            nodesDraggable={false}
            nodesConnectable={false}
            elementsSelectable
          >
            <Background color="#243049" gap={20} />
            <Controls showInteractive={false} />
          </ReactFlow>
        </ReactFlowProvider>
      </div>
    </div>
  );
}
