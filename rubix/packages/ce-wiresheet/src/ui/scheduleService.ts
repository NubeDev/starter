// Resolve the singleton schedule service (type ending in "schedule.service")
// globally via REST, and read its ref-list output.
//
// Unlike the alarm service — which owns synthesized *records* (full data the FE
// reads straight off the service) — the schedule service emits **refs only**: a
// list of the schedule component uids. The FE resolves each uid's live fields
// (name / path / nextChange / active / out) on demand via `selectNodes`
// (POST /nodes/select), so a renamed or moved schedule reflects immediately and
// the service never carries stale name/path.
//
// The exact output prop name and ref encoding aren't pinned yet (the service
// build isn't on the running engine), so the reader is deliberately tolerant of
// the three plausible shapes — see parseRefUids().

import { getRootNodes, selectNodes, type SelectNodeResult } from "../lib/rest";
import type { Component, Property } from "../lib/engine-types";

const lastSeg = (t: string) => t.toLowerCase().split(/[:/.\\]+/).filter(Boolean).pop() ?? t.toLowerCase();

// type ends in "schedule.service" → last two segments are "schedule","service".
function isScheduleService(type: string): boolean {
  const t = type.toLowerCase();
  return t.endsWith("schedule.service") || lastSeg(type) === "service" && /schedule/.test(t);
}

function find(nodes: Component[]): Component | undefined {
  for (const c of nodes) {
    if (isScheduleService(c.type)) return c;
    const inChild = c.children ? find(c.children) : undefined;
    if (inChild) return inChild;
  }
  return undefined;
}

export async function resolveScheduleService(): Promise<Component | undefined> {
  const r = await getRootNodes({ depth: 3, nested: true });
  return find(r.nodes);
}

// Tolerant ref-list parser. Accepts any of:
//   • JSON number array          → [100623, 100636]
//   • JSON array of {uid} objects → [{ "uid": 100623, … }]
//   • comma/space-separated uids  → "100623,100636" or "100623 100636"
//   • already a JS array/number   (in case the value arrives pre-parsed)
// Returns a de-duped list of positive integer uids in first-seen order.
export function parseRefUids(raw: unknown): number[] {
  const out: number[] = [];
  const push = (n: unknown) => {
    const v = typeof n === "string" ? Number(n.trim()) : typeof n === "number" ? n : NaN;
    if (Number.isInteger(v) && v > 0 && !out.includes(v)) out.push(v);
  };
  const ingest = (val: unknown): void => {
    if (val == null) return;
    if (typeof val === "number") return push(val);
    if (Array.isArray(val)) {
      for (const item of val) {
        if (item && typeof item === "object") push((item as { uid?: unknown }).uid);
        else push(item);
      }
      return;
    }
    if (typeof val === "string") {
      const s = val.trim();
      if (!s) return;
      if (s[0] === "[" || s[0] === "{") {
        try {
          return ingest(JSON.parse(s));
        } catch {
          /* fall through to delimiter split */
        }
      }
      for (const tok of s.split(/[,\s]+/)) push(tok);
    }
  };
  ingest(raw);
  return out;
}

// Read the schedule uids the service currently references. Tries every output-ish
// prop (`schedules`, `refs`, `out`, plus any prop whose value parses to uids) so we
// don't have to pin the name before the service build lands.
const REF_PROP_CANDIDATES = ["schedules", "refs", "scheduleUids", "uids", "out"];
export function readServiceRefUids(service: Component): number[] {
  const props = (service.properties ?? {}) as Record<string, Property>;
  for (const name of REF_PROP_CANDIDATES) {
    const uids = parseRefUids(props[name]?.value);
    if (uids.length) return uids;
  }
  // Last resort: scan every prop for one that parses to a uid list.
  for (const [name, p] of Object.entries(props)) {
    if (name === "__facets") continue;
    const uids = parseRefUids((p as Property)?.value);
    if (uids.length > 1) return uids;
  }
  return [];
}

// One-shot: resolve the service, read its refs, and project the fields a list view
// needs in a single batch call. Returns [] (not throwing) when the service is
// absent so the panel can render an empty/"no service" state.
const LIST_FIELDS = ["name", "path", "type", "parent", "prop:nextChange", "prop:active", "prop:out"];
export async function loadScheduleList(): Promise<SelectNodeResult[]> {
  const service = await resolveScheduleService();
  if (!service) return [];
  const uids = readServiceRefUids(service);
  if (!uids.length) return [];
  const rows = await selectNodes(uids, LIST_FIELDS);
  return rows.filter((r) => !r.missing);
}
