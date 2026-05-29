// `site-map.tsx` — interactive multi-site map using react-map-gl
// over MapLibre GL + Carto's free dark basemap (no token needed).
//
// Each site is rendered as a glowing pulse marker scaled by its
// share of the period total. Click cycles selection through the
// dashboard's site filter so users can drill into one site at a
// time without leaving the map.

import * as React from "react";
// Alias `Map` → `MapGL`: a bare `Map` import shadows the global
// `Map` constructor, so `new Map<...>()` below (and any other Map
// use in this file) would call the React component instead — which
// fails at runtime with `Map$1 is not a constructor` after Rollup
// renames the duplicated identifier.
import { Map as MapGL, Marker, NavigationControl, Popup } from "react-map-gl/maplibre";
import "maplibre-gl/dist/maplibre-gl.css";
import { useHostThemeMode } from "./use-host-theme-mode";

import type { SiteGeo } from "./sites-geo";

export interface SiteMarker {
  site: SiteGeo;
  value: number;
  selected: boolean;
}

// Locality cluster — collapses markers that share a `locality`
// string (e.g. both "Berrinba, QLD" sites or both "RC-1"/"RC-2" at
// Chullora) into a single bubble. Scales gracefully when many
// buildings cluster in the same business park / industrial estate.
interface Cluster {
  key: string;
  locality: string;
  lat: number;
  lon: number;
  totalValue: number;
  members: ReadonlyArray<SiteMarker>;
  anySelected: boolean;
  allSelected: boolean;
}

function buildClusters(markers: ReadonlyArray<SiteMarker>): ReadonlyArray<Cluster> {
  const groups = new Map<string, SiteMarker[]>();
  for (const m of markers) {
    const k = m.site.locality || `${m.site.lat.toFixed(2)},${m.site.lon.toFixed(2)}`;
    const arr = groups.get(k) ?? [];
    arr.push(m);
    groups.set(k, arr);
  }
  const out: Cluster[] = [];
  for (const [key, members] of groups) {
    const totalValue = members.reduce((s, m) => s + m.value, 0);
    // Centroid for the bubble position.
    const lat = members.reduce((s, m) => s + m.site.lat, 0) / members.length;
    const lon = members.reduce((s, m) => s + m.site.lon, 0) / members.length;
    out.push({
      key,
      locality: members[0]!.site.locality || key,
      lat,
      lon,
      totalValue,
      members,
      anySelected: members.some((m) => m.selected),
      allSelected: members.every((m) => m.selected),
    });
  }
  return out;
}

