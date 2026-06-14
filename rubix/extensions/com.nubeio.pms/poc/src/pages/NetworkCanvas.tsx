import { useCallback, useMemo, useRef, useState } from "react";
import {
  ReactFlow,
  Background,
  BackgroundVariant,
  Controls,
  type NodeProps,
  type Node,
  Handle,
  Position,
  ReactFlowProvider,
  useReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Server, Cpu, Trash2, AlertTriangle, GripVertical } from "lucide-react";
import { NETWORK_META, type NetworkType } from "@/types";
import type { Project, DeviceTemplate, GatewayInstance, NetworkBus } from "@/types";
import { Card } from "@/components/ui";
import { BulkAddPanel } from "@/components/BulkAddPanel";
import * as edit from "@/lib/projectEdits";
import { checkDrop, allocAddresses, freeSlots, isFull } from "@/lib/networks";
import { buildLayout } from "@/lib/canvasLayout";

// Drag payload carried via dataTransfer when dragging a template onto a bus.
const DND_TYPE = "application/x-pms-template";

// ---- Custom CAD-style nodes ----------------------------------------------

function GatewayNode({ data }: NodeProps) {
  const gw = data.gw as GatewayInstance;
  return (
    <div
      className="rounded-md w-[230px] overflow-hidden"
      style={{ border: "1.5px solid var(--color-accent)", background: "var(--color-panel)" }}
    >
      <div
        className="px-3 py-1.5 flex items-center gap-2 text-white"
        style={{ background: "var(--color-accent)" }}
      >
        <Server size={15} />
        <span className="text-sm font-semibold tracking-tight">{gw.name}</span>
      </div>
      <div className="px-3 py-2 font-mono text-[10px] text-muted leading-relaxed">
        <div>{String(data.templateName ?? "")}</div>
        <div>
          {gw.address ? `@ ${gw.address}` : "DHCP"} · {gw.buses.length} port(s)
        </div>
      </div>
      <Handle type="source" position={Position.Right} style={handleStyle("var(--color-accent)")} />
    </div>
  );
}

function BusHeadNode({ data }: NodeProps) {
  const bus = data.bus as NetworkBus;
  const meta = NETWORK_META[bus.network];
  const full = isFull(bus);
  const rejection = data.rejection as string | undefined;
  const onSelect = data.onSelect as (busId: string) => void;
  const [over, setOver] = useState(false);
  const pct = Math.min(100, (bus.devices.length / bus.maxDevices) * 100);

  // The actual drop is handled on the ReactFlow wrapper (the pane overlay
  // swallows drops on the node). Here we only mirror the drag-over state for
  // the highlight; `nodrag` lets these DOM events through to us.
  return (
    <div
      className="nodrag rounded-md w-[210px] cursor-pointer overflow-hidden transition-all"
      onClick={() => onSelect(bus.id)}
      onDragOver={(e) => {
        if (e.dataTransfer.types.includes(DND_TYPE)) {
          e.preventDefault();
          setOver(true);
        }
      }}
      onDragLeave={() => setOver(false)}
      onDrop={() => setOver(false)}
      style={{
        border: `1.5px ${over ? "solid" : "solid"} ${
          rejection ? "var(--color-crit)" : over ? meta.color : data.selected ? "var(--color-accent)" : "var(--color-border)"
        }`,
        boxShadow: over ? `0 0 0 3px ${meta.color}44` : data.selected ? `0 0 0 2px var(--color-accent)55` : undefined,
        background: "var(--color-panel)",
      }}
    >
      <Handle type="target" position={Position.Left} style={handleStyle(meta.color)} />
      <div
        className="px-2.5 py-1.5 flex items-center justify-between"
        style={{ background: `${meta.color}22`, borderBottom: `1px solid ${meta.color}55` }}
      >
        <div className="flex items-center gap-1.5">
          <span className="w-2.5 h-2.5 rounded-sm" style={{ background: meta.color }} />
          <span className="text-xs font-semibold">{meta.label}</span>
        </div>
        <span
          className="font-mono text-[10px] px-1 rounded"
          style={{ color: full ? "var(--color-crit)" : meta.color, background: "var(--color-bg)" }}
        >
          {bus.devices.length}/{bus.maxDevices}
        </span>
      </div>
      <div className="px-2.5 py-1.5">
        <div className="flex items-center justify-between text-[9px] font-mono text-muted mb-1">
          <span>{meta.topology === "bus" ? "DAISY-CHAIN" : "STAR"}</span>
          <span>{Math.round(pct)}%</span>
        </div>
        <div className="h-1 rounded-full bg-bg overflow-hidden">
          <div className="h-full" style={{ width: `${pct}%`, background: full ? "var(--color-crit)" : meta.color }} />
        </div>
        {rejection ? (
          <div className="mt-1.5 text-[10px] text-crit flex items-start gap-1 leading-snug">
            <AlertTriangle size={11} className="mt-0.5 shrink-0" />
            {rejection}
          </div>
        ) : (
          <div className="mt-1 text-[9px] text-muted">drop compatible device →</div>
        )}
      </div>
      <Handle type="source" position={Position.Right} style={handleStyle(meta.color)} />
    </div>
  );
}

