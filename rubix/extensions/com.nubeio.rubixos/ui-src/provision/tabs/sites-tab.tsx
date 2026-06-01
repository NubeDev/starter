// `sites-tab.tsx` — sites + locations tree with inline create.
import * as React from "react";
import { MapPin, Plus } from "lucide-react";
import { listLocations, listSites, locationCreate, siteCreate } from "../bc-api";
import type { LocationRow, SiteRow } from "../bc-types";

const newId = (prefix: string) =>
  `${prefix}_${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

export function SitesTab(): React.ReactElement {
  const [sites, setSites] = React.useState<ReadonlyArray<SiteRow>>([]);
  const [locations, setLocations] = React.useState<ReadonlyArray<LocationRow>>([]);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [siteName, setSiteName] = React.useState("");
  const [locName, setLocName] = React.useState<Record<string, string>>({});

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

  React.useEffect(load, [load]);

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

  return (
    <div className="flex max-w-2xl flex-col gap-4">
      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      <div className="flex gap-2">
        <input
          value={siteName}
          onChange={(e) => setSiteName(e.target.value)}
          placeholder="New site name"
          aria-label="New site name"
          className="flex-1 rounded-md border border-border/60 bg-background px-3 py-1.5 text-sm text-foreground outline-none focus:border-primary"
        />
        <button type="button" onClick={addSite} className="flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground">
          <Plus className="size-4" /> Site
        </button>
      </div>

      {loading ? (
        <p className="text-sm italic text-muted-foreground">loading…</p>
      ) : sites.length === 0 ? (
        <p className="text-sm italic text-muted-foreground">No sites yet. Create one above.</p>
      ) : (
        <ul className="flex flex-col gap-3">
          {sites.map((s) => (
            <li key={s.site_id} className="rounded-lg border border-border/60 bg-card">
              <div className="flex items-center gap-2 border-b border-border/60 px-3 py-2">
                <MapPin className="size-4 text-muted-foreground" />
                <span className="text-sm font-medium text-foreground">{s.name}</span>
                <span className="font-mono text-xs text-muted-foreground">{s.site_id}</span>
              </div>
              <ul className="px-3 py-2">
                {locations
                  .filter((l) => l.site_id === s.site_id)
                  .map((l) => (
                    <li key={l.location_id} className="py-0.5 text-sm text-foreground">
                      • {l.name}
                    </li>
                  ))}
                <li className="mt-2 flex gap-2">
                  <input
                    value={locName[s.site_id] ?? ""}
                    onChange={(e) => setLocName((m) => ({ ...m, [s.site_id]: e.target.value }))}
                    placeholder="New location"
                    aria-label={`New location in ${s.name}`}
                    className="flex-1 rounded-md border border-border/60 bg-background px-2 py-1 text-sm text-foreground outline-none focus:border-primary"
                  />
                  <button type="button" onClick={() => addLocation(s.site_id)} className="rounded-md border border-border/60 px-2 py-1 text-sm hover:bg-accent">
                    Add
                  </button>
                </li>
              </ul>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
