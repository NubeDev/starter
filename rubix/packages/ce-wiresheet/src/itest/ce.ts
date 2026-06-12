// Helpers for the live-engine integration tests (*.itest.ts). These drive the
// REAL Control Engine through the same lib the editor uses (rest.ts) and read
// computed values off the binary WebSocket with the real decoder (wire.ts), so
// the whole REST-mutate → engine-compute → WS-stream path is exercised end to end.
//
// Requires a running CE with the NubeIO math extension. Point at it with CE_BASE
// (default http://127.0.0.1:7878). Suites skip themselves when it's unreachable.

import { setEngineBase, getRootNodes, addNode, removeNode } from "../lib/rest";
import { decodeBinaryFrame, type DecodedValue } from "../lib/wire";
import { TYPE_STATUS } from "../lib/engine-types";

export const CE_BASE = process.env.CE_BASE ?? "http://127.0.0.1:7878";
export const ADD = "NubeIO-math::add";
export const SUBTRACT = "NubeIO-math::subtract";

// All test components live under this folder so the suite never touches anything
// else on the engine. "Clean slate between tests" = delete + recreate this folder.
const TEST_FOLDER = "__ce_itest__";

// Is a CE answering at CE_BASE? Drives describe.skipIf so the suite is a no-op
// when no engine is running (CI, offline).
export async function ceReachable(): Promise<boolean> {
  try {
    const ctrl = new AbortController();
    const t = setTimeout(() => ctrl.abort(), 2500);
    const res = await fetch(`${CE_BASE}/api/v0/schema`, { signal: ctrl.signal });
    clearTimeout(t);
    return res.ok;
  } catch {
    return false;
  }
}

// Delete any existing test folder(s) at root (cascades their children) and create
// a fresh empty one. Returns its uid. Scoped — other components are untouched.
export async function resetTestFolder(): Promise<number> {
  setEngineBase(CE_BASE);
  await deleteTestFolder();
  const folder = await addNode({ type: "core-extRoot::Folder", name: TEST_FOLDER, parentUid: 0 });
  return folder.uid;
}

export async function deleteTestFolder(): Promise<void> {
  setEngineBase(CE_BASE);
  const resp = await getRootNodes({ depth: 1, nested: true });
  for (const c of resp.nodes[0]?.children ?? []) {
    if (c.name === TEST_FOLDER) await removeNode(c.uid);
  }
}

// Create a math::add with its inputs seeded at creation via defaultValues.
export function makeAdd(folder: number, name: string, in1: number, in2 = 0) {
  return addNode({
    type: ADD,
    name,
    parentUid: folder,
    defaultValues: { properties: { in1: { value: in1 }, in2: { value: in2 } } },
  });
}

function wsBase(): string {
  return CE_BASE.replace(/^http/, "ws") + "/ws";
}

// Subscribe (by component and/or by property) and collect the latest value per
// watched prop uid (decoded with the real lib decoder), or — with `status: true`
// — the latest STATUS uint32 per watched prop. Resolves once every `expect`
// target matches, or after `timeoutMs`. The engine computes asynchronously, so a
// value may arrive a tick or two after the mutations land.
export function readValues(opts: {
  components?: number[];
  properties?: number[]; // property-level subscription (exposed-ports mechanism)
  watch: number[];
  expect?: Record<number, number>; // value (or STATUS uint32 when status:true) to wait for
  status?: boolean; // read the STATUS section instead of the value sections
  timeoutMs?: number;
}): Promise<Map<number, DecodedValue>> {
  const { components = [], properties = [], watch, expect = {}, status = false, timeoutMs = 10000 } = opts;
  const want = new Set(watch);
  return new Promise((resolve) => {
    const values = new Map<number, DecodedValue>();
    const ws = new WebSocket(wsBase());
    ws.binaryType = "arraybuffer";
    let done = false;
    const finish = () => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      try {
        ws.close();
      } catch {
        /* ignore */
      }
      resolve(values);
    };
    const timer = setTimeout(finish, timeoutMs);
    const satisfied = () => {
      const keys = Object.keys(expect);
      return keys.length > 0 && keys.every((uid) => values.get(Number(uid)) === expect[Number(uid)]);
    };
    ws.onopen = () => ws.send(JSON.stringify({ type: "configure", tickHz: 10 }));
    ws.onmessage = (ev) => {
      if (typeof ev.data === "string") {
        try {
          const m = JSON.parse(ev.data) as { type?: string };
          if (m.type === "schema") {
            if (components.length) ws.send(JSON.stringify({ type: "subscribe", components }));
            if (properties.length) ws.send(JSON.stringify({ type: "subscribe", properties }));
          }
        } catch {
          /* ignore non-JSON */
        }
        return;
      }
      const frame = decodeBinaryFrame(ev.data as ArrayBuffer);
      for (const sec of frame.sections) {
        const isStatus = sec.typeTag === TYPE_STATUS;
        if (status !== isStatus) continue; // value sections vs the STATUS section
        for (let i = 0; i < sec.uids.length; i++) {
          if (want.has(sec.uids[i])) {
            values.set(sec.uids[i], (sec.values as ArrayLike<DecodedValue>)[i]);
          }
        }
      }
      if (satisfied()) finish();
    };
    ws.onerror = () => finish();
  });
}

// Connect a socket, run `trigger` (a REST mutation) once subscribed, and resolve
// with the first topology JSON message whose `type` matches — or null on timeout.
// Used to assert the live-collaboration push path (REST mutate → WS topology event).
export function waitForTopology(
  type: "topologyAdded" | "topologyRemoved" | "topologyChanged",
  trigger: () => Promise<unknown>,
  timeoutMs = 8000,
): Promise<Record<string, unknown> | null> {
  return new Promise((resolve) => {
    const ws = new WebSocket(wsBase());
    ws.binaryType = "arraybuffer";
    let done = false;
    const finish = (v: Record<string, unknown> | null) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      try {
        ws.close();
      } catch {
        /* ignore */
      }
      resolve(v);
    };
    const timer = setTimeout(() => finish(null), timeoutMs);
    ws.onopen = () => ws.send(JSON.stringify({ type: "configure", tickHz: 10 }));
    ws.onmessage = (ev) => {
      if (typeof ev.data !== "string") return;
      let m: Record<string, unknown>;
      try {
        m = JSON.parse(ev.data) as Record<string, unknown>;
      } catch {
        return;
      }
      if (m.type === "schema") {
        // Subscribed/ready — fire the trigger that should produce the event.
        void trigger();
        return;
      }
      if (m.type === type) finish(m);
    };
    ws.onerror = () => finish(null);
  });
}
