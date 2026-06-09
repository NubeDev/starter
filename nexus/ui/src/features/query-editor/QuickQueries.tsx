import { Database, Table2 } from "lucide-react";

import type { SchemaTable } from "@/api/types";
import { useDatasourceSchema } from "@/features/sql-editor";

// Discovery shortcuts for Explore: one click inserts a ready-made query and
// runs it, so a user can poke at an unfamiliar database without knowing its
// schema or typing SQL. Two tiers:
//   • generic chips that work on any Postgres (list tables / columns), useful
//     even before any table name is known;
//   • a chip per introspected table that peeks its first rows.
// Reads the same cached schema the editor's autocomplete uses, so showing
// these costs no extra request.

// A schema-qualified identifier, quoted only if it isn't a plain lowercase
// name — keeps the common case readable while staying correct for mixed-case
// or reserved names.
function ident(name: string): string {
  return /^[a-z_][a-z0-9_]*$/.test(name) ? name : `"${name}"`;
}

function peekQuery(t: SchemaTable): string {
  const ref = t.schema === "public" ? ident(t.name) : `${ident(t.schema)}.${ident(t.name)}`;
  return `SELECT * FROM ${ref} LIMIT 100;`;
}

const LIST_TABLES = `SELECT table_schema, table_name
FROM information_schema.tables
WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
ORDER BY table_schema, table_name;`;

const LIST_COLUMNS = `SELECT table_schema, table_name, column_name, data_type
FROM information_schema.columns
WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
ORDER BY table_schema, table_name, ordinal_position;`;

export function QuickQueries({
  datasourceId,
  onRun,
}: {
  datasourceId: string | undefined;
  /** Replace the editor's SQL with `sql` and run it immediately. */
  onRun: (sql: string) => void;
}) {
  const { data: schema } = useDatasourceSchema(datasourceId);

  // Nothing to scope a query to yet — prompt the user to pick a datasource
  // rather than show dead buttons.
  if (!datasourceId) {
    return (
      <p className="text-xs text-muted-foreground">
        Pick a datasource to discover its tables.
      </p>
    );
  }

  const tables = schema?.tables ?? [];

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <Chip onClick={() => onRun(LIST_TABLES)} icon={<Database className="size-3.5" />}>
        List tables
      </Chip>
      <Chip onClick={() => onRun(LIST_COLUMNS)} icon={<Database className="size-3.5" />}>
        List columns
      </Chip>

      {tables.length > 0 ? (
        <span className="mx-1 h-4 w-px bg-border/60" aria-hidden />
      ) : null}

      {tables.map((t) => {
        const label = t.schema === "public" ? t.name : `${t.schema}.${t.name}`;
        return (
          <Chip
            key={`${t.schema}.${t.name}`}
            onClick={() => onRun(peekQuery(t))}
            icon={<Table2 className="size-3.5" />}
            title={`Peek 100 rows from ${label}`}
          >
            {label}
          </Chip>
        );
      })}
    </div>
  );
}

function Chip({
  children,
  onClick,
  icon,
  title,
}: {
  children: React.ReactNode;
  onClick: () => void;
  icon: React.ReactNode;
  title?: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      className="inline-flex items-center gap-1.5 rounded-full border border-border/60 bg-background/40 px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:border-primary/50 hover:bg-primary/10 hover:text-foreground"
    >
      <span className="text-primary">{icon}</span>
      {children}
    </button>
  );
}
