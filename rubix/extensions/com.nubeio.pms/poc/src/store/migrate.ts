// Upgrade persisted state to the current schema.
//
// The bus model (Gateway → buses[] → devices[]) superseded the original
// flat shape (Gateway with `network` + `devices[]`). Old localStorage
// blobs predate `buses`, so we lift each legacy gateway into a single bus
// keyed on its old `network`, moving its devices onto that bus.

import type { AppState, GatewayInstance, NetworkBus, NetworkType } from "@/types";
import { NETWORK_META } from "@/types";

// Legacy shapes (only the fields we read).
interface LegacyDevice {
  id: string;
  templateId: string;
  name: string;
  network?: NetworkType;
  address?: string | number;
  settings: Record<string, string | number | boolean>;
}
interface LegacyGateway {
  id: string;
  templateId: string;
  name: string;
  network?: NetworkType;
  address?: string;
  settings: Record<string, string | number | boolean>;
  devices?: LegacyDevice[];
  buses?: NetworkBus[];
}

let counter = 0;
function gid(prefix: string): string {
  // Migration-time ids must be deterministic-free of Date/Math (fine here;
  // this runs in the browser, not a workflow). Keep it simple + unique.
  counter += 1;
  return `${prefix}-mig${counter}-${Math.random().toString(36).slice(2, 7)}`;
}

function defaultCap(network: NetworkType): number {
  return NETWORK_META[network]?.maxDevices ?? 32;
}

function migrateGateway(g: LegacyGateway): GatewayInstance {
  // Already migrated.
  if (Array.isArray(g.buses)) {
    return { ...(g as GatewayInstance), buses: g.buses };
  }

  const network: NetworkType = g.network ?? "modbus_rtu";
  const addressed = NETWORK_META[network]?.addressed ?? true;
  const bus: NetworkBus = {
    id: gid("bus"),
    network,
    maxDevices: defaultCap(network),
    devices: (g.devices ?? []).map((d) => {
      const rawAddr = d.address;
      const numAddr =
        typeof rawAddr === "number"
          ? rawAddr
          : typeof rawAddr === "string" && rawAddr.trim() !== "" && !Number.isNaN(Number(rawAddr))
            ? Number(rawAddr)
            : undefined;
      return {
        id: d.id,
        templateId: d.templateId,
        name: d.name,
        address: addressed ? numAddr : undefined,
        idTag: addressed ? undefined : typeof rawAddr === "string" ? rawAddr : "",
        settings: d.settings ?? {},
      };
    }),
  };

  return {
    id: g.id,
    templateId: g.templateId,
    name: g.name,
    address: g.address,
    settings: g.settings ?? {},
    buses: [bus],
  };
}

/** Bring a parsed state blob up to the current schema. Safe to run on
 *  already-current data (idempotent). */
export function migrateState(raw: unknown): AppState {
  const s = raw as Partial<AppState> & { projects?: { gateways?: LegacyGateway[] }[] };
  const projects = (s.projects ?? []).map((p) => ({
    ...(p as object),
    gateways: ((p.gateways ?? []) as LegacyGateway[]).map(migrateGateway),
  })) as AppState["projects"];

  return {
    clients: s.clients ?? [],
    sites: s.sites ?? [],
    templates: s.templates ?? [],
    projects,
  };
}
