import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";
import type { Project, AppState } from "@/types";
import { projectToProvision } from "./provision";
import { slug } from "./exportExcel";

// Human-readable project document: cover, site summary, per-gateway
// device schedule, and a full points list.

export function exportProjectPdf(project: Project, state: AppState): void {
  const prov = projectToProvision(project, state);
  const doc = new jsPDF({ unit: "pt", format: "a4" });
  const W = doc.internal.pageSize.getWidth();
  let y = 56;

  // Cover band
  doc.setFillColor(31, 41, 55);
  doc.rect(0, 0, W, 90, "F");
  doc.setTextColor(255, 255, 255);
  doc.setFontSize(20);
  doc.text("Project Design Document", 40, 46);
  doc.setFontSize(11);
  doc.setTextColor(180, 190, 210);
  doc.text(`${project.name}  ·  ${prov.client}`, 40, 68);

  y = 120;
  doc.setTextColor(20, 20, 20);
  doc.setFontSize(13);
  doc.text("Site", 40, y);
  y += 8;

  autoTable(doc, {
    startY: y,
    theme: "plain",
    styles: { fontSize: 9, cellPadding: 3 },
    body: [
      ["Site", prov.name],
      ["Address", prov.address ?? "—"],
      ["Coordinates", prov.lat != null ? `${prov.lat}, ${prov.lng}` : "—"],
      ["Gateways", String(prov.locations.length)],
      [
        "End Devices",
        String(
          prov.locations.reduce(
            (n, l) => n + l.buses.reduce((m, b) => m + b.devices.length, 0),
            0,
          ),
        ),
      ],
      ["Generated", project.createdAt],
    ],
    columnStyles: { 0: { fontStyle: "bold", cellWidth: 120, textColor: [90, 90, 90] } },
  });
  // @ts-expect-error lastAutoTable is attached at runtime
  y = doc.lastAutoTable.finalY + 24;

  for (const l of prov.locations) {
    if (y > 720) {
      doc.addPage();
      y = 56;
    }
    doc.setFontSize(12);
    doc.setTextColor(31, 41, 55);
    doc.text(`Gateway: ${l.gateway.name}`, 40, y);
    doc.setFontSize(9);
    doc.setTextColor(120, 120, 120);
    doc.text(
      `${l.gateway.template} · ${l.buses.length} bus(es)${l.gateway.address ? " · " + l.gateway.address : ""}`,
      40,
      y + 14,
    );
    y += 26;

    for (const b of l.buses) {
      if (y > 720) {
        doc.addPage();
        y = 56;
      }
      doc.setFontSize(10);
      doc.setTextColor(70, 70, 90);
      doc.text(
        `Bus · ${b.network}  (${b.device_count}/${b.max_devices} devices)`,
        48,
        y,
      );
      y += 6;
      autoTable(doc, {
        startY: y,
        head: [["Device", "Template", "Address", "Points"]],
        body: b.devices.length
          ? b.devices.map((d) => [d.name, d.template, String(d.address ?? "—"), String(d.points.length)])
          : [["— no devices —", "", "", ""]],
        headStyles: { fillColor: [59, 130, 246], fontSize: 9 },
        styles: { fontSize: 9, cellPadding: 4 },
        margin: { left: 48 },
      });
      // @ts-expect-error runtime
      y = doc.lastAutoTable.finalY + 14;
    }
    y += 6;
  }

  // Points schedule (all devices)
  doc.addPage();
  y = 56;
  doc.setFontSize(13);
  doc.setTextColor(20, 20, 20);
  doc.text("Points Schedule", 40, y);
  y += 10;

  const rows: string[][] = [];
  for (const l of prov.locations) {
    for (const b of l.buses) {
      for (const d of b.devices) {
        for (const p of d.points) {
          rows.push([
            d.name,
            p.key,
            p.name,
            p.unit ?? "",
            p.kind,
            p.writable ? "W" : "R",
            p.alarms.length ? `${p.alarms.length} alarm(s)` : "",
          ]);
        }
      }
    }
  }
  autoTable(doc, {
    startY: y,
    head: [["Device", "Key", "Name", "Unit", "Kind", "RW", "Alarms"]],
    body: rows,
    headStyles: { fillColor: [34, 211, 238], textColor: 20, fontSize: 8 },
    styles: { fontSize: 8, cellPadding: 3 },
  });

  doc.save(`${slug(project.name)}.pdf`);
}