function DeviceNode({ data }: NodeProps) {
  const dev = data.dev as { id: string; name: string; address?: number; idTag?: string };
  const network = data.network as NetworkType;
  const seq = data.seq as number;
  const meta = NETWORK_META[network];
  const addr = meta.addressed ? dev.address : dev.idTag;
  const onRemove = data.onRemove as (id: string) => void;
  const chain = meta.topology === "bus";

  return (
    <div
      className="rounded-md w-[196px] overflow-hidden group bg-panel"
      style={{ border: `1.5px solid ${meta.color}` }}
    >
      {/* In a daisy chain, the trunk enters left and continues right. */}
      <Handle type="target" position={Position.Left} style={handleStyle(meta.color)} />
      {chain && <Handle type="source" position={Position.Right} style={handleStyle(meta.color)} />}

      <div className="px-2 py-1 flex items-center justify-between" style={{ background: `${meta.color}1a` }}>
        <div className="flex items-center gap-1.5 min-w-0">
          <Cpu size={12} style={{ color: meta.color }} className="shrink-0" />
          <span className="text-[11px] font-medium truncate">{dev.name}</span>
        </div>
        <button
          className="opacity-0 group-hover:opacity-100 text-muted hover:text-crit shrink-0"
          onClick={(e) => {
            e.stopPropagation();
            onRemove(dev.id);
          }}
        >
          <Trash2 size={11} />
        </button>
      </div>
      <div className="px-2 py-1 flex items-center justify-between font-mono text-[9px] text-muted">
        <span>#{seq}</span>
        {addr != null && addr !== "" && (
          <span
            className="px-1 rounded text-[9px]"
            style={{ background: "var(--color-bg)", color: meta.color }}
          >
            {meta.addressed ? `addr ${addr}` : String(addr)}
          </span>
        )}
      </div>
    </div>
  );
}

// End-of-line terminator — the 120Ω resistor symbol that caps an RS-485 trunk.
function TerminatorNode({ data }: NodeProps) {
  const color = data.color as string;
  return (
    <div className="flex items-center" style={{ height: 28 }}>
      <Handle type="target" position={Position.Left} style={handleStyle(color)} />
      <svg width="34" height="28" viewBox="0 0 34 28">
        <line x1="0" y1="14" x2="8" y2="14" stroke={color} strokeWidth="2.5" />
        {/* resistor zigzag */}
        <polyline
          points="8,14 11,7 15,21 19,7 23,21 26,14"
          fill="none"
          stroke={color}
          strokeWidth="2"
        />
        {/* ground / end cap */}
        <line x1="26" y1="6" x2="26" y2="22" stroke={color} strokeWidth="2.5" />
        <line x1="30" y1="9" x2="30" y2="19" stroke={color} strokeWidth="2" />
        <line x1="33" y1="12" x2="33" y2="16" stroke={color} strokeWidth="2" />
      </svg>
    </div>
  );
}

function handleStyle(color: string): React.CSSProperties {
  return { width: 7, height: 7, background: color, border: "none" };
}

const nodeTypes = {
  gateway: GatewayNode,
  bushead: BusHeadNode,
  device: DeviceNode,
  terminator: TerminatorNode,
};

// ---- Legend --------------------------------------------------------------

