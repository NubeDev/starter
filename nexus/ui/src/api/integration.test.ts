import { beforeAll, describe, expect, it } from "vitest";
import { StarterClient } from "@nube/starter-client-ts";

import { getMe } from "@/api/me/get";
import { listDatasources } from "@/api/datasources/list";
import { createDatasource } from "@/api/datasources/create";
import { removeDatasource } from "@/api/datasources/remove";
import { queryDatasource } from "@/api/datasources/query";
import { testDatasource } from "@/api/datasources/test";
import { listAlertRules } from "@/api/alerts/rules";
import { createDashboard } from "@/api/dashboards/create";
import { getDashboard } from "@/api/dashboards/get";
import { removeDashboard } from "@/api/dashboards/remove";
import { addPanel } from "@/api/dashboards/addPanel";
import { updatePanel } from "@/api/dashboards/updatePanel";
import { removePanel } from "@/api/dashboards/removePanel";
import { createFlow } from "@/api/flows/create";
import { removeFlow } from "@/api/flows/remove";
import { startFlow, stopFlow } from "@/api/flows/lifecycle";
import { login } from "@/auth/login";

// Integration suite — runs the bindings against a REAL nexus-api, never a
// faked network (F10/README §6). Opt-in: set NEXUS_E2E_URL to a running
// instance (e.g. http://127.0.0.1:8080); otherwise the whole suite skips,
// so CI without a backend stays green. The seeded admin is the default
// dev login; override with NEXUS_E2E_EMAIL / NEXUS_E2E_PASSWORD.
//
//   NEXUS_E2E_URL=http://127.0.0.1:8080 pnpm test src/api/integration
const BASE = process.env.NEXUS_E2E_URL;
const EMAIL = process.env.NEXUS_E2E_EMAIL ?? "admin@nexus.local";
const PASSWORD = process.env.NEXUS_E2E_PASSWORD ?? "change-me-admin";

// A datasource pointing at nexus-api's own metadata Postgres — present in
// every dev deploy, so the query path has something real to hit. Override
// the connection via NEXUS_E2E_PG_* if the dev DB differs.
const PG = {
  host: process.env.NEXUS_E2E_PG_HOST ?? "127.0.0.1",
  port: Number(process.env.NEXUS_E2E_PG_PORT ?? 5432),
  database: process.env.NEXUS_E2E_PG_DB ?? "nexus",
  user: process.env.NEXUS_E2E_PG_USER ?? "nexus",
  password: process.env.NEXUS_E2E_PG_PASSWORD ?? "nexus",
};

// A browser keeps a cookie jar automatically; node's `fetch` does not, so
// the session cookie set by login wouldn't ride subsequent requests. Wrap
// `fetch` with a minimal jar that captures `set-cookie` and replays it —
// the same `credentials: "include"` behaviour the app gets in a browser.
function jarFetch(): typeof fetch {
  let cookie = "";
  return (async (input: RequestInfo | URL, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    if (cookie) headers.set("cookie", cookie);
    const res = await globalThis.fetch(input, { ...init, headers });
    const set = res.headers.get("set-cookie");
    if (set) {
      // Keep just the name=value pairs, drop attributes (path, httponly…).
      const pairs = set
        .split(/,(?=[^ ;]+=)/)
        .map((c) => c.split(";")[0].trim());
      cookie = pairs.join("; ");
      // Mirror into jsdom's document.cookie so `readCsrfHeader()` (which
      // reads the `starter_csrf` cookie) can echo the token on mutations,
      // exactly as it does in a real browser.
      for (const p of pairs) document.cookie = p;
    }
    return res;
  }) as typeof fetch;
}

describe.skipIf(!BASE)("integration: nexus-api", () => {
  let client: StarterClient;

  beforeAll(async () => {
    client = new StarterClient({ baseUrl: BASE!, fetch: jarFetch() });
    await login(client, { email: EMAIL, password: PASSWORD });
    // Generous: the first request after a backend rebuild can be slow.
  }, 30_000);

  it("returns the authenticated principal from /me", async () => {
    const me = await getMe(client);
    expect(me.subject).toBeTruthy();
    expect(["reader", "writer", "admin"]).toContain(me.role);
  });

  it("lists datasources without error", async () => {
    const list = await listDatasources(client);
    expect(Array.isArray(list)).toBe(true);
  });

  it("registers a datasource and queries it for real rows, then cleans up", async () => {
    const ds = await createDatasource(client, {
      name: `e2e-${Date.now()}`,
      kind: "postgres",
      ...PG,
    });
    expect(ds.id).toBeTruthy();
    try {
      const res = await queryDatasource(client, ds.id, {
        sql: "select 42 as answer, 'ok' as status",
      });
      expect(res.rows[0]).toMatchObject({ answer: 42, status: "ok" });
      expect(res.columns.map((c) => c.name)).toEqual(["answer", "status"]);
      expect(res.stats.truncated).toBe(false);
    } finally {
      await removeDatasource(client, ds.id);
    }
  });

  it("lists alert rules without error", async () => {
    const rules = await listAlertRules(client);
    expect(Array.isArray(rules)).toBe(true);
  });

  it("probes a datasource connection (test endpoint)", async () => {
    const ds = await createDatasource(client, {
      name: `e2e-test-${Date.now()}`,
      kind: "postgres",
      ...PG,
    });
    try {
      const probe = await testDatasource(client, ds.id);
      // The seeded metadata DB is reachable, so the probe should connect.
      expect(probe.ok).toBe(true);
      expect(typeof probe.latency_ms === "number" || probe.latency_ms === null).toBe(true);
    } finally {
      await removeDatasource(client, ds.id);
    }
  });

  it("round-trips a dashboard with a panel: create → add → PATCH layout → delete", async () => {
    const slug = `e2e-${Date.now()}`;
    const ds = await createDatasource(client, {
      name: `e2e-panel-ds-${Date.now()}`,
      kind: "postgres",
      ...PG,
    });
    await createDashboard(client, { name: "E2E dashboard", slug });
    try {
      const panel = await addPanel(client, slug, {
        title: "E2E panel",
        sql: "select 1 as v",
        datasource_id: ds.id,
        viz: "stat",
        layout: { x: 0, y: 0, w: 3, h: 2 },
      });
      expect(panel.id).toBeTruthy();

      // PATCH only the layout; title/sql/viz stay put (partial update).
      const moved = await updatePanel(client, panel.id, {
        layout: { x: 6, y: 4, w: 3, h: 2 },
      });
      expect(moved.title).toBe("E2E panel");
      expect((moved.layout as { x: number }).x).toBe(6);

      // The dashboard detail reflects the panel.
      const detail = await getDashboard(client, slug);
      expect(detail.panels.map((p) => p.id)).toContain(panel.id);

      await removePanel(client, panel.id);
      const after = await getDashboard(client, slug);
      expect(after.panels.map((p) => p.id)).not.toContain(panel.id);
    } finally {
      await removeDashboard(client, slug);
      await removeDatasource(client, ds.id);
    }
  });

  it("round-trips a flow lifecycle: create → start → stop → delete", async () => {
    const flow = await createFlow(client, {
      name: `e2e-flow-${Date.now()}`,
      enabled: true,
      input: { type: "generate", interval: "1s" },
      pipeline: [],
      output: { type: "stdout" },
    });
    try {
      expect(flow.id).toBeTruthy();
      const started = await startFlow(client, flow.id);
      expect(started.running).toBe(true);
      const stopped = await stopFlow(client, flow.id);
      expect(stopped.running).toBe(false);
    } finally {
      await removeFlow(client, flow.id);
    }
  });
});
