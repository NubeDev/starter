// `place.tsx` — pick site, then location (or new), then optional page (or new).
import * as React from "react";
import { listLocations, listPages, listSites } from "../provision/bc-api";
import type { LocationRow, PageRow, SiteRow } from "../provision/bc-types";

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

  React.useEffect(() => {
    listSites().then(setSites).catch(() => setSites([]));
    listPages().then(setPages).catch(() => setPages([]));
  }, []);
  React.useEffect(() => {
    if (!value.siteId) return setLocations([]);
    listLocations({ site_id: value.siteId }).then(setLocations).catch(() => setLocations([]));
  }, [value.siteId]);

  return (
    <div className="flex flex-col gap-4">
      <Field label="Site">
        <Picker
          value={value.siteId}
          placeholder="Select a site"
          options={sites.map((s) => ({ value: s.site_id, label: s.name }))}
          onChange={(v) => set({ siteId: v, locationId: "", newLocation: "" })}
        />
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

      <Field label="Page (optional)">
        <Picker
          value={value.pageId}
          placeholder="+ New page"
          options={pages.map((p) => ({ value: p.page_id, label: p.name }))}
          onChange={(v) => set({ pageId: v, newPage: "" })}
        />
        {!value.pageId ? (
          <input
            value={value.newPage}
            onChange={(e) => set({ newPage: e.target.value })}
            placeholder="New page name (optional)"
            className="mt-2 w-full rounded-lg border border-border/60 bg-background px-3 py-3 text-base text-foreground outline-none focus:border-primary"
          />
        ) : null}
      </Field>
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