function Legend({ used }: { used: NetworkType[] }) {
  if (used.length === 0) return null;
  return (
    <div
      className="absolute bottom-3 left-3 z-10 rounded-md px-3 py-2 backdrop-blur"
      style={{ background: "color-mix(in oklab, var(--color-panel) 88%, transparent)", border: "1px solid var(--color-border)" }}
    >
      <div className="text-[9px] font-mono uppercase tracking-wider text-muted mb-1.5">Networks</div>
      <div className="grid grid-cols-1 gap-1">
        {used.map((n) => {
          const m = NETWORK_META[n];
          return (
            <div key={n} className="flex items-center gap-2 text-[10px]">
              <span className="w-4 h-0.5 rounded" style={{ background: m.color }} />
              <span className="font-medium">{m.label}</span>
              <span className="font-mono text-muted">
                {m.topology === "bus" ? "chain" : "star"} · ≤{m.maxDevices}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

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

  const usedNetworks = useMemo(() => {
    const s = new Set<NetworkType>();
    for (const gw of project.gateways) for (const b of gw.buses) s.add(b.network);
    return [...s];
  }, [project]);

  const flash = useCallback((busId: string, reason: string) => {
    setRejection({ busId, reason });
    window.clearTimeout(rejectTimer.current);
    rejectTimer.current = window.setTimeout(() => setRejection(null), 3500);
  }, []);

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
    const g = buildLayout(project, { selectedBusId, rejection, templates });
    for (const n of g.nodes) {
      if (n.type === "bushead") n.data = { ...n.data, onSelect: setSelectedBusId };
      else if (n.type === "device") n.data = { ...n.data, onRemove: removeDevice };
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
                <span className="flex gap-0.5">
                  {t.networks.slice(0, 3).map((n) => (
                    <span key={n} className="w-1.5 h-1.5 rounded-sm" style={{ background: NETWORK_META[n].color }} />
                  ))}
                </span>
              </div>
            ))}
          </div>
          <p className="text-[10px] text-muted mt-2">
            A device only drops onto a bus whose network it supports.
          </p>
        </Card>

        {selected && (
          <Card className="p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="label !mb-0">{NETWORK_META[selected.bus.network].label} bus</span>
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
      <div className="card overflow-hidden relative" style={{ height: 640, background: "#070b12" }}>
        <ReactFlowProvider>
          <FlowCanvas
            nodes={nodes}
            edges={edges}
            usedNetworks={usedNetworks}
            onDropTemplate={(busId, tplId) => {
              const owner = project.gateways.find((g) => g.buses.some((b) => b.id === busId));
              if (owner) addOne(busId, owner.id, tplId);
            }}
          />
        </ReactFlowProvider>
      </div>
    </div>
  );
}

// Inner flow — lives under ReactFlowProvider so it can use the coordinate
// transform. Drop is handled here (not on the bus node) because React Flow's
// `.pane` overlay covers the nodes and swallows their drop events. We map the
// cursor to flow coordinates and hit-test against the bus-head node bounds.
function FlowCanvas({
  nodes,
  edges,
  usedNetworks,
  onDropTemplate,
}: {
  nodes: Node[];
  edges: import("@xyflow/react").Edge[];
  usedNetworks: NetworkType[];
  onDropTemplate: (busId: string, tplId: string) => void;
}) {
  const rf = useReactFlow();

  const busAt = useCallback(
    (clientX: number, clientY: number): string | null => {
      const p = rf.screenToFlowPosition({ x: clientX, y: clientY });
      for (const n of nodes) {
        if (n.type !== "bushead") continue;
        const w = n.measured?.width ?? 210;
        const h = n.measured?.height ?? 96;
        if (p.x >= n.position.x && p.x <= n.position.x + w && p.y >= n.position.y && p.y <= n.position.y + h) {
          return n.id;
        }
      }
      return null;
    },
    [rf, nodes],
  );

  return (
    <>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView
        minZoom={0.2}
        proOptions={{ hideAttribution: true }}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
        panOnScroll
        onDragOver={(e) => {
          if (e.dataTransfer.types.includes(DND_TYPE)) {
            e.preventDefault();
            e.dataTransfer.dropEffect = "copy";
          }
        }}
        onDrop={(e) => {
          const tplId = e.dataTransfer.getData(DND_TYPE);
          if (!tplId) return;
          e.preventDefault();
          const busId = busAt(e.clientX, e.clientY);
          if (busId) onDropTemplate(busId, tplId);
        }}
      >
        <Background variant={BackgroundVariant.Lines} color="#13203a" gap={26} />
        <Background id="fine" variant={BackgroundVariant.Dots} color="#1b2c4a" gap={26 / 4} size={1} />
        <Controls showInteractive={false} />
      </ReactFlow>
      <Legend used={usedNetworks} />
    </>
  );
}
