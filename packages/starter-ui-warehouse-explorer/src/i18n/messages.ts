// Localizable strings the explorer library emits at runtime. The
// package is `react-intl`-free; hosts derive an `ExplorerMessages`
// object from their own translation hook and pass it via
// `<ExplorerI18nProvider>` (or as `i18n?: Partial<ExplorerMessages>`
// to `<Explorer />`). Same pattern as `@nube/starter-ui-authz`.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

export interface ExplorerMessages {
  /** Shell-level labels. */
  shell: {
    title: string;
    tabs: {
      overview: string;
      tables: string;
      schema: string;
      query: string;
    };
  };
  /** Generic action / column labels reused across views. */
  common: {
    loading: string;
    empty: string;
    error: string;
    refresh: string;
    cancel: string;
    confirm: string;
  };
  /** Overview view (`/overview`). */
  overview: {
    eyebrow: string;
    counters: {
      tables: string;
      tablesDescription: string;
      indexes: string;
      indexesDescription: string;
      views: string;
      viewsDescription: string;
      triggers: string;
      triggersDescription: string;
    };
    sections: {
      rowsPerTable: string;
      moreMetadata: string;
      moreMetadataDescription: string;
      indexesPerTable: string;
      columnsPerTable: string;
    };
    metadata: {
      databaseSize: string;
      databaseSizeDescription: string;
      createdOn: string;
      createdOnDescription: string;
      modifiedOn: string;
      modifiedOnDescription: string;
    };
    columnsHeaderColumn: string;
    columnsHeaderCount: string;
    indexesHeaderIndex: string;
    indexesHeaderCount: string;
    tooltipRowHasCount: string;
  };
  /** Tables view. */
  tables: {
    emptyTitle: string;
    emptyDescription: string;
    notFoundTitle: string;
    notFoundDescription: string;
    virtualBadge: string;
    counters: {
      rows: string;
      rowsDescription: string;
      indexes: string;
      indexesDescription: string;
      columns: string;
      columnsDescription: string;
      size: string;
      sizeDescription: string;
    };
  };
  /** Schema (ERD) view. */
  schema: {
    emptyTitle: string;
    emptyDescription: string;
  };
  /** Query view. */
  query: {
    autoExecuteEnable: string;
    autoExecuteDisable: string;
    execute: string;
    errorTitle: string;
    errorDescription: string;
    noResultsTitle: string;
    noResultsDescription: string;
  };
}

/** Default English messages. */
export const DEFAULT_EXPLORER_MESSAGES: ExplorerMessages = {
  shell: {
    title: "Warehouse explorer",
    tabs: {
      overview: "Overview",
      tables: "Tables",
      schema: "Schema",
      query: "Query",
    },
  },
  common: {
    loading: "Loading…",
    empty: "Nothing here yet.",
    error: "Something went wrong.",
    refresh: "Refresh",
    cancel: "Cancel",
    confirm: "Confirm",
  },
  overview: {
    eyebrow: "EXPLORING",
    counters: {
      tables: "TABLES",
      tablesDescription: "The number of tables in the DB.",
      indexes: "INDEXES",
      indexesDescription: "The number of indexes in the DB.",
      views: "VIEWS",
      viewsDescription: "The number of views in the DB.",
      triggers: "TRIGGERS",
      triggersDescription: "The number of triggers in the DB.",
    },
    sections: {
      rowsPerTable: "ROWS PER TABLE",
      moreMetadata: "MORE METADATA",
      moreMetadataDescription: "More info about the DB.",
      indexesPerTable: "INDEXES PER TABLE",
      columnsPerTable: "COLUMNS PER TABLE",
    },
    metadata: {
      databaseSize: "DATABASE SIZE",
      databaseSizeDescription: "The size of the DB on disk.",
      createdOn: "CREATED ON",
      createdOnDescription: "The date and time when the DB was created.",
      modifiedOn: "MODIFIED ON",
      modifiedOnDescription: "The date and time when the DB was last modified.",
    },
    columnsHeaderColumn: "Column",
    columnsHeaderCount: "Count",
    indexesHeaderIndex: "Index",
    indexesHeaderCount: "Count",
    tooltipRowHasCount: "Table {table} has {value}.",
  },
  tables: {
    emptyTitle: "No tables found",
    emptyDescription: "The database has no tables.",
    notFoundTitle: "Table not found",
    notFoundDescription:
      "Could not find {table}. Showing the first available table instead.",
    virtualBadge: "Virtual Table",
    counters: {
      rows: "ROW COUNT",
      rowsDescription: "The number of rows in the table.",
      indexes: "INDEXES",
      indexesDescription: "The number of indexes in the table.",
      columns: "COLUMNS",
      columnsDescription: "The number of columns in the table.",
      size: "TABLE SIZE",
      sizeDescription: "The size of the table on disk.",
    },
  },
  schema: {
    emptyTitle: "No tables found",
    emptyDescription:
      "The database has no tables to display in the schema diagram.",
  },
  query: {
    autoExecuteEnable: "Enable auto execute",
    autoExecuteDisable: "Disable auto execute",
    execute: "Execute",
    errorTitle: "Error",
    errorDescription: "Query didn't execute successfully.",
    noResultsTitle: "Query executed",
    noResultsDescription: "Returned no data.",
  },
};

/** Deep-merge a partial override on top of `DEFAULT_EXPLORER_MESSAGES`. */
export function mergeExplorerMessages(
  overrides?: Partial<ExplorerMessages>,
): ExplorerMessages {
  if (!overrides) return DEFAULT_EXPLORER_MESSAGES;
  const base = DEFAULT_EXPLORER_MESSAGES;
  return {
    shell: {
      ...base.shell,
      ...overrides.shell,
      tabs: { ...base.shell.tabs, ...overrides.shell?.tabs },
    },
    common: { ...base.common, ...overrides.common },
    overview: {
      ...base.overview,
      ...overrides.overview,
      counters: { ...base.overview.counters, ...overrides.overview?.counters },
      sections: { ...base.overview.sections, ...overrides.overview?.sections },
      metadata: { ...base.overview.metadata, ...overrides.overview?.metadata },
    },
    tables: {
      ...base.tables,
      ...overrides.tables,
      counters: { ...base.tables.counters, ...overrides.tables?.counters },
    },
    schema: { ...base.schema, ...overrides.schema },
    query: { ...base.query, ...overrides.query },
  };
}
