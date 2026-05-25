// Tabbed shell composing the four view components. The tab strip
// styling mirrors upstream sql-studio's uppercase header nav
// (https://github.com/frectonz/sql-studio,
// `ui/src/routes/__root.tsx`) rather than the rubix kit's pill
// `<Tabs>` — keeps the explorer visually recognisable to anyone
// who has used the standalone tool.
//
// Hosts wanting a different layout can mount the individual view
// components (`<ExplorerOverview />`, `<ExplorerTables />`, …)
// directly from `./views`.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { useState, type ReactNode } from "react";

import { cn } from "../lib/utils.js";
import { ExplorerI18nProvider } from "../i18n/context.js";
import {
  useExplorerMessages,
  type ExplorerMessages,
} from "../i18n/index.js";
import { ExplorerOverview } from "./overview.js";
import { ExplorerTables } from "./tables.js";
import { ExplorerSchema } from "./schema.js";
import { ExplorerQuery } from "./query.js";

export type ExplorerTab = "overview" | "tables" | "schema" | "query";

export interface ExplorerProps {
  /** Initial tab. Defaults to `"overview"`. */
  defaultTab?: ExplorerTab;
  /** Optional partial i18n override merged on top of
   * `DEFAULT_EXPLORER_MESSAGES`. */
  i18n?: Partial<ExplorerMessages>;
  /** Optional slot rendered above the tabs (page header). */
  header?: ReactNode;
}

export function Explorer({ defaultTab = "overview", i18n, header }: ExplorerProps) {
  return (
    <ExplorerI18nProvider value={i18n}>
      <ExplorerInner defaultTab={defaultTab} header={header} />
    </ExplorerI18nProvider>
  );
}

const TABS: ExplorerTab[] = ["overview", "tables", "query", "schema"];

function ExplorerInner({
  defaultTab,
  header,
}: {
  defaultTab: ExplorerTab;
  header?: ReactNode;
}) {
  const m = useExplorerMessages();
  const [tab, setTab] = useState<ExplorerTab>(defaultTab);

  const labels: Record<ExplorerTab, string> = {
    overview: m.shell.tabs.overview,
    tables: m.shell.tabs.tables,
    query: m.shell.tabs.query,
    schema: m.shell.tabs.schema,
  };

  return (
    <div className="flex flex-col gap-6">
      {header ?? (
        <header>
          <h1 className="text-2xl font-semibold tracking-tight">
            {m.shell.title}
          </h1>
        </header>
      )}

      {/* Upstream sql-studio nav: uppercase links, active is
        * primary + extra-bold, separated from content by a thin
        * bottom border. No pill chrome — keeps the four views
        * visually flush with the page below. */}
      <nav
        role="tablist"
        aria-label={m.shell.title}
        className="flex items-center gap-5 md:gap-6 border-b pb-2"
      >
        {TABS.map((id) => {
          const active = tab === id;
          return (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => setTab(id)}
              className={cn(
                "uppercase text-[14px] tracking-wide transition-colors",
                active
                  ? "text-primary font-extrabold"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {labels[id]}
            </button>
          );
        })}
      </nav>

      <section role="tabpanel" aria-label={labels[tab]}>
        {tab === "overview" && <ExplorerOverview />}
        {tab === "tables" && <ExplorerTables />}
        {tab === "query" && <ExplorerQuery />}
        {tab === "schema" && <ExplorerSchema />}
      </section>
    </div>
  );
}
