// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
//
// Tables view — list of tables on the left, per-table summary +
// paginated data grid on the right. Rewritten from the upstream
// `routes/tables.tsx`:
//   * No `createFileRoute` / loader / search params — selected
//     table name is local component state.
//   * Visible strings come from `useExplorerMessages()`.
//   * Data flows through `useChTable*` hooks.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import "react-data-grid/lib/styles.css";

import { useState, type UIEvent } from "react";
import {
  HardDrive,
  DatabaseZap,
  TableProperties,
  Table as TableIcon,
} from "lucide-react";
import { DataGrid } from "react-data-grid";
import { CodeBlock, irBlack as CodeDarkTheme } from "react-code-blocks";

import { cn } from "../lib/utils.js";
import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card.js";
import { Badge } from "../components/ui/badge.js";
import { Skeleton } from "../components/ui/skeleton.js";
import { InfoCard, InfoCardProps } from "../components/info-card.js";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../components/ui/tabs.js";
import {
  useChTable,
  useChTableData,
  useChTables,
  useResolvedTheme,
} from "../hooks/index.js";
import { useExplorerMessages } from "../i18n/index.js";

export interface ExplorerTablesProps {
  /** Optional initial table name (selected on mount). */
  initialTable?: string;
}

export function ExplorerTables({ initialTable }: ExplorerTablesProps) {
  const m = useExplorerMessages();
  const { data, isPending } = useChTables();
  const [selected, setSelected] = useState<string | undefined>(initialTable);

  if (isPending || !data) return <Skeleton className="w-[70vw] h-[30px]" />;

  if (data.tables.length === 0) {
    return (
      <Card>
        <CardHeader className="flex items-center">
          <TableIcon className="mb-4 h-12 w-12 text-muted-foreground" />
          <CardTitle>{m.tables.emptyTitle}</CardTitle>
          <CardDescription>{m.tables.emptyDescription}</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  const validRequested =
    selected && data.tables.some(({ name }) => name === selected);
  const selectedTable = validRequested ? selected! : data.tables[0].name;
  const requestedTableMissing = !!selected && !validRequested;

  return (
    <>
      {requestedTableMissing && (
        <Card className="mb-3">
          <CardHeader>
            <CardTitle>{m.tables.notFoundTitle}</CardTitle>
            <CardDescription>
              {m.tables.notFoundDescription.replace(
                "{table}",
                JSON.stringify(selected),
              )}
            </CardDescription>
          </CardHeader>
        </Card>
      )}

      <Tabs value={selectedTable} onValueChange={(v) => setSelected(v)}>
        <TabsList>
          {data.tables.map((n) => (
            <TabsTrigger key={n.name} value={n.name}>
              {n.name} [{n.count.toLocaleString()}]
            </TabsTrigger>
          ))}
        </TabsList>
        {data.tables.map(({ name }) => (
          <TabsContent key={name} value={name} className="py-4">
            <ExplorerTableDetail name={name} />
          </TabsContent>
        ))}
      </Tabs>
    </>
  );
}

export interface ExplorerTableDetailProps {
  name: string;
}

export function ExplorerTableDetail({ name }: ExplorerTableDetailProps) {
  const m = useExplorerMessages();
  const currentTheme = useResolvedTheme();
  const { data } = useChTable(name);

  if (!data) return <TableSkeleton />;

  const isVirtual = data.sql?.startsWith("CREATE VIRTUAL TABLE") ?? false;

  const cards: InfoCardProps[] = [
    {
      title: m.tables.counters.rows,
      value: data.row_count.toLocaleString(),
      description: m.tables.counters.rowsDescription,
      icon: TableIcon,
    },
    {
      title: m.tables.counters.indexes,
      value: data.index_count.toLocaleString(),
      description: m.tables.counters.indexesDescription,
      icon: DatabaseZap,
    },
    {
      title: m.tables.counters.columns,
      value: data.column_count.toLocaleString(),
      description: m.tables.counters.columnsDescription,
      icon: TableProperties,
    },
    {
      title: m.tables.counters.size,
      value: data.table_size,
      description: m.tables.counters.sizeDescription,
      icon: HardDrive,
    },
  ];

  return (
    <div className="flex flex-1 flex-col gap-4 md:gap-8">
      <h2 className="px-2 text-foreground scroll-m-20 border-b pb-2 text-3xl font-semibold tracking-tight first:mt-0 flex items-center gap-3">
        {data.name}
        {isVirtual && <Badge variant="secondary">{m.tables.virtualBadge}</Badge>}
      </h2>

      <div className="grid gap-4 md:grid-cols-2 md:gap-8 lg:grid-cols-4">
        {cards.map((card, i) => (
          <InfoCard
            key={i}
            title={card.title}
            value={card.value}
            description={card.description}
            icon={card.icon}
          />
        ))}
      </div>

      {data.sql && (
        <Card className="font-mono text-sm">
          <CodeBlock
            text={data.sql}
            language="sql"
            theme={currentTheme === "dark" ? CodeDarkTheme : undefined}
            showLineNumbers={false}
            customStyle={{
              FontFace: "JetBrains Mono",
              padding: "10px",
              backgroundColor: currentTheme === "dark" ? "#091813" : "#f5faf9",
              borderRadius: "10px",
            }}
          />
        </Card>
      )}

      <Card className="p-2">
        <TableDataGrid name={data.name} />
      </Card>
    </div>
  );
}

function TableSkeleton() {
  return (
    <div className="flex flex-1 flex-col gap-4 md:gap-8">
      <div className="flex flex-col gap-2">
        <Skeleton className="w-[50vw] h-[50px]" />
        <span className="border-b" />
      </div>
      <div className="grid gap-4 md:grid-cols-2 md:gap-8 lg:grid-cols-4">
        <Skeleton className="h-[100px]" />
        <Skeleton className="h-[100px]" />
        <Skeleton className="h-[100px]" />
        <Skeleton className="h-[100px]" />
      </div>
      <Skeleton className="h-[400px]" />
      <Skeleton className="h-[400px]" />
    </div>
  );
}

function isAtBottom({ currentTarget }: UIEvent<HTMLDivElement>): boolean {
  return (
    currentTarget.scrollTop + 10 >=
    currentTarget.scrollHeight - currentTarget.clientHeight
  );
}

interface TableDataGridProps {
  name: string;
}

function TableDataGrid({ name }: TableDataGridProps) {
  const currentTheme = useResolvedTheme();
  const { isLoading, data, fetchNextPage, hasNextPage } = useChTableData(name);

  if (!data) return <Skeleton className="h-[400px]" />;

  function handleScroll(event: UIEvent<HTMLDivElement>) {
    if (isLoading || !isAtBottom(event) || !hasNextPage) return;
    fetchNextPage();
  }

  const firstPage = data.pages[0];
  if (!firstPage) return <Skeleton className="h-[400px]" />;
  const columns = firstPage.columns.map((col) => ({ key: col, name: col }));

  const grouped = data.pages.map((page) =>
    page.rows.map((row) =>
      (row as unknown[]).reduce<Record<string, unknown>>((acc, curr, i) => {
        acc[page.columns[i]] = curr;
        return acc;
      }, {}),
    ),
  );
  const rows = grouped.flat();

  return (
    <DataGrid
      rows={rows}
      columns={columns}
      onScroll={handleScroll}
      defaultColumnOptions={{ resizable: true }}
      className={cn(currentTheme === "light" ? "rdg-light" : "rdg-dark")}
    />
  );
}
