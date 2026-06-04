// Flatten a POC Project into the rubixos provision import shape.
//
// rubixos provisions: site → location → device → points (+ alarms).
// A gateway lives at a site, so we map:
//   Project   → site
//   Gateway   → location (the gateway *is* the site node) + its own device row
//   NetworkBus→ a bus segment inside the location (network + cap)
//   EndDevice → device under its bus
//   point     → point rows (repeat-expanded, like rubixos)
//
// The resulting object is intended to be read by
// process/src/provision as a site bundle.

import type {
  Project,
  AppState,
  DeviceTemplate,
  EndDeviceInstance,
  GatewayInstance,
  NetworkBus,
  PointSpec,
} from "@/types";

export interface ProvisionPoint {
  key: string;
  name: string;
  unit?: string;
  kind: string;
  widget?: string;
  writable: boolean;
  trend: boolean;
  trend_interval?: string;
  address?: string;
  alarms: { when: string; severity: string; message: string }[];
}

export interface ProvisionDevice {
  device_id: string;
  template: string; // model key (matches rubixos `template`)
  name: string;
  category?: string;
  address?: string | number;
  settings: Record<string, string | number | boolean>;
  points: ProvisionPoint[];
}

export interface ProvisionBus {
  bus_id: string;
  network: string;
  max_devices: number;
  device_count: number;
  devices: ProvisionDevice[];
}

export interface ProvisionLocation {
  location_id: string;
  name: string;
  // The gateway itself, modelled as a device on the location.
  gateway: ProvisionDevice;
  buses: ProvisionBus[];
}

export interface ProvisionSite {
  schema: "rubix.provision/v1";
  site_id: string;
  name: string;
  address?: string;
  lat?: number;
  lng?: number;
  client: string;
  locations: ProvisionLocation[];
}

function expandPoints(specs: PointSpec[]): ProvisionPoint[] {
  const out: ProvisionPoint[] = [];
  for (const s of specs) {
    const make = (key: string, name: string): ProvisionPoint => ({
      key,
      name,
      unit: s.unit,
      kind: s.kind,
      widget: s.widget,
      writable: !!s.writable,
      trend: !!s.trend,
      trend_interval: s.trendInterval,
      address: s.address,
      alarms: (s.alarms ?? []).map((a) => ({ ...a })),
    });
    if (s.repeat && s.repeat > 1) {
      for (let i = 1; i <= s.repeat; i++) out.push(make(`${s.key}${i}`, `${s.name} ${i}`));
    } else {
      out.push(make(s.key, s.name));
    }
  }
  return out;
}

function deviceFromInstance(
  inst: EndDeviceInstance | GatewayInstance,
  tpl: DeviceTemplate,
): ProvisionDevice {
  const addr =
    "address" in inst && inst.address != null
      ? inst.address
      : "idTag" in inst && inst.idTag
        ? inst.idTag
        : undefined;
  return {
    device_id: inst.id,
    template: tpl.model ?? tpl.id,
    name: inst.name,
    category: tpl.category,
    address: addr,
    settings: inst.settings,
    points: expandPoints(tpl.points),
  };
}

function busToProvision(
  bus: NetworkBus,
  tplById: Map<string, DeviceTemplate>,
): ProvisionBus {
  const devices = bus.devices
    .map((d) => {
      const t = tplById.get(d.templateId);
      return t ? deviceFromInstance(d, t) : null;
    })
    .filter((d): d is ProvisionDevice => d !== null);
  return {
    bus_id: bus.id,
    network: bus.network,
    max_devices: bus.maxDevices,
    device_count: devices.length,
    devices,
  };
}

export function projectToProvision(project: Project, state: AppState): ProvisionSite {
  const site = state.sites.find((s) => s.id === project.siteId);
  const client = state.clients.find((c) => c.id === project.clientId);
  const tplById = new Map(state.templates.map((t) => [t.id, t]));

  const locations: ProvisionLocation[] = project.gateways.map((gw) => {
    const gwTpl = tplById.get(gw.templateId);
    return {
      location_id: gw.id,
      name: gw.name,
      gateway: gwTpl
        ? deviceFromInstance(gw, gwTpl)
        : {
            device_id: gw.id,
            template: "unknown",
            name: gw.name,
            settings: gw.settings,
            points: [],
          },
      buses: gw.buses.map((b) => busToProvision(b, tplById)),
    };
  });

  return {
    schema: "rubix.provision/v1",
    site_id: project.siteId,
    name: site?.name ?? project.name,
    address: site?.address,
    lat: site?.lat,
    lng: site?.lng,
    client: client?.name ?? project.clientId,
    locations,
  };
}

export function downloadJSON(filename: string, data: unknown): void {
  const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
  triggerDownload(filename, blob);
}

export function triggerDownload(filename: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
