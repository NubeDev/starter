import { beforeEach, describe, expect, it, vi } from "vitest";

import { parseCustomOptions, resolveOptions } from "@/features/variables/resolve";

// Mock the two API modules the resolver calls so the option-projection and
// cascading logic can be tested without a server. `queryDatasource` captures
// its request so we can assert parents are passed as `variables` (not inlined).
const queryDatasource = vi.fn();
const listDatasources = vi.fn();
vi.mock("@/api/datasources/query", () => ({
  queryDatasource: (...args: unknown[]) => queryDatasource(...args),
}));
vi.mock("@/api/datasources/list", () => ({
  listDatasources: (...args: unknown[]) => listDatasources(...args),
}));

const client = {} as never;

beforeEach(() => {
  queryDatasource.mockReset();
  listDatasources.mockReset();
});

describe("parseCustomOptions", () => {
  it("splits comma list and supports text:value", () => {
    expect(parseCustomOptions("prod, staging, Dev : dev")).toEqual([
      { text: "prod", value: "prod" },
      { text: "staging", value: "staging" },
      { text: "Dev", value: "dev" },
    ]);
  });

  it("drops blank entries", () => {
    expect(parseCustomOptions("a,, ,b")).toEqual([
      { text: "a", value: "a" },
      { text: "b", value: "b" },
    ]);
  });
});

describe("resolveOptions: static kinds", () => {
  it("constant yields one option", async () => {
    expect(await resolveOptions(client, "constant", { value: "v" }, {})).toEqual([
      { text: "v", value: "v" },
    ]);
  });

  it("interval maps steps", async () => {
    expect(
      await resolveOptions(client, "interval", { steps: ["1m", "5m"] }, {}),
    ).toEqual([
      { text: "1m", value: "1m" },
      { text: "5m", value: "5m" },
    ]);
  });
});

describe("resolveOptions: context kind", () => {
  const ctx = {
    nav: { nodeId: "n1", slug: "energy", name: "Building-1", path: ["Buildings"] },
    url: { building: "b-url" },
    tags: { building: "b-tag" },
    values: { building: "b1" },
  };

  it("reads a values-source key and binds it as the single option", async () => {
    expect(
      await resolveOptions(
        client,
        "context",
        { source: "values", key: "building" },
        {},
        ctx,
      ),
    ).toEqual([{ text: "b1", value: "b1" }]);
  });

  it("reads a url-source bare param", async () => {
    expect(
      await resolveOptions(
        client,
        "context",
        { source: "url", key: "building" },
        {},
        ctx,
      ),
    ).toEqual([{ text: "b-url", value: "b-url" }]);
  });

  it("yields no option when the source/key is absent", async () => {
    expect(
      await resolveOptions(
        client,
        "context",
        { source: "tag", key: "missing" },
        {},
        ctx,
      ),
    ).toEqual([]);
  });

  it("never fetches", async () => {
    await resolveOptions(client, "context", { source: "nav", key: "slug" }, {}, ctx);
    expect(queryDatasource).not.toHaveBeenCalled();
    expect(listDatasources).not.toHaveBeenCalled();
  });
});

describe("resolveOptions: datasource kind", () => {
  it("filters by kind and binds the id as value", async () => {
    listDatasources.mockResolvedValue([
      { id: "1", name: "PG-A", kind: "postgres" },
      { id: "2", name: "CH-B", kind: "clickhouse" },
    ]);
    const opts = await resolveOptions(client, "datasource", { kindFilter: "postgres" }, {});
    expect(opts).toEqual([{ text: "PG-A", value: "1" }]);
  });
});

describe("resolveOptions: query kind (cascading)", () => {
  it("passes referenced parents as bound variables, projects rows, dedupes", async () => {
    queryDatasource.mockResolvedValue({
      columns: [{ name: "region" }],
      rows: [{ region: "us" }, { region: "eu" }, { region: "us" }],
      stats: {},
    });
    const opts = await resolveOptions(
      client,
      "query",
      { sql: "select region where dc=$dc", datasourceId: "d" },
      { dc: ["dc1"] },
    );
    expect(opts).toEqual([
      { text: "us", value: "us" },
      { text: "eu", value: "eu" },
    ]);
    // The parent selection is passed as a bound `QueryVariable`, never inlined.
    const [, , request] = queryDatasource.mock.calls[0];
    expect(request.variables).toEqual([{ name: "dc", values: ["dc1"] }]);
  });

  it("projects separate text/value columns", async () => {
    queryDatasource.mockResolvedValue({
      columns: [{ name: "label" }, { name: "id" }],
      rows: [{ label: "Site A", id: "a" }],
      stats: {},
    });
    const opts = await resolveOptions(
      client,
      "query",
      { sql: "select label, id", datasourceId: "d", textColumn: "label", valueColumn: "id" },
      {},
    );
    expect(opts).toEqual([{ text: "Site A", value: "a" }]);
  });

  it("returns nothing for empty SQL", async () => {
    expect(await resolveOptions(client, "query", { sql: "", datasourceId: "d" }, {})).toEqual(
      [],
    );
    expect(queryDatasource).not.toHaveBeenCalled();
  });
});
