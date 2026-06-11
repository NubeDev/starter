import { Database } from "lucide-react";

// Generic discovery chips for Explore: one click inserts a ready-made query and
// runs it, so a user can poke at an unfamiliar database before knowing any
// table name. Per-table browsing lives in the SchemaSidebar (a grouped,
// searchable tree); these two chips are the schema-agnostic shortcuts that work
// on any Postgres even with an empty/unintrospected schema.

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
  // Nothing to scope a query to yet — prompt the user to pick a datasource
  // rather than show dead buttons.
  if (!datasourceId) {
    return (
      <p className="text-xs text-muted-foreground">
        Pick a datasource to discover its tables.
      </p>
    );
  }

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <Chip onClick={() => onRun(LIST_TABLES)} icon={<Database className="size-3.5" />}>
        List tables
      </Chip>
      <Chip onClick={() => onRun(LIST_COLUMNS)} icon={<Database className="size-3.5" />}>
        List columns
      </Chip>
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
