import { beforeAll, describe, expect, it } from "vitest";
import { StarterClient } from "@nube/starter-client-ts";

import { getMe } from "@/api/me/get";
import { listDatasources } from "@/api/datasources/list";
import { createDatasource } from "@/api/datasources/create";
import { removeDatasource } from "@/api/datasources/remove";
import { queryDatasource } from "@/api/datasources/query";
import { listAlertRules } from "@/api/alerts/rules";
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
  });

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
});
