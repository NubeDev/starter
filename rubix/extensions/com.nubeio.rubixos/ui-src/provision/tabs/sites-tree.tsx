// `sites-tree.tsx` — a shadcn/ui-style disclosure tree of the provisioning
// hierarchy: Site → (Locations, Pages), with real device counts derived from
// the device list. Pages deep-link to Page preview; the tree is read-only
// (create/edit live in the card view).
//
// shadcn has no core Tree primitive, so this is the shadcn *pattern*
// (collapsible rows, chevrons, indent guides, muted trailing counts) built
// with host theme tokens + Lucide icons, matching the rest of Provision.
import * as React from "react";
import {
  Building2,
  ChevronRight,
  FileText,
  Folder,
  MapPin,
  Radio,
} from "lucide-react";
import { listDevices, listLocations, listPages, listSites } from "../bc-api";
import { gotoDevice } from "../nav";
import type { DeviceRow, LocationRow, PageRow, SiteRow } from "../bc-types";
import { useRefreshKey } from "../refresh";

// A row in the tree. `depth` drives indentation; `count` is an optional
// trailing badge; `onActivate` makes the row a button (else a static label).
function TreeRow({
  depth,
  icon: Icon,
  label,
  sub,
  count,
  expandable,
  expanded,
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
  onToggle?: () => void;
  onActivate?: () => void;
  accent?: boolean;
}): React.ReactElement {
  const interactive = !!(onToggle || onActivate);
  return (
    <div
      role="treeitem"
      aria-expanded={expandable ? expanded : undefined}
      style={{ paddingLeft: `${depth * 1.25 + 0.5}rem` }}
      onClick={() => (onToggle ? onToggle() : onActivate?.())}
      className={
        "group flex items-center gap-1.5 rounded-md py-1.5 pr-2 text-sm transition-colors " +
        (interactive ? "cursor-pointer hover:bg-accent " : "")
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

export function SitesTree(): React.ReactElement {
  const [sites, setSites] = React.useState<ReadonlyArray<SiteRow>>([]);
  const [locations, setLocations] = React.useState<ReadonlyArray<LocationRow>>([]);
  const [pages, setPages] = React.useState<ReadonlyArray<PageRow>>([]);
  const [devices, setDevices] = React.useState<ReadonlyArray<DeviceRow>>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
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
        // Expand all sites by default so the hierarchy is visible at a glance.
        setExpanded(new Set(s.map((x) => `site:${x.site_id}`)));
      })
      .catch((e: unknown) => !cancelled && setError(e instanceof Error ? e.message : String(e)))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [refresh]);

  const toggle = (key: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  const isOpen = (key: string) => expanded.has(key);

  // Device-count helpers (derived once).
  const byLocation = React.useMemo(() => {
    const m = new Map<string, number>();
    for (const d of devices) if (d.location_id) m.set(d.location_id, (m.get(d.location_id) ?? 0) + 1);
    return m;
  }, [devices]);
  const byPage = React.useMemo(() => {
    const m = new Map<string, number>();
    for (const d of devices) if (d.page_id) m.set(d.page_id, (m.get(d.page_id) ?? 0) + 1);
    return m;
  }, [devices]);
  const bySite = React.useMemo(() => {
    const m = new Map<string, number>();
    for (const d of devices) if (d.site_id) m.set(d.site_id, (m.get(d.site_id) ?? 0) + 1);
    return m;
  }, [devices]);

  if (loading) return <p className="text-sm italic text-muted-foreground">Loading hierarchy…</p>;
  if (error) return <p className="text-sm text-destructive">{error}</p>;
  if (sites.length === 0)
    return (
      <div className="ext-glass flex flex-col items-center gap-2 px-6 py-10 text-center">
        <Building2 className="size-8 text-muted-foreground/60" />
        <p className="text-sm text-muted-foreground">No sites yet.</p>
      </div>
    );

  return (
    <div role="tree" className="ext-glass flex flex-col gap-0.5 p-2">
      {sites.map((s) => {
        const siteKey = `site:${s.site_id}`;
        const siteLocs = locations.filter((l) => l.site_id === s.site_id);
        const sitePages = pages.filter((p) => p.site_id === s.site_id);
        const locKey = `${siteKey}:locs`;
        const pageKey = `${siteKey}:pages`;
        return (
          <div key={s.site_id}>
            <TreeRow
              depth={0}
              icon={Building2}
              label={s.name}
              sub={s.site_id}
              count={bySite.get(s.site_id) ?? 0}
              expandable
              expanded={isOpen(siteKey)}
              onToggle={() => toggle(siteKey)}
              accent
            />
            {isOpen(siteKey) ? (
              <>
                {/* Locations group */}
                <TreeRow
                  depth={1}
                  icon={Folder}
                  label="Locations"
                  count={siteLocs.length}
                  expandable
                  expanded={isOpen(locKey)}
                  onToggle={() => toggle(locKey)}
                />
                {isOpen(locKey)
                  ? siteLocs.length === 0
                    ? <EmptyLeaf depth={2} text="No locations" />
                    : siteLocs.map((l) => (
                        <TreeRow
                          key={l.location_id}
                          depth={2}
                          icon={MapPin}
                          label={l.name}
                          count={byLocation.get(l.location_id) ?? 0}
                        />
                      ))
                  : null}

                {/* Pages group */}
                <TreeRow
                  depth={1}
                  icon={Folder}
                  label="Pages"
                  count={sitePages.length}
                  expandable
                  expanded={isOpen(pageKey)}
                  onToggle={() => toggle(pageKey)}
                />
                {isOpen(pageKey)
                  ? sitePages.length === 0
                    ? <EmptyLeaf depth={2} text="No pages" />
                    : sitePages.map((p) => (
                        <TreeRow
                          key={p.page_id}
                          depth={2}
                          icon={FileText}
                          label={p.name}
                          sub={p.page_id}
                          count={byPage.get(p.page_id) ?? 0}
                        />
                      ))
                  : null}
              </>
            ) : null}
          </div>
        );
      })}

      {/* Unassigned devices (no site) — surfaced so nothing is hidden. */}
      <UnassignedBranch
        devices={devices.filter((d) => !d.site_id)}
        expanded={isOpen("unassigned")}
        onToggle={() => toggle("unassigned")}
      />
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

function UnassignedBranch({
  devices,
  expanded,
  onToggle,
}: {
  devices: ReadonlyArray<DeviceRow>;
  expanded: boolean;
  onToggle: () => void;
}): React.ReactElement | null {
  if (devices.length === 0) return null;
  return (
    <>
      <TreeRow
        depth={0}
        icon={Folder}
        label="Unassigned devices"
        count={devices.length}
        expandable
        expanded={expanded}
        onToggle={onToggle}
      />
      {expanded
        ? devices.map((d) => (
            <TreeRow
              key={d.device_id}
              depth={1}
              icon={Radio}
              label={d.name ?? d.device_id}
              sub={d.device_id}
              onActivate={() => gotoDevice(d.device_id)}
            />
          ))
        : null}
    </>
  );
}
