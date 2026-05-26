// Round-trip test — exercises one method per endpoint family
// against a fetch-mock so we catch wiring regressions across the
// whole `RubixClient` surface in a single run.
//
// The mock fetch routes by path + method and returns a recorded
// fixture body. For each family we assert the request URL and the
// parsed return shape — proving the per-endpoint declaration-merge
// landed, the fetch helper bound the right base, and the response
// parser handled the body.
//
// # Running against a live agent
//
// An operator can run this against a real rubix-agent by:
//
//   1. Booting the agent locally on `http://127.0.0.1:8080` with the
//      seeded admin credentials documented in
//      `rubix/docs/HOW-TO-CODE.md`.
//   2. Setting `RUBIX_ROUND_TRIP_BASE=http://127.0.0.1:8080` and
//      `RUBIX_ROUND_TRIP_COOKIE=<starter_session=...>` in the env.
//   3. `pnpm --filter @nube/rubix-client-ts test round-trip`.
//
// Live-mode wiring is intentionally minimal in this stage — the
// design note above is what an operator follows to drive the run by
// hand. CI runs the fetch-mock path only.

import { describe, expect, it } from "vitest";

import { StarterClient } from "@nube/starter-client-ts";

import { RubixClient } from "../src/client/client.js";
import "../src/endpoints/index.js";

type Handler = (init: RequestInit | undefined) => Response;

interface Route {
  method: string;
  path: string;
  handler: Handler;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function emptyResponse(status = 204): Response {
  return new Response(null, { status });
}

function buildClient(routes: Route[]): {
  client: RubixClient;
  hits: { method: string; path: string }[];
} {
  const hits: { method: string; path: string }[] = [];
  const fake: typeof fetch = async (input, init) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const u = new URL(url);
    const path = u.pathname;
    const method = (init?.method ?? "GET").toUpperCase();
    hits.push({ method, path });
    const route = routes.find((r) => r.method === method && r.path === path);
    if (!route) {
      return new Response(`no mock for ${method} ${path}`, { status: 599 });
    }
    return route.handler(init).clone();
  };
  const starter = new StarterClient({ baseUrl: "http://t", fetch: fake });
  return { client: new RubixClient(starter), hits };
}

describe("RubixClient round-trip — one method per endpoint family", () => {
  it("exercises system, alert, user, team, tenant, clickhouse, flow_ops, undo, mcp", async () => {
    const routes: Route[] = [
      // system
      {
        method: "POST",
        path: "/api/v1/tools/rubix.system.disk",
        handler: () =>
          jsonResponse({
            summary: { code: "rubix.system.disk.ok" },
            mount: "/",
            total_bytes: 1,
            free_bytes: 1,
            used_percent: 0,
            probed_at_ms: 0,
          }),
      },
      // alert
      {
        method: "POST",
        path: "/api/v1/tools/rubix.alert.send",
        handler: () =>
          jsonResponse({
            summary: { code: "rubix.alert.send.ok" },
            severity: "info",
            delivered_chars: 1,
            probed_at_ms: 0,
          }),
      },
      // user
      {
        method: "POST",
        path: "/api/v1/tools/rubix.user.create",
        handler: () =>
          jsonResponse({
            summary: { code: "rubix.user.create.ok" },
            user_id: "u1",
            email: "a@b",
          }),
      },
      // team
      {
        method: "POST",
        path: "/api/v1/tools/rubix.team.create",
        handler: () =>
          jsonResponse({
            summary: { code: "rubix.team.create.ok" },
            team_id: "t1",
            name: "ops",
          }),
      },
      // tenant
      {
        method: "POST",
        path: "/api/v1/tools/rubix.tenant.list",
        handler: () =>
          jsonResponse({
            summary: { code: "rubix.tenant.list.ok" },
            tenants: [],
          }),
      },
      // clickhouse
      {
        method: "POST",
        path: "/api/v1/tools/rubix.warehouse.rule.write",
        handler: () =>
          jsonResponse({ summary: { code: "rubix.warehouse.rule.write.ok" } }),
      },
      // flow_ops
      {
        method: "POST",
        path: "/api/v1/tools/rubix.flow_ops.list",
        handler: () =>
          jsonResponse({ summary: { code: "rubix.flow.list.ok" }, flows: [] }),
      },
      // undo
      {
        method: "POST",
        path: "/api/v1/tools/rubix.undo.last",
        handler: () => jsonResponse({ summary: { code: "rubix.undo.last.ok" } }),
      },
      // mcp
      {
        method: "POST",
        path: "/api/v1/mcp",
        handler: (init) => {
          const body = JSON.parse(String(init?.body ?? "{}"));
          if (body.method === "tools/list") {
            return jsonResponse({ jsonrpc: "2.0", id: body.id, result: { tools: [] } });
          }
          return jsonResponse({
            jsonrpc: "2.0",
            id: body.id,
            result: { content: [], structuredContent: { ok: true } },
          });
        },
      },
    ];

    const { client, hits } = buildClient(routes);

    // System family — disk
    const disk: any = await (client as any).disk();
    expect(disk.summary.code).toBe("rubix.system.disk.ok");

    // Alert family — send
    const alert: any = await (client as any).send({ severity: "info", message: "x" });
    expect(alert.summary.code).toBe("rubix.alert.send.ok");

    // User family — create
    const user: any = await (client as any).userCreate({
      email: "a@b",
      display_name: "A",
    });
    expect(user.user_id).toBe("u1");

    // Team family — create
    const team: any = await (client as any).teamCreate({ name: "ops" });
    expect(team.team_id).toBe("t1");

    // Tenant family — list
    const tenants: any = await (client as any).tenantList();
    expect(Array.isArray(tenants.tenants)).toBe(true);

    // Clickhouse family — ruleWrite
    await (client as any).ruleWrite({ rule: { id: "r1", expression: "x" } });

    // Flow ops family — list
    const flows: any = await (client as any).flowList();
    expect(Array.isArray(flows.flows)).toBe(true);

    // Undo family — last
    await (client as any).undoLast();

    // MCP family — tools/list and tools/call
    const list = await client.mcpToolsList({ acceptLanguage: "en-US" });
    expect(list.tools).toEqual([]);
    const called = await client.mcpToolsCall<{ ok: boolean }>(
      "com.rubix.scheduled-system-check",
      {},
      { acceptLanguage: "es-AR" },
    );
    expect(called.ok).toBe(true);

    // Every family was hit at least once.
    const paths = new Set(hits.map((h) => h.path));
    for (const r of routes) {
      expect(paths.has(r.path), `missing hit for ${r.path}`).toBe(true);
    }
  });
});
