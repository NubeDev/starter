import ExcelJS from "exceljs";
import type { Project, AppState } from "@/types";
import { projectToProvision } from "./provision";
import { triggerDownload } from "./provision";

// Multi-sheet workbook: Site / Gateways / Devices / Points.
// Flat tabular form, easy to eyeball and re-import.

export async function exportProjectExcel(project: Project, state: AppState): Promise<void> {
  const prov = projectToProvision(project, state);
  const wb = new ExcelJS.Workbook();
  wb.creator = "Rubix PMS POC";

  const headerStyle = (ws: ExcelJS.Worksheet) => {
    ws.getRow(1).font = { bold: true, color: { argb: "FFFFFFFF" } };
    ws.getRow(1).fill = { type: "pattern", pattern: "solid", fgColor: { argb: "FF1F2937" } };
  };

  // -- Site sheet --
  const siteWs = wb.addWorksheet("Site");
  siteWs.columns = [
    { header: "Field", key: "f", width: 18 },
    { header: "Value", key: "v", width: 50 },
  ];
  siteWs.addRows([
    { f: "Project", v: project.name },
    { f: "Client", v: prov.client },
    { f: "Site", v: prov.name },
    { f: "Address", v: prov.address ?? "" },
    { f: "Lat/Lng", v: prov.lat != null ? `${prov.lat}, ${prov.lng}` : "" },
    { f: "Gateways", v: prov.locations.length },
    { f: "Buses", v: prov.locations.reduce((n, l) => n + l.buses.length, 0) },
    {
      f: "End Devices",
      v: prov.locations.reduce(
        (n, l) => n + l.buses.reduce((m, b) => m + b.devices.length, 0),
        0,
      ),
    },
    { f: "Exported", v: project.createdAt },
  ]);
  headerStyle(siteWs);

  // -- Gateways sheet --
  const gwWs = wb.addWorksheet("Gateways");
  gwWs.columns = [
    { header: "Gateway", key: "name", width: 26 },
    { header: "Template", key: "tpl", width: 18 },
    { header: "Address", key: "addr", width: 18 },
    { header: "Buses", key: "nbus", width: 8 },
    { header: "Settings", key: "settings", width: 50 },
    { header: "Devices", key: "ndev", width: 10 },
  ];
  for (const l of prov.locations) {
    gwWs.addRow({
      name: l.gateway.name,
      tpl: l.gateway.template,
      addr: l.gateway.address ?? "",
      nbus: l.buses.length,
      settings: kv(l.gateway.settings),
      ndev: l.buses.reduce((m, b) => m + b.devices.length, 0),
    });
  }
  headerStyle(gwWs);

  // -- Buses sheet --
  const busWs = wb.addWorksheet("Buses");
  busWs.columns = [
    { header: "Gateway", key: "gw", width: 24 },
    { header: "Network", key: "net", width: 16 },
    { header: "Devices", key: "ndev", width: 10 },
    { header: "Max", key: "max", width: 8 },
    { header: "Utilisation", key: "util", width: 14 },
  ];
  for (const l of prov.locations) {
    for (const b of l.buses) {
      busWs.addRow({
        gw: l.gateway.name,
        net: b.network,
        ndev: b.device_count,
        max: b.max_devices,
        util: `${Math.round((b.device_count / b.max_devices) * 100)}%`,
      });
    }
  }
  headerStyle(busWs);

  // -- Devices sheet --
  const devWs = wb.addWorksheet("Devices");
  devWs.columns = [
    { header: "Gateway", key: "gw", width: 24 },
    { header: "Bus", key: "net", width: 14 },
    { header: "Device", key: "name", width: 26 },
    { header: "Template", key: "tpl", width: 18 },
    { header: "Category", key: "cat", width: 14 },
    { header: "Address", key: "addr", width: 16 },
    { header: "Settings", key: "settings", width: 44 },
    { header: "Points", key: "npts", width: 8 },
  ];
  for (const l of prov.locations) {
    for (const b of l.buses) {
      for (const d of b.devices) {
        devWs.addRow({
          gw: l.gateway.name,
          net: b.network,
          name: d.name,
          tpl: d.template,
          cat: d.category ?? "",
          addr: d.address ?? "",
          settings: kv(d.settings),
          npts: d.points.length,
        });
      }
    }
  }
  headerStyle(devWs);

  // -- Points sheet --
  const ptWs = wb.addWorksheet("Points");
  ptWs.columns = [
    { header: "Gateway", key: "gw", width: 22 },
    { header: "Device", key: "dev", width: 24 },
    { header: "Point Key", key: "key", width: 14 },
    { header: "Name", key: "name", width: 22 },
    { header: "Unit", key: "unit", width: 8 },
    { header: "Kind", key: "kind", width: 10 },
    { header: "Widget", key: "widget", width: 10 },
    { header: "Writable", key: "w", width: 9 },
    { header: "Trend", key: "trend", width: 8 },
    { header: "Address", key: "addr", width: 10 },
    { header: "Alarms", key: "alarms", width: 40 },
  ];
  for (const l of prov.locations) {
    for (const b of l.buses) {
      for (const d of b.devices) {
        for (const p of d.points) {
          ptWs.addRow({
            gw: l.gateway.name,
            dev: d.name,
            key: p.key,
            name: p.name,
            unit: p.unit ?? "",
            kind: p.kind,
            widget: p.widget ?? "",
            w: p.writable ? "yes" : "",
            trend: p.trend ? "yes" : "",
            addr: p.address ?? "",
            alarms: p.alarms.map((a) => `${a.when} ${a.severity}: ${a.message}`).join(" | "),
          });
        }
      }
    }
  }
  headerStyle(ptWs);

  const buf = await wb.xlsx.writeBuffer();
  triggerDownload(
    `${slug(project.name)}.xlsx`,
    new Blob([buf], { type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" }),
  );
}

function kv(obj: Record<string, string | number | boolean>): string {
  return Object.entries(obj)
    .map(([k, v]) => `${k}=${v}`)
    .join(", ");
}

export function slug(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "") || "project";
}
