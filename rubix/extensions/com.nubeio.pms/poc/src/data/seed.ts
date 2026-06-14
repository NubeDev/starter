import type { AppState, DeviceTemplate, Client, SiteInfo } from "@/types";

// Admin-loaded blueprints. In the real product these come from the
// rubixos templates/*.yaml; here we ship a representative BMS/EMS set.

const gateways: DeviceTemplate[] = [
  {
    id: "tpl-gw-edge",
    role: "gateway",
    name: "Rubix Edge Gateway",
    vendor: "Nube iO",
    model: "rubix_edge",
    category: "gateway",
    icon: "server",
    networks: ["ethernet", "wifi", "lora", "modbus_rtu", "bacnet_mstp"],
    settings: [
      { key: "uplink", label: "Uplink", type: "select", options: ["ethernet", "wifi", "4g"], default: "ethernet" },
      { key: "static_ip", label: "Static IP", type: "text", help: "Leave blank for DHCP" },
      { key: "lora_region", label: "LoRa Region", type: "select", options: ["AU915", "EU868", "US915"], default: "AU915" },
    ],
    points: [],
    notes: "Primary site controller / network head-end. Hosts LoRa, Modbus RTU and BACnet MS/TP buses.",
  },
  {
    id: "tpl-gw-485",
    role: "gateway",
    name: "RS-485 / Modbus Gateway",
    vendor: "Generic",
    model: "modbus_gw",
    category: "gateway",
    icon: "cable",
    networks: ["modbus_rtu", "modbus_tcp", "ethernet"],
    settings: [
      { key: "baud", label: "Baud Rate", type: "select", options: ["9600", "19200", "38400", "115200"], default: "9600" },
      { key: "parity", label: "Parity", type: "select", options: ["none", "even", "odd"], default: "none" },
      { key: "tcp_port", label: "Modbus TCP Port", type: "number", default: 502 },
    ],
    points: [],
  },
  {
    id: "tpl-gw-lora",
    role: "gateway",
    name: "LoRaWAN Gateway",
    vendor: "Generic",
    model: "lora_gw",
    category: "gateway",
    icon: "radio",
    networks: ["lora", "ethernet"],
    settings: [
      { key: "lora_region", label: "Region", type: "select", options: ["AU915", "EU868", "US915"], default: "AU915" },
      { key: "ns_url", label: "Network Server URL", type: "text" },
    ],
    points: [],
  },
];

const endDevices: DeviceTemplate[] = [
  {
    id: "tpl-meter-3ph",
    role: "end_device",
    name: "3-Phase Energy Meter",
    vendor: "Generic",
    model: "energy_meter_3ph",
    category: "meter",
    icon: "zap",
    networks: ["modbus_rtu", "modbus_tcp"],
    settings: [
      { key: "slave_id", label: "Modbus Address", type: "number", default: 1 },
      { key: "ct_ratio", label: "CT Ratio", type: "text", default: "100/5", help: "Current transformer ratio" },
    ],
    points: [
      { key: "kwh", name: "Active Energy", unit: "kWh", kind: "counter", widget: "counter", trend: true, trendInterval: "5m", address: "40001" },
      { key: "kw", name: "Active Power", unit: "kW", kind: "analog", widget: "stat", trend: true, trendInterval: "1m", address: "40003" },
      { key: "voltage", name: "Voltage", unit: "V", kind: "analog", widget: "gauge", trend: true, address: "40005" },
      { key: "current", name: "Current", unit: "A", kind: "analog", widget: "gauge", trend: true, address: "40007" },
      { key: "pf", name: "Power Factor", unit: "", kind: "analog", widget: "stat", trend: false, address: "40009" },
    ],
  },
  {
    id: "tpl-droplet",
    role: "end_device",
    name: "Droplet Wall Sensor",
    vendor: "Nube iO",
    model: "droplet",
    category: "sensor",
    icon: "droplet",
    networks: ["lora"],
    settings: [
      { key: "dev_eui", label: "DevEUI", type: "text", help: "16-hex LoRa identifier" },
      { key: "interval", label: "Report Interval", type: "select", options: ["1m", "5m", "15m"], default: "5m" },
    ],
    points: [
      { key: "temp", name: "Temperature", unit: "°C", kind: "analog", widget: "gauge", trend: true, trendInterval: "5m", alarms: [{ when: "> 35", severity: "warning", message: "High temperature" }, { when: "< 5", severity: "warning", message: "Low temperature" }] },
      { key: "humidity", name: "Humidity", unit: "%RH", kind: "analog", widget: "gauge", trend: true, trendInterval: "5m" },
      { key: "battery", name: "Battery", unit: "%", kind: "analog", widget: "battery", trend: false, alarms: [{ when: "< 20", severity: "warning", message: "Low battery" }, { when: "< 5", severity: "critical", message: "Battery critical" }] },
    ],
  },
  {
    id: "tpl-io22",
    role: "end_device",
    name: "IO-22 Controller",
    vendor: "Nube iO",
    model: "io_22",
    category: "controller",
    icon: "toggle-right",
    networks: ["modbus_rtu", "modbus_tcp"],
    settings: [{ key: "slave_id", label: "Modbus Address", type: "number", default: 1 }],
    points: [
      { key: "di", name: "DI", kind: "digital", widget: "led", repeat: 12, trend: false },
      { key: "do", name: "DO", kind: "digital", widget: "toggle", writable: true, repeat: 10, trend: false },
    ],
  },
  {
    id: "tpl-vav",
    role: "end_device",
    name: "VAV Controller",
    vendor: "Generic",
    model: "vav_ctrl",
    category: "controller",
    icon: "wind",
    networks: ["bacnet_mstp", "bacnet_ip"],
    settings: [
      { key: "instance", label: "BACnet Instance", type: "number", default: 1001 },
      { key: "zone", label: "Zone Name", type: "text" },
    ],
    points: [
      { key: "room_temp", name: "Room Temp", unit: "°C", kind: "analog", widget: "gauge", trend: true },
      { key: "setpoint", name: "Setpoint", unit: "°C", kind: "analog", widget: "stat", writable: true, trend: true },
      { key: "damper", name: "Damper Position", unit: "%", kind: "analog", widget: "gauge", trend: true },
      { key: "occupancy", name: "Occupancy", kind: "digital", widget: "led", trend: false },
    ],
  },
];

const clients: Client[] = [
  { id: "cli-acme", name: "Acme Property Group", contact: "Jane Doe", email: "jane@acme.example" },
];

const sites: SiteInfo[] = [
  { id: "site-tower", clientId: "cli-acme", name: "Acme Tower", address: "1 George St, Sydney", lat: -33.8688, lng: 151.2093 },
];

export const SEED: AppState = {
  clients,
  sites,
  templates: [...gateways, ...endDevices],
  projects: [],
};
