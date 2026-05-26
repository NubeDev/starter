// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: 1a0736055a4647c18d0be19347e4325007c7bd52.
// Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.
//
// Types only. The data layer lives in `./hooks/use-warehouse-ch.ts`
// and hits the warehouse-ch sub-router at `/api/warehouse/ch/*`
// through the host's `StarterClient`.

export type Counts = { name: string; count: number }[];

export type Overview = {
  file_name: string;
  sqlite_version: string | null;
  // Backend field name is `size_on_disk`; upstream sql-studio called
  // it `db_size`. The hook reviver normalises the wire format to this
  // shape so the upstream view code stays untouched.
  db_size: string;
  created: Date | null;
  modified: Date | null;
  tables: number;
  indexes: number;
  triggers: number;
  views: number;
  row_counts: Counts;
  column_counts: Counts;
  index_counts: Counts;
};

export type Tables = { tables: Counts };

export type Table = {
  name: string;
  sql: string | null;
  row_count: number;
  index_count: number;
  column_count: number;
  table_size: string;
};

export type TableData = {
  columns: string[];
  rows: unknown[][];
};

export type Query = TableData;

export type Autocomplete = {
  tables: { columns: string[]; table_name: string }[];
};

export type ErdColumn = {
  name: string;
  data_type: string;
  nullable: boolean;
  is_primary_key: boolean;
};

export type ErdTable = { name: string; columns: ErdColumn[] };

export type ErdRelationship = {
  from_table: string;
  from_column: string;
  to_table: string;
  to_column: string;
};

export type Erd = {
  tables: ErdTable[];
  relationships: ErdRelationship[];
};
