// Bulk-add end devices to a bus — two modes, both compatibility- and
// cap-safe. Returns the new instances plus a summary for the UI.

import type { DeviceTemplate, NetworkBus, EndDeviceInstance } from "@/types";
import { NETWORK_META } from "@/types";
import {
  checkDrop,
  allocAddresses,
  allocRange,
  defaultSettings,
} from "@/lib/networks";

export interface BulkResult {
  devices: EndDeviceInstance[];
  added: number;
  requested: number;
  skipped: number[]; // addresses dropped (taken / over cap)
  reason?: string; // set when nothing could be added
}

function mk(
  tpl: DeviceTemplate,
  bus: NetworkBus,
  address: number,
  idx: number,
  newId: (p: string) => string,
): EndDeviceInstance {
  const addressed = NETWORK_META[bus.network].addressed;
  return {
    id: newId("dev"),
    templateId: tpl.id,
    name: `${tpl.name} ${idx}`,
    address: addressed ? address : undefined,
    idTag: addressed ? undefined : "",
    settings: { ...defaultSettings(tpl) },
  };
}

/** Add `count` devices starting at `startAddr`, skipping taken addresses. */
export function bulkByCount(
  tpl: DeviceTemplate,
  bus: NetworkBus,
  count: number,
  startAddr: number,
  newId: (p: string) => string,
): BulkResult {
  const gate = checkDrop(tpl, bus, 1);
  if (!gate.ok) return { devices: [], added: 0, requested: count, skipped: [], reason: gate.reason };

  const addrs = allocAddresses(bus, count, startAddr);
  const base = bus.devices.length;
  const devices = addrs.map((a, i) => mk(tpl, bus, a, base + i + 1, newId));
  return {
    devices,
    added: devices.length,
    requested: count,
    skipped: [],
  };
}

/** Fill an explicit address range (e.g. 1-32) with one template. */
export function bulkByRange(
  tpl: DeviceTemplate,
  bus: NetworkBus,
  range: number[],
  newId: (p: string) => string,
): BulkResult {
  const gate = checkDrop(tpl, bus, 1);
  if (!gate.ok) return { devices: [], added: 0, requested: range.length, skipped: range, reason: gate.reason };

  const { use, skipped } = allocRange(bus, range);
  const base = bus.devices.length;
  const devices = use.map((a, i) => mk(tpl, bus, a, base + i + 1, newId));
  return {
    devices,
    added: devices.length,
    requested: range.length,
    skipped,
  };
}
