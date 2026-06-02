// `page-preview-tab.tsx` — the client view: browse the provisioning
// hierarchy as a shadcn/ui-style explorer tree (Site → Location → Page)
// and render the selected page's widgets beside it. Mirrors how an end
// user navigates "Building A → Level 2 → its dashboard".
//
// Schema note: a Page carries `site_id` and (optionally) `location_id`,
// so it slots directly into the Site → Location → Page tree. Pages with
// no `location_id` — legacy pages, or ones pinned to a site but no
// specific location — hang in a site-level "Unassigned pages" group so
// nothing is hidden. Device counts are derived from the device list.
import * as React from "react";
import { Building2, ChevronRight, FileText, Folder, MapPin } from "lucide-react";
import { listDevices, listLocations, listPages, listSites } from "../bc-api";
import { useRefreshKey } from "../refresh";
import type { DeviceRow, LocationRow, PageRow, SiteRow } from "../bc-types";
import { PageView } from "../page-render/page-view";

// A single tree row. `depth` drives indentation; `count` is an optional
// trailing badge; `selected` highlights the active page leaf.
function TreeRow({
  depth,
  icon: Icon,
  label,
  sub,
  count,
  expandable,
  expanded,
  selected,
  onToggle,
  onActivate,
  accent,
}: {
  depth: number;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  sub?: string;
  count?: number;
  expandable?: boolean;
  expanded?: boolean;
  selected?: boolean;
  onToggle?: () => void;
  onActivate?: () => void;
  accent?: boolean;
}): React.ReactElement {
  const interactive = !!(onToggle || onActivate);
  return (
    <div
      role="treeitem"
      aria-expanded={expandable ? expanded : undefined}
      aria-selected={onActivate ? !!selected : undefined}
      style={{ paddingLeft: `${depth * 1.25 + 0.5}rem` }}
      onClick={() => (onToggle ? onToggle() : onActivate?.())}
      className={
        "group flex items-center gap-1.5 rounded-md py-1.5 pr-2 text-sm transition-colors " +
        (interactive ? "cursor-pointer hover:bg-accent " : "") +
        (selected ? "bg-accent text-accent-foreground " : "")
      }
    >
      <span className="flex size-4 shrink-0 items-center justify-center text-muted-foreground">
        {expandable ? (
          <ChevronRight className={"size-4 transition-transform duration-150 " + (expanded ? "rotate-90" : "")} />
        ) : null}
      </span>
      <Icon className={"size-4 shrink-0 " + (accent ? "text-primary" : "text-muted-foreground")} />
      <span className={"truncate " + (accent ? "font-medium text-foreground" : "text-foreground")}>{label}</span>
      {sub ? <span className="truncate font-mono text-[11px] text-muted-foreground">{sub}</span> : null}
      {typeof count === "number" ? (
        <span className="ext-num ml-auto shrink-0 rounded-full border border-border/60 px-1.5 py-0.5 text-[11px] text-muted-foreground">
          {count}
        </span>
      ) : null}
    </div>
  );
}

function EmptyLeaf({ depth, text }: { depth: number; text: string }): React.ReactElement {
  return (
    <div style={{ paddingLeft: `${depth * 1.25 + 0.5}rem` }} className="py-1 pl-6 text-xs italic text-muted-foreground">
      {text}
    </div>
  );
}

