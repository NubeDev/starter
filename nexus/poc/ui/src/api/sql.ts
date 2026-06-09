// SQL playground: run DataFusion SQL over inline JSON rows.

import { postJson } from "./client";

export interface SqlResponse {
  ok: boolean;
  error: string | null;
  row_count: number;
  rows: Record<string, unknown>[];
}

export function runSql(query: string, rows: unknown[]): Promise<SqlResponse> {
  return postJson<SqlResponse>("/api/sql/query", { query, rows });
}
