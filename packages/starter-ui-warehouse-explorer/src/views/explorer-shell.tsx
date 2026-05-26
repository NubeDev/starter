// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: 1a0736055a4647c18d0be19347e4325007c7bd52.
// Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.
//
// Upstream's __root.tsx wrapped everything in TanStack Router's
// `createRootRoute` and used <Link>s for tab switching. The rubix
// shell owns the outer route — internal tab navigation lives in
// local state. Theme is read from the host (`.dark` on <html>); the
// in-header toggle is dropped. Shutdown button and GitHub link are
// dropped — they don't apply in the rubix shell.

import { useState, type ReactNode } from "react";
import { Code2, Database, Home, Network, Table as TableIcon } from "lucide-react";

import { cn } from "../lib/utils";
import { Overview } from "../views/overview";
import { Tables } from "../views/tables";
import { Query } from "../views/query";
import { Schema } from "../views/schema";
import {
  ExplorerI18nProvider,
  useExplorerMessages,
} from "../i18n/context";
import type { ExplorerMessages } from "../i18n/messages";

type Tab = "overview" | "tables" | "query" | "schema";

const TABS: { id: Tab; icon: typeof Home }[] = [
  { id: "overview", icon: Home },
  { id: "tables", icon: TableIcon },
  { id: "query", icon: Code2 },
  { id: "schema", icon: Network },
];

export interface ExplorerProps {
  /** Optional message overrides applied via `ExplorerI18nProvider`. */
  i18n?: Partial<ExplorerMessages>;
  /** Optional replacement for the built-in header (pass `<></>` to hide). */
  header?: ReactNode;
}

export function Explorer({ i18n, header }: ExplorerProps = {}) {
  return (
    <ExplorerI18nProvider value={i18n}>
      <ExplorerShellInner header={header} />
    </ExplorerI18nProvider>
  );
}

function ExplorerShellInner({ header }: { header?: ReactNode }) {
  const [tab, setTab] = useState<Tab>("overview");
  const messages = useExplorerMessages();
  const tabLabel = (id: Tab): string => messages.shell.tabs[id];

  return (
    <div className="flex min-h-screen w-full flex-col bg-background">
      {header !== undefined ? (
        header
      ) : (
        <header className="sticky top-0 flex h-14 items-center justify-between gap-4 border-b bg-background px-4 md:px-6 z-50">
          <nav className="flex flex-row items-center gap-2 sm:gap-3">
            <div className="flex items-center gap-2 pr-2 text-primary">
              <Database className="h-5 w-5" />
              <span className="hidden sm:inline font-mono text-sm font-semibold uppercase tracking-wide">
                Warehouse
              </span>
            </div>
            {TABS.map(({ id, icon: Icon }) => (
              <button
                key={id}
                type="button"
                onClick={() => setTab(id)}
                className={cn(
                  "inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-[13px] uppercase font-medium transition-colors",
                  tab === id
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground hover:bg-muted",
                )}
              >
                <Icon className="h-4 w-4" />
                <span className="hidden sm:inline">{tabLabel(id)}</span>
              </button>
            ))}
          </nav>
        </header>
      )}
      <main className="flex flex-1 flex-col gap-4 p-4 md:gap-8 md:p-8">
        {tab === "overview" && <Overview />}
        {tab === "tables" && <Tables />}
        {tab === "query" && <Query />}
        {tab === "schema" && <Schema />}
      </main>
    </div>
  );
}
