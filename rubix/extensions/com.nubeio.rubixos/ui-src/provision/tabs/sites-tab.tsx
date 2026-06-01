// `sites-tab.tsx` — sites + locations as a responsive card grid with
// inline create. Visual language matches the rest of Provision: glass
// surfaces, eyebrow labels, host theme tokens (no hardcoded palette).
import * as React from "react";
import { Building2, LayoutGrid, ListTree, MapPin, Plus } from "lucide-react";
import { listLocations, listSites, locationCreate, siteCreate } from "../bc-api";
import { useRefreshKey } from "../refresh";
import type { LocationRow, SiteRow } from "../bc-types";
import { SitesTree } from "./sites-tree";

type View = "cards" | "tree";

const newId = (prefix: string) =>
  `${prefix}_${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

export function SitesTab(): React.ReactElement {
  const [sites, setSites] = React.useState<ReadonlyArray<SiteRow>>([]);
  const [locations, setLocations] = React.useState<ReadonlyArray<LocationRow>>([]);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [siteName, setSiteName] = React.useState("");
  const [locName, setLocName] = React.useState<Record<string, string>>({});
  const [view, setView] = React.useState<View>("cards");
  const refresh = useRefreshKey();

  const load = React.useCallback(() => {
    setLoading(true);
    setError(null);
    Promise.all([listSites(), listLocations()])
      .then(([s, l]) => {
        setSites(s);
        setLocations(l);
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, []);

  React.useEffect(load, [load, refresh]);

  const addSite = () => {
    if (!siteName.trim()) return;
    siteCreate({ site_id: newId("site"), name: siteName.trim() })
      .then(() => {
        setSiteName("");
        load();
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  const addLocation = (siteId: string) => {
    const name = (locName[siteId] ?? "").trim();
    if (!name) return;
    locationCreate({ location_id: newId("loc"), site_id: siteId, name })
      .then(() => {
        setLocName((m) => ({ ...m, [siteId]: "" }));
        load();
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  // Submit a location on Enter for the given site.
  const locKeyDown = (siteId: string) => (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") addLocation(siteId);
  };

  return (
    <div className="flex flex-col gap-4">
      {error ? (
        <div role="alert" className="rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {/* View toggle: cards (create/edit) vs tree (browse hierarchy) */}
      <div className="flex w-fit self-end rounded-lg border border-border/60 bg-muted/20 p-0.5 text-sm">
        <button
          type="button"
          onClick={() => setView("cards")}
          className={"flex cursor-pointer items-center gap-1.5 rounded-md px-3 py-1.5 transition-colors " + (view === "cards" ? "bg-card font-medium text-foreground shadow-sm ring-1 ring-border/60" : "text-muted-foreground hover:text-foreground")}
        >
          <LayoutGrid className="size-3.5" /> Cards
        </button>
        <button
          type="button"
          onClick={() => setView("tree")}
          className={"flex cursor-pointer items-center gap-1.5 rounded-md px-3 py-1.5 transition-colors " + (view === "tree" ? "bg-card font-medium text-foreground shadow-sm ring-1 ring-border/60" : "text-muted-foreground hover:text-foreground")}
        >
          <ListTree className="size-3.5" /> Tree
        </button>
      </div>

      {view === "tree" ? (
        <SitesTree />
      ) : (
      <>
      {/* Quick-add site bar */}
      <div className="ext-glass flex flex-col gap-2 p-3 sm:flex-row sm:items-center">
        <input
          value={siteName}
          onChange={(e) => setSiteName(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") addSite(); }}
          placeholder="New site name (e.g. Building A)"
          aria-label="New site name"
          className="flex-1 rounded-lg border border-border/60 bg-background px-3 py-2 text-sm text-foreground outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/30"
        />
        <button
          type="button"
          onClick={addSite}
          disabled={!siteName.trim()}
          className="flex cursor-pointer items-center justify-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Plus className="size-4" /> Site
        </button>
      </div>

      {loading ? (
        <p className="text-sm italic text-muted-foreground">Loading sites…</p>
      ) : sites.length === 0 ? (
        <div className="ext-glass flex flex-col items-center gap-2 px-6 py-10 text-center">
          <Building2 className="size-8 text-muted-foreground/60" />
          <p className="text-sm text-muted-foreground">No sites yet. Create one above to get started.</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          {sites.map((s) => {
            const locs = locations.filter((l) => l.site_id === s.site_id);
            return (
              <div key={s.site_id} className="ext-glass group flex flex-col transition-colors hover:border-primary/40">
                {/* Card header band */}
                <header className="flex items-start justify-between gap-2 border-b border-border/60 bg-muted/20 px-4 py-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <Building2 className="size-4 shrink-0 text-primary" />
                      <h3 className="truncate text-sm font-semibold text-foreground">{s.name}</h3>
                    </div>
                    <span className="mt-1 inline-block rounded bg-muted/50 px-1.5 py-0.5 font-mono text-[11px] text-muted-foreground">
                      {s.site_id}
                    </span>
                  </div>
                  <span className="ext-num shrink-0 rounded-full border border-border/60 px-2 py-0.5 text-[11px] text-muted-foreground">
                    {locs.length} loc{locs.length === 1 ? "" : "s"}
                  </span>
                </header>

                {/* Locations list */}
                <div className="flex-1 px-4 py-3">
                  <div className="ext-eyebrow mb-2">Locations</div>
                  {locs.length === 0 ? (
                    <p className="text-xs italic text-muted-foreground">No locations defined yet.</p>
                  ) : (
                    <ul className="flex flex-col gap-1.5">
                      {locs.map((l) => (
                        <li
                          key={l.location_id}
                          className="flex items-center gap-2 rounded-md border border-border/40 bg-muted/20 px-2.5 py-1.5 text-sm text-foreground"
                        >
                          <span className="inline-block size-1.5 shrink-0 rounded-full bg-primary" />
                          <span className="truncate">{l.name}</span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>

                {/* Add-location footer */}
                <div className="flex gap-2 border-t border-border/60 bg-muted/10 px-4 py-3">
                  <div className="relative flex-1">
                    <MapPin className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
                    <input
                      value={locName[s.site_id] ?? ""}
                      onChange={(e) => setLocName((m) => ({ ...m, [s.site_id]: e.target.value }))}
                      onKeyDown={locKeyDown(s.site_id)}
                      placeholder="New location"
                      aria-label={`New location in ${s.name}`}
                      className="w-full rounded-lg border border-border/60 bg-background py-1.5 pl-8 pr-2.5 text-sm text-foreground outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/30"
                    />
                  </div>
                  <button
                    type="button"
                    onClick={() => addLocation(s.site_id)}
                    disabled={!(locName[s.site_id] ?? "").trim()}
                    className="cursor-pointer rounded-lg border border-border/60 px-3 py-1.5 text-sm font-medium text-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    Add
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
      </>
      )}
    </div>
  );
}
