// POC domain schema for the BMS / Electrical-EMS project builder.
//
// Hierarchy:  Client → Site → Gateway(network) → EndDevice(network, settings, points)
//
// Templates are admin-authored blueprints. A project instance references a
// template and overrides instance-level fields (name, address, settings).
// On export we flatten this into the rubixos provision shape
// (site → location → device → points).

export type NetworkType =
  | "ethernet"
  | "wifi"
  | "lora"
  | "rs485" // generic 2-wire serial
  | "modbus_rtu" // Modbus over RS-485
  | "modbus_tcp"
  | "bacnet_mstp" // BACnet over RS-485
  | "bacnet_ip"
  | "mbus";

// Per-network metadata: a human label, a default bus device cap, and a
// colour used to tint the bus on the canvas. The cap encodes the
// physical limit of the field bus (e.g. a Modbus RTU segment tops out at
// 32 unit loads, BACnet MS/TP at 127 MAC addresses).
/** Physical wiring topology of a network segment.
 *  - `bus`  — multi-drop serial trunk; devices daisy-chain in series and
 *             the far end is terminated (RS-485, Modbus RTU, BACnet MS/TP, M-Bus).
 *  - `star` — point-to-multipoint; devices hang off the head-end
 *             independently (Ethernet/IP, Wi-Fi, LoRaWAN). */
export type Topology = "bus" | "star";

// Per-network metadata: a human label, a default bus device cap, and a
// colour used to tint the bus on the canvas. The cap encodes the
// physical limit of the field bus (e.g. a Modbus RTU segment tops out at
// 32 unit loads, BACnet MS/TP at 127 MAC addresses).
export interface NetworkMeta {
  label: string;
  /** Default max devices per bus of this network. */
  maxDevices: number;
  /** Whether the network is address-based (Modbus/BACnet) vs id-based (LoRa). */
  addressed: boolean;
  /** Physical wiring — drives daisy-chain vs star rendering on the canvas. */
  topology: Topology;
  color: string; // hex, for the canvas
  short: string; // compact bus label
}

export const NETWORK_META: Record<NetworkType, NetworkMeta> = {
  ethernet: { label: "Ethernet", maxDevices: 254, addressed: false, topology: "star", color: "#64748b", short: "ETH" },
  wifi: { label: "Wi-Fi", maxDevices: 254, addressed: false, topology: "star", color: "#0ea5e9", short: "WIFI" },
  lora: { label: "LoRaWAN", maxDevices: 1000, addressed: false, topology: "star", color: "#22c55e", short: "LORA" },
  rs485: { label: "RS-485", maxDevices: 32, addressed: true, topology: "bus", color: "#f59e0b", short: "485" },
  modbus_rtu: { label: "Modbus RTU", maxDevices: 32, addressed: true, topology: "bus", color: "#f97316", short: "MB-RTU" },
  modbus_tcp: { label: "Modbus TCP", maxDevices: 247, addressed: true, topology: "star", color: "#fb7185", short: "MB-TCP" },
  bacnet_mstp: { label: "BACnet MS/TP", maxDevices: 127, addressed: true, topology: "bus", color: "#a855f7", short: "BAC-MSTP" },
  bacnet_ip: { label: "BACnet/IP", maxDevices: 1000, addressed: true, topology: "star", color: "#8b5cf6", short: "BAC-IP" },
  mbus: { label: "M-Bus", maxDevices: 250, addressed: true, topology: "bus", color: "#14b8a6", short: "MBUS" },
};

export type PointKind = "analog" | "digital" | "counter" | "string";
export type WidgetKind =
  | "gauge"
  | "stat"
  | "led"
  | "toggle"
  | "counter"
  | "battery"
  | "chart";

export type Severity = "info" | "warning" | "critical";

export interface AlarmRule {
  when: string; // e.g. "> 35", "< 5"
  severity: Severity;
  message: string;
}

// ---- Settings descriptors (typed key/value with UI hints) ----------------

export type SettingType = "number" | "text" | "select" | "bool";

export interface SettingSpec {
  key: string;
  label: string;
  type: SettingType;
  unit?: string;
  options?: string[]; // for select
  default?: string | number | boolean;
  help?: string;
}

// ---- Point spec (template-level) -----------------------------------------

export interface PointSpec {
  key: string;
  name: string;
  unit?: string;
  kind: PointKind;
  widget?: WidgetKind;
  writable?: boolean;
  repeat?: number; // expand key1..keyN like rubixos io_22
  trend?: boolean;
  trendInterval?: string;
  // modbus/bacnet addressing hint, optional for the POC
  address?: string;
  alarms?: AlarmRule[];
}

// ---- Templates -----------------------------------------------------------

export type TemplateRole = "gateway" | "end_device";

export interface DeviceTemplate {
  id: string;
  role: TemplateRole;
  name: string; // display name
  vendor?: string;
  model?: string; // matched on provision; like rubixos `template` key
  category?: string; // sensor | controller | meter | gateway ...
  icon?: string; // lucide icon name
  networks: NetworkType[]; // networks this device supports
  settings: SettingSpec[]; // configurable settings
  points: PointSpec[]; // end-device points (gateways may have few/none)
  notes?: string;
}

// ---- Project instances ---------------------------------------------------

export interface Client {
  id: string;
  name: string;
  contact?: string;
  email?: string;
}

export interface SiteInfo {
  id: string;
  clientId: string;
  name: string;
  address?: string;
  lat?: number;
  lng?: number;
  notes?: string;
}

// A configured setting value on an instance.
export type SettingValues = Record<string, string | number | boolean>;

export interface XY {
  x: number;
  y: number;
}

export interface EndDeviceInstance {
  id: string;
  templateId: string;
  name: string;
  /** Numeric field-bus address (Modbus slave id / BACnet MAC). Optional
   *  for id-based networks like LoRa where `idTag` is used instead. */
  address?: number;
  idTag?: string; // DevEUI / serial for non-addressed networks
  settings: SettingValues;
}

/** A single field-bus hanging off a gateway. One network type, its own
 *  device list and device cap. A gateway that speaks BACnet MS/TP *and*
 *  Modbus RTU has two buses; a Modbus-only meter can only join the
 *  Modbus bus. */
export interface NetworkBus {
  id: string;
  network: NetworkType;
  /** Cap for this bus; defaults from NETWORK_META but editable per bus. */
  maxDevices: number;
  devices: EndDeviceInstance[];
}

export interface GatewayInstance {
  id: string;
  templateId: string;
  name: string;
  address?: string; // gateway uplink IP / host
  settings: SettingValues;
  buses: NetworkBus[];
  pos?: XY; // canvas position
}

export interface Project {
  id: string;
  clientId: string;
  siteId: string;
  name: string;
  createdAt: string; // ISO
  gateways: GatewayInstance[];
}

// ---- Whole app state -----------------------------------------------------

export interface AppState {
  clients: Client[];
  sites: SiteInfo[];
  templates: DeviceTemplate[];
  projects: Project[];
}
