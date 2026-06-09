import type { SQLNamespace } from "@codemirror/lang-sql";

import type { DatasourceSchema } from "@/api/types";

// Translate an introspected datasource schema into the shape
// `@codemirror/lang-sql` wants for schema-aware completion. The `schema`
// option maps a table name to its column names; CodeMirror then completes
// table names after FROM/JOIN and column names after a qualifying table.
//
// A table is keyed both bare (`events`) and schema-qualified (`public.events`)
// so completion works whether or not the user qualifies it. `public` — the
// default search path — is also exposed unqualified for the common case.
export function toSqlNamespace(schema: DatasourceSchema | undefined): {
  schema: SQLNamespace;
  tables: string[];
} {
  const ns: Record<string, string[]> = {};
  const tables: string[] = [];
  if (!schema) return { schema: ns, tables };

  for (const t of schema.tables) {
    const columns = t.columns.map((c) => c.name);
    const qualified = `${t.schema}.${t.name}`;
    ns[qualified] = columns;
    tables.push(qualified);
    // Unqualified key: last writer wins on a name collision across schemas,
    // which matches Postgres resolving an unqualified name via search_path.
    ns[t.name] = columns;
    if (t.schema === "public") tables.push(t.name);
  }
  return { schema: ns, tables };
}