export function PagePreviewTab(): React.ReactElement {
  const [sites, setSites] = React.useState<ReadonlyArray<SiteRow>>([]);
  const [locations, setLocations] = React.useState<ReadonlyArray<LocationRow>>([]);
  const [pages, setPages] = React.useState<ReadonlyArray<PageRow>>([]);
  const [devices, setDevices] = React.useState<ReadonlyArray<DeviceRow>>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [pageId, setPageId] = React.useState("");
  const [expanded, setExpanded] = React.useState<Set<string>>(new Set());
  const refresh = useRefreshKey();

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([listSites(), listLocations(), listPages({}), listDevices({ limit: 1000 })])
      .then(([s, l, p, d]) => {
        if (cancelled) return;
        setSites(s);
        setLocations(l);
        setPages(p);
        setDevices(d);
        // Expand every site by default so pages are reachable in one click.
        setExpanded(new Set(s.map((x) => `site:${x.site_id}`)));
      })
      .catch((e: unknown) => !cancelled && setError(e instanceof Error ? e.message : String(e)))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [refresh]);

  // device count per page, for the trailing badge.
  const devicesByPage = React.useMemo(() => {
    const m = new Map<string, number>();
    for (const d of devices) if (d.page_id) m.set(d.page_id, (m.get(d.page_id) ?? 0) + 1);
    return m;
  }, [devices]);

  const toggle = (key: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  const isOpen = (key: string) => expanded.has(key);

  const renderPage = (p: PageRow, depth: number) => (
    <TreeRow
      key={`${depth}:${p.page_id}`}
      depth={depth}
      icon={FileText}
      label={p.name}
      sub={p.page_id}
      count={devicesByPage.get(p.page_id) ?? 0}
      selected={pageId === p.page_id}
      onActivate={() => setPageId(p.page_id)}
    />
  );

  const tree = (
    <div role="tree" className="ext-glass flex flex-col gap-0.5 p-2">
      {sites.length === 0 ? (
        <div className="flex flex-col items-center gap-2 px-6 py-10 text-center">
          <Building2 className="size-8 text-muted-foreground/60" />
          <p className="text-sm text-muted-foreground">No sites yet.</p>
        </div>
      ) : (
        sites.map((s) => {
          const siteKey = `site:${s.site_id}`;
          const siteLocs = locations.filter((l) => l.site_id === s.site_id);
          const sitePages = pages.filter((p) => p.site_id === s.site_id);
          // A page is "unassigned" when it carries no location_id.
          const unassigned = sitePages.filter((p) => !p.location_id);
          const unassignedKey = `${siteKey}:unassigned`;
          return (
            <div key={s.site_id}>
              <TreeRow
                depth={0}
                icon={Building2}
                label={s.name}
                sub={s.site_id}
                count={sitePages.length}
                expandable
                expanded={isOpen(siteKey)}
                onToggle={() => toggle(siteKey)}
                accent
              />
              {isOpen(siteKey) ? (
                <>
                  {siteLocs.map((l) => {
                    const locKey = `${siteKey}:loc:${l.location_id}`;
                    const locPages = sitePages.filter((p) => p.location_id === l.location_id);
                    return (
                      <div key={l.location_id}>
                        <TreeRow
                          depth={1}
                          icon={MapPin}
                          label={l.name}
                          count={locPages.length}
                          expandable
                          expanded={isOpen(locKey)}
                          onToggle={() => toggle(locKey)}
                        />
                        {isOpen(locKey)
                          ? locPages.length === 0
                            ? <EmptyLeaf depth={2} text="No pages" />
                            : locPages.map((p) => renderPage(p, 2))
                          : null}
                      </div>
                    );
                  })}

                  {/* Pages not pinned to any location, kept at site level. */}
                  {unassigned.length > 0 ? (
                    <>
                      <TreeRow
                        depth={1}
                        icon={Folder}
                        label="Unassigned pages"
                        count={unassigned.length}
                        expandable
                        expanded={isOpen(unassignedKey)}
                        onToggle={() => toggle(unassignedKey)}
                      />
                      {isOpen(unassignedKey) ? unassigned.map((p) => renderPage(p, 2)) : null}
                    </>
                  ) : null}

                  {siteLocs.length === 0 && unassigned.length === 0 ? (
                    <EmptyLeaf depth={1} text="No pages for this site yet" />
                  ) : null}
                </>
              ) : null}
            </div>
          );
        })
      )}
    </div>
  );

  return (
    <div className="flex flex-col gap-4">
      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}
      {loading ? (
        <p className="text-sm italic text-muted-foreground">Loading hierarchy…</p>
      ) : (
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start">
          <div className="w-full shrink-0 lg:w-80">{tree}</div>
          <div className="min-w-0 flex-1">
            <PageView pageId={pageId} />
          </div>
        </div>
      )}
    </div>
  );
}
