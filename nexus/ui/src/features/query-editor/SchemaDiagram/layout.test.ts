import { describe, expect, it } from "vitest";

import type { DatasourceSchema } from "@/api/types";
import { buildModel, gridPositions, tableKey } from "./layout";

const col = (name: string, data_type = "text") => ({ name, data_type });

const schema = (
  tables: DatasourceSchema["tables"],
  relations: DatasourceSchema["relations"] = [],
): DatasourceSchema => ({ tables, relations });

describe("buildModel", () => {
  it("maps tables to nodes keyed by schema.name", () => {
    const { nodes } = buildModel(
      schema([
        { schema: "public", name: "orders", columns: [col("id")] },
        { schema: "billing", name: "orders", columns: [col("id")] },
      ]),
    );
    expect(nodes.map((n) => n.key)).toEqual([
      "public.orders",
      "billing.orders",
    ]);
  });

  it("builds one edge per FK and marks the FK columns on both ends", () => {
    const { nodes, edges } = buildModel(
      schema(
        [
          {
            schema: "public",
            name: "orders",
            columns: [col("id"), col("customer_id")],
          },
          { schema: "public", name: "customers", columns: [col("id")] },
        ],
        [
          {
            from_schema: "public",
            from_table: "orders",
            from_column: "customer_id",
            to_schema: "public",
            to_table: "customers",
            to_column: "id",
          },
        ],
      ),
    );
    expect(edges).toHaveLength(1);
    expect(edges[0].from).toBe("public.orders");
    expect(edges[0].to).toBe("public.customers");
    expect(edges[0].label).toBe("customer_id → id");

    const orders = nodes.find((n) => n.key === "public.orders")!;
    const customers = nodes.find((n) => n.key === "public.customers")!;
    expect(orders.fkColumns.has("customer_id")).toBe(true);
    expect(customers.fkColumns.has("id")).toBe(true);
  });

  it("drops a FK whose referenced table isn't a node (dangling edge)", () => {
    const { edges } = buildModel(
      schema(
        [{ schema: "public", name: "orders", columns: [col("customer_id")] }],
        [
          {
            from_schema: "public",
            from_table: "orders",
            from_column: "customer_id",
            to_schema: "pg_catalog",
            to_table: "pg_class",
            to_column: "oid",
          },
        ],
      ),
    );
    expect(edges).toHaveLength(0);
  });

  it("tolerates a schema with no relations field", () => {
    const { edges } = buildModel({
      tables: [{ schema: "public", name: "t", columns: [col("id")] }],
    } as DatasourceSchema);
    expect(edges).toHaveLength(0);
  });
});

describe("gridPositions", () => {
  it("places every node at a unique position", () => {
    const { nodes, edges } = buildModel(
      schema([
        { schema: "public", name: "a", columns: [] },
        { schema: "public", name: "b", columns: [] },
        { schema: "public", name: "c", columns: [] },
        { schema: "public", name: "d", columns: [] },
      ]),
    );
    const pos = gridPositions(nodes, edges);
    expect(pos.size).toBe(4);
    const seen = new Set([...pos.values()].map((p) => `${p.x},${p.y}`));
    expect(seen.size).toBe(4);
  });
});

describe("tableKey", () => {
  it("qualifies with the schema", () => {
    expect(tableKey("public", "users")).toBe("public.users");
  });
});
