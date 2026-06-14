// Framework-agnostic network rules: compatibility, capacity, and address
// allocation for a gateway's field buses. No React in here so the same
// logic backs both the form view and the canvas (and is unit-testable).

import {
  NETWORK_META,
  type NetworkType,
  type NetworkBus,
  type DeviceTemplate,
  type EndDeviceInstance,
} from "@/types";

/** Networks a gateway template can host buses for. */
export function gatewayNetworks(tpl: DeviceTemplate | undefined): NetworkType[] {
  return tpl?.networks ?? [];
}

/** Is this end-device template electrically compatible with the bus? */
export function isCompatible(tpl: DeviceTemplate, bus: NetworkBus): boolean {
  return tpl.role === "end_device" && tpl.networks.includes(bus.network);
}

/** End-device templates that can join the given bus. */
export function compatibleTemplates(
  bus: NetworkBus,
  templates: DeviceTemplate[],
): DeviceTemplate[] {
  return templates.filter((t) => isCompatible(t, bus));
}

/** Remaining device slots on a bus (never negative). */
export function freeSlots(bus: NetworkBus): number {
  return Math.max(0, bus.maxDevices - bus.devices.length);
}

export function isFull(bus: NetworkBus): boolean {
  return freeSlots(bus) === 0;
}

/** Explain why a device can/can't be dropped on a bus. `ok:false` carries
 *  a human reason for the canvas tooltip. */
export function checkDrop(
  tpl: DeviceTemplate,
  bus: NetworkBus,
  count = 1,
): { ok: boolean; reason?: string } {
  if (!isCompatible(tpl, bus)) {
    return {
      ok: false,
      reason: `${tpl.name} is not compatible with a ${NETWORK_META[bus.network].label} bus (supports: ${tpl.networks
        .map((n) => NETWORK_META[n].label)
        .join(", ")}).`,
    };
  }
  if (freeSlots(bus) < count) {
    return {
      ok: false,
      reason: `Bus is full — ${bus.devices.length}/${bus.maxDevices} ${
        NETWORK_META[bus.network].label
      } devices. ${freeSlots(bus)} slot(s) free.`,
    };
  }
  return { ok: true };
}

/** Addresses already taken on a bus. */
export function takenAddresses(bus: NetworkBus): Set<number> {
  const s = new Set<number>();
  for (const d of bus.devices) if (typeof d.address === "number") s.add(d.address);
  return s;
}

/** Allocate `count` free sequential-ish addresses starting at `start`,
 *  skipping ones already in use, clamped to the bus's free slots. */
export function allocAddresses(bus: NetworkBus, count: number, start = 1): number[] {
  const taken = takenAddresses(bus);
  const out: number[] = [];
  let addr = Math.max(1, Math.floor(start));
  const limit = Math.min(count, freeSlots(bus));
  // BACnet MS/TP MAC tops at 127; Modbus at 247 — cap by maxDevices range too.
  const ceiling = Math.max(bus.maxDevices, 254);
  while (out.length < limit && addr <= ceiling) {
    if (!taken.has(addr)) {
      out.push(addr);
      taken.add(addr);
    }
    addr++;
  }
  return out;
}

/** Parse an "a-b" or "a..b" range into a sorted list of numbers. */
export function parseRange(input: string): number[] {
  const m = input.trim().match(/^(\d+)\s*(?:-|\.\.)\s*(\d+)$/);
  if (!m) {
    const single = Number(input.trim());
    return Number.isFinite(single) && single > 0 ? [single] : [];
  }
  const a = Number(m[1]);
  const b = Number(m[2]);
  if (a < 1 || b < a) return [];
  const out: number[] = [];
  for (let i = a; i <= b; i++) out.push(i);
  return out;
}

/** Of a requested address range, return those free on the bus, clamped to
 *  the cap, with the list of rejected (taken/over-cap) addresses. */
export function allocRange(
  bus: NetworkBus,
  range: number[],
): { use: number[]; skipped: number[] } {
  const taken = takenAddresses(bus);
  const free = freeSlots(bus);
  const use: number[] = [];
  const skipped: number[] = [];
  for (const a of range) {
    if (use.length >= free || taken.has(a)) skipped.push(a);
    else {
      use.push(a);
      taken.add(a);
    }
  }
  return { use, skipped };
}

/** Default cap for a freshly-created bus of a network. */
export function defaultCap(network: NetworkType): number {
  return NETWORK_META[network].maxDevices;
}

/** Build a settings map from a template's defaults. */
export function defaultSettings(tpl: DeviceTemplate): EndDeviceInstance["settings"] {
  const v: EndDeviceInstance["settings"] = {};
  for (const s of tpl.settings) if (s.default !== undefined) v[s.key] = s.default;
  return v;
}
