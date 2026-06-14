// Pure helpers that produce a new Project with one edit applied. Shared by
// the form view and the canvas so both mutate state the same way.

import type {
  Project,
  GatewayInstance,
  NetworkBus,
  EndDeviceInstance,
  NetworkType,
  DeviceTemplate,
} from "@/types";
import { defaultCap, defaultSettings } from "@/lib/networks";
import { NETWORK_META } from "@/types";

type Id = (p: string) => string;

export function addGateway(
  project: Project,
  tpl: DeviceTemplate,
  newId: Id,
  pos?: { x: number; y: number },
): Project {
  // Seed one bus per network the gateway supports — that's the physical
  // reality (the head-end exposes those ports).
  const buses: NetworkBus[] = tpl.networks.map((network) => ({
    id: newId("bus"),
    network,
    maxDevices: defaultCap(network),
    devices: [],
  }));
  const gw: GatewayInstance = {
    id: newId("gw"),
    templateId: tpl.id,
    name: tpl.name,
    settings: defaultSettings(tpl),
    buses,
    pos,
  };
  return { ...project, gateways: [...project.gateways, gw] };
}

export function updateGateway(project: Project, gw: GatewayInstance): Project {
  return { ...project, gateways: project.gateways.map((g) => (g.id === gw.id ? gw : g)) };
}

export function removeGateway(project: Project, gatewayId: string): Project {
  return { ...project, gateways: project.gateways.filter((g) => g.id !== gatewayId) };
}

export function mapBus(
  gw: GatewayInstance,
  busId: string,
  fn: (b: NetworkBus) => NetworkBus,
): GatewayInstance {
  return { ...gw, buses: gw.buses.map((b) => (b.id === busId ? fn(b) : b)) };
}

export function addBus(gw: GatewayInstance, network: NetworkType, newId: Id): GatewayInstance {
  if (gw.buses.some((b) => b.network === network)) return gw; // one bus per network
  return {
    ...gw,
    buses: [
      ...gw.buses,
      { id: newId("bus"), network, maxDevices: defaultCap(network), devices: [] },
    ],
  };
}

export function removeBus(gw: GatewayInstance, busId: string): GatewayInstance {
  return { ...gw, buses: gw.buses.filter((b) => b.id !== busId) };
}

export function addDevices(
  gw: GatewayInstance,
  busId: string,
  devices: EndDeviceInstance[],
): GatewayInstance {
  return mapBus(gw, busId, (b) => ({ ...b, devices: [...b.devices, ...devices] }));
}

export function updateDevice(
  gw: GatewayInstance,
  busId: string,
  dev: EndDeviceInstance,
): GatewayInstance {
  return mapBus(gw, busId, (b) => ({
    ...b,
    devices: b.devices.map((d) => (d.id === dev.id ? dev : d)),
  }));
}

export function removeDevice(gw: GatewayInstance, busId: string, devId: string): GatewayInstance {
  return mapBus(gw, busId, (b) => ({ ...b, devices: b.devices.filter((d) => d.id !== devId) }));
}

/** A single device created from a template with the next free address. */
export function makeOneDevice(
  tpl: DeviceTemplate,
  bus: NetworkBus,
  address: number | undefined,
  newId: Id,
): EndDeviceInstance {
  const addressed = NETWORK_META[bus.network].addressed;
  return {
    id: newId("dev"),
    templateId: tpl.id,
    name: tpl.name,
    address: addressed ? address : undefined,
    idTag: addressed ? undefined : "",
    settings: { ...defaultSettings(tpl) },
  };
}
