// `place.tsx` — pick site, then location (or new), then optional page (or new).
import * as React from "react";
import { listLocations, listPages, listSites, siteCreate } from "../provision/bc-api";
import { useRefreshKey } from "../provision/refresh";
import type { LocationRow, PageRow, SiteRow } from "../provision/bc-types";

const newSiteId = () =>
  `site_${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

/** Placement choice shared by the PWA and the desktop wizard. */
export interface Placement {
  siteId: string;
  locationId: string;
  newLocation: string;
  pageId: string;
  newPage: string;
}

export const EMPTY_PLACEMENT: Placement = {
  siteId: "",
  locationId: "",
  newLocation: "",
  pageId: "",
  newPage: "",
};

export function Place({
  value,
  onChange,
}: {
  value: Placement;
  onChange: (next: Placement) => void;
}): React.ReactElement {
  const [sites, setSites] = React.useState<ReadonlyArray<SiteRow>>([]);
  const [locations, setLocations] = React.useState<ReadonlyArray<LocationRow>>([]);
  const [pages, setPages] = React.useState<ReadonlyArray<PageRow>>([]);
  const set = (patch: Partial<Placement>) => onChange({ ...value, ...patch });
  const refresh = useRefreshKey();
  const [newSite, setNewSite] = React.useState("");
  const [creatingSite, setCreatingSite] = React.useState(false);

  const createSite = () => {
    const nm = newSite.trim();
    if (!nm || creatingSite) return;
    const id = newSiteId();
    setCreatingSite(true);
    siteCreate({ site_id: id, name: nm })
      // Refetch the authoritative list FIRST so the new <option> exists
      // before we select it — a controlled <select> whose value isn't
      // among its options renders blank and won't hold the selection.
      .then(() => listSites().catch(() => [{ site_id: id, name: nm } as SiteRow]))
      .then((list) => {
        setSites(list.some((s) => s.site_id === id) ? list : [...list, { site_id: id, name: nm } as SiteRow]);
        setNewSite("");
        set({ siteId: id, locationId: "", newLocation: "" });
      })
      .finally(() => setCreatingSite(false));
  };

  React.useEffect(() => {
    listSites().then(setSites).catch(() => setSites([]));
  }, [refresh]);
  React.useEffect(() => {
    if (!value.siteId) return setLocations([]);
    listLocations({ site_id: value.siteId }).then(setLocations).catch(() => setLocations([]));
  }, [value.siteId, refresh]);
  // Pages are scoped to the chosen site — a page belongs to one site,
  // so you only pick/create pages within the site you're placing into.
  React.useEffect(() => {
    if (!value.siteId) return setPages([]);
    listPages({ site_id: value.siteId }).then(setPages).catch(() => setPages([]));
  }, [value.siteId, refresh]);

  return (
    <div className="flex flex-col gap-4">
      <Field label="Site">
        <Picker
          value={value.siteId}
          placeholder={sites.length ? "+ New site" : "+ Create your first site"}
          options={sites.map((s) => ({ value: s.site_id, label: s.name }))}
          onChange={(v) => set({ siteId: v, locationId: "", newLocation: "" })}
        />
        {!value.siteId ? (
          <div className="mt-2 flex gap-2">
            <input
              value={newSite}
              onChange={(e) => setNewSite(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && createSite()}
              placeholder="New site name (e.g. Building A)"
              aria-label="New site name"
              className="w-full rounded-lg border border-border/60 bg-background px-3 py-3 text-base text-foreground outline-none focus:border-primary"
            />
            <button
              type="button"
              onClick={createSite}
              disabled={!newSite.trim() || creatingSite}
              className="shrink-0 rounded-lg bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50"
            >
              {creatingSite ? "…" : "Create"}
            </button>
          </div>
        ) : null}
      </Field>

      {value.siteId ? (
        <Field label="Location">
          <Picker
            value={value.locationId}
            placeholder="+ New location"
            options={locations.map((l) => ({ value: l.location_id, label: l.name }))}
            onChange={(v) => set({ locationId: v, newLocation: "" })}
          />
          {!value.locationId ? (
            <input
              value={value.newLocation}
              onChange={(e) => set({ newLocation: e.target.value })}
              placeholder="New location name"
              className="mt-2 w-full rounded-lg border border-border/60 bg-background px-3 py-3 text-base text-foreground outline-none focus:border-primary"
            />
          ) : null}
        </Field>
      ) : null}

      {value.siteId ? (
        <Field label="Dashboard page">
          <Picker
            value={value.pageId}
            placeholder="+ New page for this site"
            options={pages.map((p) => ({ value: p.page_id, label: p.name }))}
            onChange={(v) => set({ pageId: v, newPage: "" })}
          />
          {!value.pageId ? (
            <input
              value={value.newPage}
              onChange={(e) => set({ newPage: e.target.value })}
              placeholder="New page name (e.g. Floor 3)"
              className="mt-2 w-full rounded-lg border border-border/60 bg-background px-3 py-3 text-base text-foreground outline-none focus:border-primary"
            />
          ) : null}
        </Field>
      ) : null}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }): React.ReactElement {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      {children}
    </div>
  );
}

function Picker({
  value,
  options,
  placeholder,
  onChange,
}: {
  value: string;
  options: ReadonlyArray<{ value: string; label: string }>;
  placeholder: string;
  onChange: (v: string) => void;
}): React.ReactElement {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full rounded-lg border border-border/60 bg-background px-3 py-3 text-base text-foreground outline-none focus:border-primary"
    >
      <option value="">{placeholder}</option>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