export function SiteMap({
  markers,
  unit,
  onToggleSite,
  height = 360,
}: {
  markers: ReadonlyArray<SiteMarker>;
  unit: string | null;
  onToggleSite: (host_uuid: string) => void;
  height?: number;
}): React.ReactElement {
  const [hovered, setHovered] = React.useState<string | null>(null);
  // The currently-"expanded" cluster: when clicked, its members
  // fan out individually so users can pick one of N co-located
  // buildings. Pinned until another cluster (or empty space) is
  // clicked, so the popup doesn't vanish mid-click.
  const [expandedKey, setExpandedKey] = React.useState<string | null>(null);

  const clusters = React.useMemo(() => buildClusters(markers), [markers]);
  const expanded = expandedKey
    ? clusters.find((c) => c.key === expandedKey) ?? null
    : null;

  // Fit roughly to Australia east coast where the dump sites live.
  const initialViewState = React.useMemo(
    () => ({
      longitude: 148.0,
      latitude: -33.0,
      zoom: 4.0,
    }),
    [],
  );

  // Carto's free vector basemaps. Swap dark-matter / positron to
  // match the host theme so the map blends into the surrounding
  // glass panels instead of always looking like a black hole on
  // light themes (or a blinding white slab on dark).
  //
  // We read the mode from `document.documentElement` directly
  // because the SDK's `useHostTheme()` falls back to "light" when
  // the host doesn't pass `theme` to `<ExtensionSlot>` (rubix-
  // frontend currently doesn't), and the host toggles a `.dark`
  // class on `<html>` exactly when dark mode is active.
  const themeMode = useHostThemeMode();
  const mapStyle = themeMode === "light"
    ? "https://basemaps.cartocdn.com/gl/positron-gl-style/style.json"
    : "https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json";

  const maxCluster = Math.max(1, ...clusters.map((c) => c.totalValue));

  return (
    <div
      style={{ height }}
      className={
        "relative w-full overflow-hidden rounded-xl border " +
        (themeMode === "light" ? "border-slate-200" : "border-white/10")
      }
    >
      <MapGL
        initialViewState={initialViewState}
        mapStyle={mapStyle}
        attributionControl={false}
        style={{ width: "100%", height: "100%" }}
        onClick={() => setExpandedKey(null)}
      >
        <NavigationControl position="top-right" showCompass={false} />

        {clusters.map((c) => {
          const share = maxCluster > 0 ? c.totalValue / maxCluster : 0;
          // Bubble disc grows with cluster usage share; min 16, max 52.
          const size = 16 + Math.round(share * 36);
          const isMulti = c.members.length > 1;
          return (
            <Marker
              key={c.key}
              longitude={c.lon}
              latitude={c.lat}
              anchor="center"
              onClick={(e) => {
                e.originalEvent.stopPropagation();
                if (isMulti) {
                  // Cluster: toggle expansion (fan-out). Single click
                  // doesn't change selection \u2014 user picks via fan.
                  setExpandedKey((k) => (k === c.key ? null : c.key));
                } else {
                  onToggleSite(c.members[0]!.site.host_uuid);
                }
              }}
            >
              <button
                type="button"
                onMouseEnter={() => setHovered(c.key)}
                onMouseLeave={() => setHovered(null)}
                aria-label={
                  isMulti
                    ? `${c.locality} \u2014 ${c.members.length} sites, ${c.totalValue.toFixed(1)}${unit ? ` ${unit}` : ""}`
                    : `${c.members[0]!.site.label} \u2014 ${c.totalValue.toFixed(1)}${unit ? ` ${unit}` : ""}`
                }
                className="relative flex items-center justify-center cursor-pointer outline-none"
                style={{ width: size, height: size }}
              >
                {/* Outer pulse (animation defined in app.css). */}
                <span
                  className={
                    "absolute inset-0 rounded-full " +
                    (c.anySelected ? "site-pulse--on" : "site-pulse--off")
                  }
                  style={{
                    background:
                      "radial-gradient(circle at center, rgba(45,212,191,0.55) 0%, rgba(45,212,191,0) 65%)",
                  }}
                />
                {/* Inner dot */}
                <span
                  className={
                    "relative rounded-full ring-2 transition-all " +
                    (c.allSelected
                      ? "bg-teal-300 ring-teal-200 shadow-[0_0_18px_rgba(94,234,212,0.85)]"
                      : c.anySelected
                        ? "bg-teal-300/70 ring-teal-200/70 shadow-[0_0_14px_rgba(94,234,212,0.6)]"
                        : "bg-slate-200/80 ring-white/40 shadow-[0_0_10px_rgba(255,255,255,0.25)]")
                  }
                  style={{
                    width: Math.max(8, size * 0.5),
                    height: Math.max(8, size * 0.5),
                  }}
                />
                {/* Cluster count badge */}
                {isMulti ? (
                  <span
                    className={
                      "absolute -top-1 -right-1 min-w-[1rem] h-4 px-1 " +
                      "flex items-center justify-center rounded-full " +
                      "text-[0.6rem] font-semibold tabular-nums " +
                      "bg-slate-900/80 text-slate-100 ring-1 ring-white/30"
                    }
                    aria-hidden="true"
                  >
                    {c.members.length}
                  </span>
                ) : null}
              </button>
            </Marker>
          );
        })}

        {/* Fan-out: expanded cluster members rendered as small
            offset pins around the cluster centroid. */}
        {expanded && expanded.members.length > 1 ? expanded.members.map((m, i, arr) => {
          const angle = (i / arr.length) * Math.PI * 2 - Math.PI / 2;
          // Offset in degrees \u2014 small enough to keep them near the
          // cluster, large enough to be visually separate at zoom \u22654.
          const r = 0.04;
          const lon = expanded.lon + Math.cos(angle) * r;
          const lat = expanded.lat + Math.sin(angle) * r * 0.7;
          return (
            <Marker
              key={`fan-${m.site.host_uuid}`}
              longitude={lon}
              latitude={lat}
              anchor="center"
              onClick={(e) => {
                e.originalEvent.stopPropagation();
                onToggleSite(m.site.host_uuid);
              }}
            >
              <button
                type="button"
                onMouseEnter={() => setHovered(`fan-${m.site.host_uuid}`)}
                onMouseLeave={() => setHovered(null)}
                aria-label={`${m.site.label} \u2014 ${m.value.toFixed(1)}${unit ? ` ${unit}` : ""}`}
                className={
                  "px-2 py-0.5 rounded-full text-[0.7rem] font-medium cursor-pointer " +
                  "ring-1 transition-colors backdrop-blur-sm " +
                  (m.selected
                    ? "bg-teal-300/90 text-slate-900 ring-teal-200"
                    : "bg-slate-900/80 text-slate-100 ring-white/30 hover:bg-slate-800")
                }
              >
                {m.site.label}
              </button>
            </Marker>
          );
        }) : null}

        {hovered ? (
          (() => {
            // Two hover scopes: a cluster bubble ("key" = locality)
            // or a fan-out pin ("fan-<uuid>").
            if (hovered.startsWith("fan-")) {
              const uuid = hovered.slice(4);
              const m = markers.find((x) => x.site.host_uuid === uuid);
              if (!m) return null;
              return (
                <Popup
                  longitude={m.site.lon}
                  latitude={m.site.lat}
                  anchor="bottom"
                  closeButton={false}
                  closeOnClick={false}
                  offset={20}
                  className="!font-sans"
                >
                  <div className="text-xs text-slate-900">
                    <div className="font-semibold">{m.site.label}</div>
                    <div className="text-slate-500">{m.site.locality}</div>
                    <div className="mt-1 tabular-nums">
                      {m.value.toLocaleString(undefined, { maximumFractionDigits: 1 })}
                      {unit ? ` ${unit}` : ""}
                    </div>
                  </div>
                </Popup>
              );
            }
            const c = clusters.find((x) => x.key === hovered);
            if (!c) return null;
            const isMulti = c.members.length > 1;
            return (
              <Popup
                longitude={c.lon}
                latitude={c.lat}
                anchor="bottom"
                closeButton={false}
                closeOnClick={false}
                offset={20}
                className="!font-sans"
              >
                <div className="text-xs text-slate-900 min-w-[10rem]">
                  <div className="font-semibold">
                    {isMulti ? c.locality : c.members[0]!.site.label}
                  </div>
                  <div className="text-slate-500">
                    {isMulti
                      ? `${c.members.length} sites \u00b7 ${c.locality}`
                      : c.members[0]!.site.locality}
                  </div>
                  <div className="mt-1 tabular-nums">
                    {c.totalValue.toLocaleString(undefined, { maximumFractionDigits: 1 })}
                    {unit ? ` ${unit}` : ""}
                  </div>
                  {isMulti ? (
                    <div className="mt-1 text-[0.65rem] text-slate-500">
                      {expandedKey === c.key ? "click again to collapse" : "click to expand"}
                    </div>
                  ) : null}
                </div>
              </Popup>
            );
          })()
        ) : null}
      </MapGL>
    </div>
  );
}
