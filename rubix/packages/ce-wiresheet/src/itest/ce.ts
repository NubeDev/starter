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

// Subscribe to `components` and collect the latest streamed value per watched
// prop uid (decoded with the real lib decoder). Resolves once every `expect`
// target matches, or after `timeoutMs`. The engine computes asynchronously, so
// the value may arrive a tick or two after the mutations land.
export function readValues(opts: {
  components: number[];
  watch: number[];
  expect?: Record<number, number>;
  timeoutMs?: number;
}): Promise<Map<number, DecodedValue>> {
  const { components, watch, expect = {}, timeoutMs = 10000 } = opts;
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
          if (m.type === "schema") ws.send(JSON.stringify({ type: "subscribe", components }));
        } catch {
          /* ignore non-JSON */
        }
        return;
      }
      const frame = decodeBinaryFrame(ev.data as ArrayBuffer);
      for (const sec of frame.sections) {
        if (sec.typeTag === TYPE_STATUS) continue; // status bits, not values
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
