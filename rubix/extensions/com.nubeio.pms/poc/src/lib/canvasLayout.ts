// CAD-style canvas layout: turn a project into React Flow nodes + edges.
//
// Two wiring topologies, drawn differently (NETWORK_META.topology):
//   bus  — daisy chain: gateway → [trunk head] → dev → dev → … → [terminator],
//          laid out left-to-right, joined by a single straight trunk rail.
//   star — point-to-multipoint: gateway → each device on its own branch,
//          fanned vertically.
//
// Pure geometry + data; no React. The component injects callbacks afterwards.

import type { Node, Edge } from "@xyflow/react";
import { MarkerType } from "@xyflow/react";
import { NETWORK_META } from "@/types";
import type { Project, DeviceTemplate, NetworkBus } from "@/types";

// Grid metrics (px). Tuned so the schematic reads cleanly.
const GW_X = 24;
const GW_W = 230;
const HEAD_X = GW_X + GW_W + 90; // trunk head / first branch column
const DEV_W = 196;
const DEV_GAP_X = 56; // horizontal gap between daisy-chained devices
const STAR_DEV_X = HEAD_X + 40;
const ROW_H = 96; // vertical pitch between buses
const STAR_ROW_H = 58; // vertical pitch between star devices
const GW_GAP = 70;

export interface LayoutMeta {
  selectedBusId: string | null;
  rejection: { busId: string; reason: string } | null;
  templates: DeviceTemplate[];
}

function tplName(templates: DeviceTemplate[], id: string): string {
  return templates.find((t) => t.id === id)?.name ?? "?";
}

export function buildLayout(project: Project, meta: LayoutMeta): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  let cursorY = 24;

  for (const gw of project.gateways) {
    const gwTop = cursorY;
    let y = cursorY;

    // measure each bus row height first so we can centre the gateway
    const rows = gw.buses.map((bus) => {
      const topo = NETWORK_META[bus.network].topology;
      const h =
        topo === "star"
          ? Math.max(ROW_H, bus.devices.length * STAR_ROW_H + 28)
          : ROW_H;
      const row = { bus, top: y, h };
      y += h;
      return row;
    });
    const gwHeight = Math.max(ROW_H, y - gwTop);

    nodes.push({
      id: gw.id,
      type: "gateway",
      position: { x: GW_X, y: gwTop + gwHeight / 2 - 34 },
      data: { gw, templateName: tplName(meta.templates, gw.templateId) },
      draggable: false,
      selectable: false,
    });

    for (const { bus, top } of rows) {
      const topo = NETWORK_META[bus.network].topology;
      if (topo === "bus") layoutDaisyChain(nodes, edges, gw.id, bus, top, meta);
      else layoutStar(nodes, edges, gw.id, bus, top, meta);
    }

    cursorY = gwTop + gwHeight + GW_GAP;
  }

  return { nodes, edges };
}

// -- daisy chain -----------------------------------------------------------

function layoutDaisyChain(
  nodes: Node[],
  edges: Edge[],
  gwId: string,
  bus: NetworkBus,
  top: number,
  meta: LayoutMeta,
) {
  const color = NETWORK_META[bus.network].color;
  const railY = top + 30;

  // Trunk head — the bus drop where new devices land + status.
  const headId = bus.id;
  nodes.push({
    id: headId,
    type: "bushead",
    position: { x: HEAD_X, y: top },
    data: {
      bus,
      gwId,
      selected: meta.selectedBusId === bus.id,
      rejection: meta.rejection?.busId === bus.id ? meta.rejection?.reason : undefined,
    },
    draggable: false,
    selectable: false,
  });

  // Gateway → trunk head (the drop line off the controller port).
  edges.push({
    id: `${gwId}->${headId}`,
    source: gwId,
    target: headId,
    type: "smoothstep",
    style: { stroke: color, strokeWidth: 2.5 },
  });

  // Devices in series, left → right, joined node-to-node by a straight rail.
  let prevId = headId;
  bus.devices.forEach((d, i) => {
    const x = HEAD_X + DEV_W + DEV_GAP_X + i * (DEV_W + DEV_GAP_X);
    nodes.push({
      id: d.id,
      type: "device",
      position: { x, y: railY - 26 },
      data: {
        dev: d,
        network: bus.network,
        templateName: tplName(meta.templates, d.templateId),
        seq: i + 1,
      },
      draggable: false,
      selectable: false,
    });
    edges.push({
      id: `${prevId}=>${d.id}`,
      source: prevId,
      target: d.id,
      type: "straight",
      style: { stroke: color, strokeWidth: 2.5 },
    });
    prevId = d.id;
  });

  // Terminator at the end of the trunk (120Ω end-of-line).
  if (bus.devices.length > 0) {
    const termId = `${bus.id}-term`;
    const x = HEAD_X + DEV_W + DEV_GAP_X + bus.devices.length * (DEV_W + DEV_GAP_X);
    nodes.push({
      id: termId,
      type: "terminator",
      position: { x, y: railY - 14 },
      data: { color },
      draggable: false,
      selectable: false,
    });
    edges.push({
      id: `${prevId}=>${termId}`,
      source: prevId,
      target: termId,
      type: "straight",
      style: { stroke: color, strokeWidth: 2.5 },
    });
  }
}

// -- star ------------------------------------------------------------------

function layoutStar(
  nodes: Node[],
  edges: Edge[],
  gwId: string,
  bus: NetworkBus,
  top: number,
  meta: LayoutMeta,
) {
  const color = NETWORK_META[bus.network].color;

  // The bus head sits at the branch point; devices fan out vertically.
  const headId = bus.id;
  nodes.push({
    id: headId,
    type: "bushead",
    position: { x: HEAD_X, y: top },
    data: {
      bus,
      gwId,
      selected: meta.selectedBusId === bus.id,
      rejection: meta.rejection?.busId === bus.id ? meta.rejection?.reason : undefined,
    },
    draggable: false,
    selectable: false,
  });
  edges.push({
    id: `${gwId}->${headId}`,
    source: gwId,
    target: headId,
    type: "smoothstep",
    style: { stroke: color, strokeWidth: 2.5 },
  });

  bus.devices.forEach((d, i) => {
    nodes.push({
      id: d.id,
      type: "device",
      position: { x: STAR_DEV_X + DEV_W + DEV_GAP_X, y: top + 4 + i * STAR_ROW_H },
      data: {
        dev: d,
        network: bus.network,
        templateName: tplName(meta.templates, d.templateId),
        seq: i + 1,
      },
      draggable: false,
      selectable: false,
    });
    edges.push({
      id: `${headId}=>${d.id}`,
      source: headId,
      target: d.id,
      type: "smoothstep",
      style: { stroke: color, strokeWidth: 1.5, opacity: 0.8 },
      markerEnd: { type: MarkerType.ArrowClosed, color },
    });
  });
}
