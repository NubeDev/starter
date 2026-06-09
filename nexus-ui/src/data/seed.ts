import type { Dashboard, Widget } from "./types";

let wid = 0;
const w = (
  type: Widget["type"],
  title: string,
  layout: Widget["layout"],
  config: Widget["config"],
  subtitle?: string
): Widget => ({
  id: `w${++wid}`,
  type,
  title,
  subtitle,
  layout,
  config,
});

const EMERALD = "152 76% 44%";
const CYAN = "199 90% 56%";
const VIOLET = "263 80% 66%";
const AMBER = "38 95% 56%";
const ROSE = "346 84% 60%";

export const SEED_DASHBOARDS: Dashboard[] = [
  {
    id: "fleet-overview",
    name: "Fleet Overview",
    slug: "fleet-overview",
    description: "Live health across the entire device estate",
    icon: "LayoutDashboard",
    accent: EMERALD,
    starred: true,
    updatedAt: "2026-06-09T08:12:00Z",
    widgets: [
      w("stat", "Devices Online", { x: 0, y: 0, w: 3, h: 2 }, { metric: "fleet.online", unit: "", color: EMERALD, decimals: 0 }),
      w("stat", "Active Alerts", { x: 3, y: 0, w: 3, h: 2 }, { metric: "fleet.alerts", unit: "", color: ROSE, decimals: 0 }),
      w("stat", "Avg Uptime", { x: 6, y: 0, w: 3, h: 2 }, { metric: "fleet.uptime", unit: "%", color: CYAN, decimals: 2 }),
      w("gauge", "Network Load", { x: 9, y: 0, w: 3, h: 4 }, { metric: "fleet.load", unit: "%", color: AMBER, min: 0, max: 100, warn: 70, crit: 88 }),
      w("line", "Throughput", { x: 0, y: 2, w: 6, h: 4 }, { metric: "fleet.throughput", unit: "req/s", color: EMERALD }, "Messages ingested per second"),
      w("area", "Packet Latency", { x: 6, y: 2, w: 3, h: 4 }, { metric: "fleet.latency", unit: "ms", color: CYAN }),
      w("table", "Devices", { x: 0, y: 6, w: 8, h: 5 }, { metric: "fleet.devices" }, "Last reported telemetry"),
      w("status", "Subsystems", { x: 8, y: 6, w: 4, h: 5 }, { metric: "fleet.subsystems" }),
    ],
  },
  {
    id: "energy-power",
    name: "Energy & Power",
    slug: "energy-power",
    description: "Substation draw, solar yield and battery banks",
    icon: "Zap",
    accent: AMBER,
    starred: true,
    updatedAt: "2026-06-08T19:40:00Z",
    widgets: [
      w("stat", "Grid Draw", { x: 0, y: 0, w: 3, h: 2 }, { metric: "energy.grid", unit: "kW", color: AMBER, decimals: 1 }),
      w("stat", "Solar Yield", { x: 3, y: 0, w: 3, h: 2 }, { metric: "energy.solar", unit: "kW", color: EMERALD, decimals: 1 }),
      w("gauge", "Battery SoC", { x: 6, y: 0, w: 3, h: 4 }, { metric: "energy.soc", unit: "%", color: EMERALD, min: 0, max: 100, warn: 35, crit: 15 }),
      w("gauge", "Transformer Temp", { x: 9, y: 0, w: 3, h: 4 }, { metric: "energy.txtemp", unit: "°C", color: ROSE, min: 0, max: 120, warn: 80, crit: 100 }),
      w("area", "Load vs Generation", { x: 0, y: 2, w: 6, h: 4 }, { metric: "energy.balance", unit: "kW", color: AMBER }, "Rolling 48-min window"),
      w("line", "Power Factor", { x: 0, y: 6, w: 6, h: 4 }, { metric: "energy.pf", unit: "", color: VIOLET }),
      w("table", "Circuits", { x: 6, y: 4, w: 6, h: 6 }, { metric: "energy.circuits" }),
    ],
  },
  {
    id: "cold-chain",
    name: "Cold Chain",
    slug: "cold-chain",
    description: "Refrigeration units, humidity and door events",
    icon: "Snowflake",
    accent: CYAN,
    updatedAt: "2026-06-09T06:02:00Z",
    widgets: [
      w("gauge", "Freezer A", { x: 0, y: 0, w: 3, h: 4 }, { metric: "cold.a", unit: "°C", color: CYAN, min: -30, max: 10, warn: -5, crit: 0 }),
      w("gauge", "Freezer B", { x: 3, y: 0, w: 3, h: 4 }, { metric: "cold.b", unit: "°C", color: CYAN, min: -30, max: 10, warn: -5, crit: 0 }),
      w("stat", "Door Opens", { x: 6, y: 0, w: 3, h: 2 }, { metric: "cold.door", unit: "", color: AMBER, decimals: 0 }),
      w("stat", "Humidity", { x: 9, y: 0, w: 3, h: 2 }, { metric: "cold.humidity", unit: "%", color: VIOLET, decimals: 0 }),
      w("line", "Temperature Trend", { x: 6, y: 2, w: 6, h: 4 }, { metric: "cold.trend", unit: "°C", color: CYAN }, "All units, 48 min"),
      w("status", "Compressors", { x: 0, y: 4, w: 6, h: 4 }, { metric: "cold.compressors" }),
      w("table", "Sensor Log", { x: 6, y: 6, w: 6, h: 4 }, { metric: "cold.sensors" }),
    ],
  },
  {
    id: "air-quality",
    name: "Air Quality",
    slug: "air-quality",
    description: "Particulate, CO₂ and VOC across building zones",
    icon: "Wind",
    accent: VIOLET,
    updatedAt: "2026-06-07T14:25:00Z",
    widgets: [
      w("stat", "PM2.5", { x: 0, y: 0, w: 3, h: 2 }, { metric: "air.pm25", unit: "µg/m³", color: VIOLET, decimals: 0 }),
      w("stat", "CO₂", { x: 3, y: 0, w: 3, h: 2 }, { metric: "air.co2", unit: "ppm", color: AMBER, decimals: 0 }),
      w("gauge", "AQI", { x: 6, y: 0, w: 3, h: 4 }, { metric: "air.aqi", unit: "", color: EMERALD, min: 0, max: 300, warn: 100, crit: 150 }),
      w("gauge", "VOC", { x: 9, y: 0, w: 3, h: 4 }, { metric: "air.voc", unit: "ppb", color: ROSE, min: 0, max: 1000, warn: 400, crit: 700 }),
      w("area", "Particulate Trend", { x: 0, y: 2, w: 6, h: 4 }, { metric: "air.trend", unit: "µg/m³", color: VIOLET }),
      w("table", "Zones", { x: 0, y: 6, w: 12, h: 4 }, { metric: "air.zones" }),
    ],
  },
];

export const ICON_CHOICES = [
  "LayoutDashboard", "Zap", "Snowflake", "Wind", "Activity", "Gauge",
  "Cpu", "Radio", "Thermometer", "Droplets", "SignalHigh", "Factory",
];
export const ACCENT_CHOICES = [EMERALD, CYAN, VIOLET, AMBER, ROSE, "199 90% 56%"];
