// `sites-geo.ts` — hardcoded coordinates for the 14 Rubix-OS sites.
//
// The warehouse `points` table doesn't carry lat/lon, only host
// names + descriptions. Rather than depend on a server-side
// geocoder, we ship a static lookup derived from the addresses in
// `point_meta_tags(key='siteRef')` (e.g. "1 Arthur Dixon Court -
// Yatala", "15-19 Muir Road Chullora NSW 2190"). Approximate
// (~town centroid) — fine for an overview map; the per-point
// chart pages still use exact identities.
//
// To regenerate after a new dump, run:
//   SELECT host_uuid, MAX(host_name), MAX(host_description)
//   FROM   com_nubeio_rubixos__points GROUP BY 1;
// then map the addresses to lat/lon manually.

export interface SiteGeo {
  host_uuid: string;
  /** Display label — also used as map marker label. */
  label: string;
  /** Geographic identity ("Yatala, QLD"). */
  locality: string;
  lat: number;
  lon: number;
}

/** Approximate lat/lon for the 14 sites in the bundled dump.
 *  All locations are Australian; resolved from the `siteRef`
 *  meta-tags. Coordinates are town-centroid level. */
export const SITES_GEO: ReadonlyArray<SiteGeo> = [
  { host_uuid: "hos_27f715934c074933", label: "1 Arthur Dixon Court",       locality: "Yatala, QLD",        lat: -27.789, lon: 153.198 },
  { host_uuid: "hos_6a63e28b59fe4f74", label: "171-199 Wayne Goss Dr GW1",  locality: "Berrinba, QLD",      lat: -27.667, lon: 153.062 },
  { host_uuid: "hos_5ca88e9f391c4caa", label: "171-199 Wayne Goss Dr GW2",  locality: "Berrinba, QLD",      lat: -27.668, lon: 153.063 },
  { host_uuid: "hos_123c1ca61ce94105", label: "2-8 Beyer Rd",               locality: "Braeside, VIC",      lat: -38.000, lon: 145.128 },
  { host_uuid: "hos_6490ab35cf9d4375", label: "26-35 Beyer",                locality: "Braeside, VIC",      lat: -37.996, lon: 145.131 },
  { host_uuid: "hos_19b96a35b37641fb", label: "56 Canterbury Rd",           locality: "Bayswater Nth, VIC", lat: -37.806, lon: 145.270 },
  { host_uuid: "hos_736dffa9ba4141fe", label: "Andretti Ct",                locality: "Truganina, VIC",     lat: -37.812, lon: 144.740 },
  { host_uuid: "hos_36391128bc314e30", label: "Keysborough RC 1",           locality: "Keysborough, VIC",   lat: -37.992, lon: 145.180 },
  { host_uuid: "hos_84e1fbc188564aa4", label: "Keysborough RC 2",           locality: "Keysborough, VIC",   lat: -37.993, lon: 145.181 },
  { host_uuid: "hos_e62834f51b544c04", label: "RC-1",                       locality: "Chullora, NSW",      lat: -33.890, lon: 151.075 },
  { host_uuid: "hos_e8cece8d72bf4c92", label: "RC-2",                       locality: "Chullora, NSW",      lat: -33.891, lon: 151.076 },
  { host_uuid: "hos_e9baeea659d444a0", label: "Unit 1 Johnston Cres",       locality: "Horsley Park, NSW",  lat: -33.851, lon: 150.857 },
  { host_uuid: "hos_c894cdbaaf134737", label: "Unit 2 Johnston Cres",       locality: "Horsley Park, NSW",  lat: -33.852, lon: 150.858 },
  { host_uuid: "hos_d0b5454c36994ab5", label: "West Park Drive",            locality: "Derrimut, VIC",      lat: -37.781, lon: 144.741 },
];

const BY_UUID: Map<string, SiteGeo> = new Map(SITES_GEO.map((s) => [s.host_uuid, s]));

export function geoForHost(host_uuid: string): SiteGeo | undefined {
  return BY_UUID.get(host_uuid);
}
