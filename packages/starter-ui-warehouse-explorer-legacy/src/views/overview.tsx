// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
//
// Overview view — counters + bar charts + per-table tables.
// Rewritten from the upstream `routes/index.tsx`:
//   * No `createFileRoute` / loader — data comes from
//     `useChOverview()` in `src/hooks`.
//   * Visible strings flow through `useExplorerMessages()`.
//   * The PR-4 `FreshnessTiles` / `MartTree` rubix overlays are
//     not mounted here in PR 1; PR 2 reintroduces them via
//     `@nube/rubix-client-react` typed hooks.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import {
  Workflow,
  TextSearch,
  DatabaseZap,
  Table as TableIcon,
} from "lucide-react";
import {
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  BarChart,
  ResponsiveContainer,
  TooltipContentProps,
} from "recharts";
import {
  NameType,
  ValueType,
} from "recharts/types/component/DefaultTooltipContent";

import {
  Card,
  CardTitle,
  CardHeader,
  CardContent,
  CardDescription,
} from "../components/ui/card.js";
import { Skeleton } from "../components/ui/skeleton.js";
import { InfoCard, InfoCardProps } from "../components/info-card.js";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../components/ui/table.js";
import { useChOverview } from "../hooks/index.js";
import { useExplorerMessages } from "../i18n/index.js";

