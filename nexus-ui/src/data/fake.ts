// Deterministic fake telemetry — same metric key always yields the same shape,
// so charts stay stable across re-renders while still looking organic. A small
// live jitter is layered on top by widgets that opt into real-time updates.

function hashString(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

/** Mulberry32 seeded PRNG → reproducible series per metric. */
function rng(seed: number) {
  return function () {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export interface Point {
  t: string;
  v: number;
}

export interface SeriesOpts {
  points?: number;
  base?: number;
  amplitude?: number;
  noise?: number;
  trend?: number;
  min?: number;
  max?: number;
}

export function series(metric: string, opts: SeriesOpts = {}): Point[] {
  const {
    points = 48,
    base = 50,
    amplitude = 18,
    noise = 6,
    trend = 0,
    min = -Infinity,
    max = Infinity,
  } = opts;
  const rand = rng(hashString(metric));
  const phase = rand() * Math.PI * 2;
  const freq = 0.18 + rand() * 0.16;
  const out: Point[] = [];
  const now = Date.now();
  for (let i = 0; i < points; i++) {
    const wave = Math.sin(i * freq + phase) * amplitude;
    const wobble = Math.sin(i * freq * 2.7 + phase) * amplitude * 0.25;
    const n = (rand() - 0.5) * 2 * noise;
    const v = clamp(base + wave + wobble + n + (i / points) * trend, min, max);
    const ts = new Date(now - (points - i) * 60_000);
    out.push({
      t: `${pad(ts.getHours())}:${pad(ts.getMinutes())}`,
      v: round(v, 1),
    });
  }
  return out;
}

export function gaugeValue(metric: string, min = 0, max = 100): number {
  const rand = rng(hashString(metric + ":g"));
  return round(min + rand() * (max - min), 0);
}

export function statValue(
  metric: string,
  base: number,
  spread: number
): { value: number; deltaPct: number } {
  const rand = rng(hashString(metric + ":s"));
  const value = round(base + (rand() - 0.4) * spread, base > 100 ? 0 : 1);
  const deltaPct = round((rand() - 0.45) * 24, 1);
  return { value, deltaPct };
}

export interface DeviceRow {
  id: string;
  name: string;
  site: string;
  status: "online" | "degraded" | "offline";
  signal: number;
  battery: number;
  lastSeen: string;
}

const SITES = ["Hangar A", "North Yard", "Rooftop", "Cold Store", "Substation", "Dock 4"];
const NAMES = ["Sensor", "Gateway", "Meter", "Controller", "Probe", "Beacon", "Relay", "Node"];

export function devices(metric: string, count = 8): DeviceRow[] {
  const rand = rng(hashString(metric + ":d"));
  const rows: DeviceRow[] = [];
  for (let i = 0; i < count; i++) {
    const r = rand();
    const status: DeviceRow["status"] =
      r > 0.82 ? "offline" : r > 0.66 ? "degraded" : "online";
    rows.push({
      id: `DV-${(1000 + Math.floor(rand() * 8999)).toString()}`,
      name: `${NAMES[Math.floor(rand() * NAMES.length)]}-${pad(i + 1)}`,
      site: SITES[Math.floor(rand() * SITES.length)],
      status,
      signal: Math.floor(40 + rand() * 60),
      battery: Math.floor(rand() * 100),
      lastSeen: status === "offline" ? `${Math.floor(rand() * 48) + 1}h ago` : `${Math.floor(rand() * 59)}s ago`,
    });
  }
  return rows;
}

export function jitter(v: number, pct = 0.03): number {
  return round(v * (1 + (Math.random() - 0.5) * pct), 1);
}

function clamp(v: number, lo: number, hi: number) {
  return Math.min(hi, Math.max(lo, v));
}
function round(v: number, d: number) {
  const p = Math.pow(10, d);
  return Math.round(v * p) / p;
}
function pad(n: number) {
  return n.toString().padStart(2, "0");
}
