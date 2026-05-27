export const EXTENSION_ID = "com.rubix.example";

/** Full customer row, as returned by `com.rubix.example.customers_sample`. */
export interface CustomerRow {
  readonly customer_id: string;
  readonly first_name: string;
  readonly last_name?: string;
  readonly company?: string;
  readonly city?: string;
  readonly country: string;
  readonly email: string;
  readonly subscription_date: string;
  readonly website?: string;
}

/** Aggregated row from `com.rubix.example.customers_by_country`. */
export interface CountryBucket {
  readonly country: string;
  readonly customer_count: number;
}

/** Row from `com.rubix.example.products_low_stock`. */
export interface ProductRow {
  readonly internal_id: string;
  readonly name: string;
  readonly brand: string;
  readonly category: string;
  readonly price: number;
  readonly stock: number;
  readonly availability: string;
}

export interface RuleOutcome {
  outcome: "ok" | "flag" | "drop";
  quality?: string;
  note?: string;
}

export interface ExtensionDetail {
  id: string;
  enabled: string;
  state: string;
  manifest: {
    id?: string;
    version?: string;
    contributes?: {
      tools?: ReadonlyArray<{ id: string }>;
      warehouse_tables?: ReadonlyArray<{ name: string }>;
      warehouse_templates?: ReadonlyArray<{ name: string }>;
      anomaly_rules?: ReadonlyArray<{ id: string }>;
      ui?: { entry: string; exposes?: ReadonlyArray<{ slot: string }> };
    };
  } | null;
}

/** Wire shape of the `com.rubix.example.warehouse_query` tool response. */
export interface WarehouseQueryResponse<R = Record<string, unknown>> {
  template: string;
  rows: ReadonlyArray<R>;
  count: number;
}