export function ExplorerOverview() {
  const m = useExplorerMessages();
  const { data, isPending } = useChOverview();

  if (isPending || !data) return <OverviewSkeleton />;

  const cards: InfoCardProps[] = [
    {
      title: m.overview.counters.tables,
      value: data.tables.toLocaleString(),
      description: m.overview.counters.tablesDescription,
      icon: TableIcon,
    },
    {
      title: m.overview.counters.indexes,
      value: data.indexes.toLocaleString(),
      description: m.overview.counters.indexesDescription,
      icon: DatabaseZap,
    },
    {
      title: m.overview.counters.views,
      value: data.views.toLocaleString(),
      description: m.overview.counters.viewsDescription,
      icon: TextSearch,
    },
    {
      title: m.overview.counters.triggers,
      value: data.triggers.toLocaleString(),
      description: m.overview.counters.triggersDescription,
      icon: Workflow,
    },
  ];

  return (
    <>
      <h2 className="scroll-m-20 border-b pb-2 text-muted-foreground text-3xl tracking-tight first:mt-0">
        {m.overview.eyebrow}{" "}
        <span className="font-bold text-foreground">{data.file_name}</span>
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

      <div className="grid gap-8 lg:grid-cols-2 xl:grid-cols-7">
        <Card className="xl:col-span-4">
          <CardHeader>
            <CardTitle>{m.overview.sections.rowsPerTable}</CardTitle>
          </CardHeader>
          <CardContent className="pl-2">
            <TheBarChart counts={data.row_counts} />
          </CardContent>
        </Card>
        <Card className="xl:col-span-3">
          <CardHeader className="flex flex-row items-center">
            <div className="grid gap-2">
              <CardTitle>{m.overview.sections.moreMetadata}</CardTitle>
              <CardDescription>
                {m.overview.sections.moreMetadataDescription}
              </CardDescription>
            </div>
          </CardHeader>
          <CardContent>
            <Table>
              <TableBody>
                <MetadataRow
                  name={m.overview.metadata.databaseSize}
                  description={m.overview.metadata.databaseSizeDescription}
                  value={data.size_on_disk}
                />
                {data.created && (
                  <MetadataRow
                    name={m.overview.metadata.createdOn}
                    description={m.overview.metadata.createdOnDescription}
                    value={data.created.toUTCString()}
                  />
                )}
                {data.modified && (
                  <MetadataRow
                    name={m.overview.metadata.modifiedOn}
                    description={m.overview.metadata.modifiedOnDescription}
                    value={data.modified.toUTCString()}
                  />
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-8 lg:grid-cols-2 xl:grid-cols-7">
        <Card className="xl:col-span-3">
          <CardHeader>
            <CardTitle>{m.overview.sections.indexesPerTable}</CardTitle>
          </CardHeader>
          <CardContent className="pl-2">
            <TheBarChart counts={data.index_counts} />
          </CardContent>
        </Card>
        <Card className="xl:col-span-4">
          <CardHeader>
            <CardTitle>{m.overview.sections.columnsPerTable}</CardTitle>
          </CardHeader>
          <CardContent className="pl-2">
            <TheBarChart counts={data.column_counts} />
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-8 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>{m.overview.sections.indexesPerTable}</CardTitle>
          </CardHeader>
          <CardContent className="pl-2 h-[400px] overflow-y-scroll">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{m.overview.indexesHeaderIndex}</TableHead>
                  <TableHead className="text-right">
                    {m.overview.indexesHeaderCount}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.index_counts.map((col) => (
                  <TableRow key={col.name}>
                    <TableCell>{col.name}</TableCell>
                    <TableCell className="text-right">{col.count}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{m.overview.sections.columnsPerTable}</CardTitle>
          </CardHeader>
          <CardContent className="pl-2 h-[400px] overflow-y-scroll">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{m.overview.columnsHeaderColumn}</TableHead>
                  <TableHead className="text-right">
                    {m.overview.columnsHeaderCount}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.column_counts.map((col) => (
                  <TableRow key={col.name}>
                    <TableCell>{col.name}</TableCell>
                    <TableCell className="text-right">{col.count}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>
    </>
  );
}

interface MetadataRowProps {
  name: string;
  description: string;
  value: string;
}

function MetadataRow({ name, description, value }: MetadataRowProps) {
  return (
    <TableRow>
      <TableCell>
        <div className="font-medium">{name}</div>
        <div className="text-sm text-muted-foreground md:inline">
          {description}
        </div>
      </TableCell>
      <TableCell className="text-right">{value}</TableCell>
    </TableRow>
  );
}

interface TheBarChartProps {
  counts: { count: number; name: string }[];
}

const compactNumberFormatter = Intl.NumberFormat("en-US", {
  notation: "compact",
  maximumFractionDigits: 1,
});

export function TheBarChart({ counts }: TheBarChartProps) {
  return (
    <ResponsiveContainer width="100%" height={350}>
      <BarChart data={counts}>
        <XAxis
          dataKey="name"
          stroke="#888888"
          fontSize={12}
          tickLine={false}
          axisLine={false}
          className="hidden"
        />
        <YAxis
          stroke="#888888"
          fontSize={12}
          tickLine={false}
          axisLine={false}
          tickFormatter={(number) => compactNumberFormatter.format(number)}
        />
        <Bar
          dataKey="count"
          fill="currentColor"
          radius={[4, 4, 0, 0]}
          className="fill-primary"
        />
        <Tooltip content={CustomTooltip} cursor={{ fill: "#00ffa61e" }} />
      </BarChart>
    </ResponsiveContainer>
  );
}

function OverviewSkeleton() {
  return (
    <>
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
      <div className="w-full grid gap-4 lg:grid-cols-2 xl:grid-cols-7">
        <Skeleton className="xl:col-span-4 h-[400px]" />
        <Skeleton className="xl:col-span-3 h-[400px]" />
      </div>
      <div className="w-full grid gap-4 lg:grid-cols-2 xl:grid-cols-7">
        <Skeleton className="xl:col-span-3 h-[400px]" />
        <Skeleton className="xl:col-span-4 h-[400px]" />
      </div>
    </>
  );
}

function CustomTooltip({
  active,
  payload,
  label,
}: TooltipContentProps<ValueType, NameType>) {
  if (!active || !payload || !payload.length) return null;
  const value = payload[0]?.value;
  return (
    <Card className="p-3">
      <CardContent className="p-0">
        <div className="font-bold"># {value?.toLocaleString?.()}</div>
        <p className="text-xs text-muted-foreground">{String(label)}</p>
      </CardContent>
    </Card>
  );
}
