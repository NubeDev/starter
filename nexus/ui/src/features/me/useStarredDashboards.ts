import { useCallback, useMemo } from "react";

import { useSaveUserSettings, useSettingsBag } from "@/features/me/useUserSettings";

// The settings-bag key under which a user's starred dashboard ids live. One
// array of dashboard ids; the star is per-user (each caller's own bag) and
// tenant-scoped (the bag is stored per tenant). See `useUserSettings`.
const STARRED_KEY = "starredDashboards";

function readStarred(bag: Record<string, unknown>): string[] {
  const raw = bag[STARRED_KEY];
  return Array.isArray(raw) ? raw.filter((x): x is string => typeof x === "string") : [];
}

// A user's starred dashboards, derived from the settings bag. Returns the id
// set (for membership tests), an `isStarred` predicate, and a `toggle` that
// read-modify-writes the whole bag through `useSaveUserSettings` (optimistic).
export function useStarredDashboards() {
  const bag = useSettingsBag();
  const save = useSaveUserSettings();

  const starredIds = useMemo(() => new Set(readStarred(bag)), [bag]);

  const isStarred = useCallback(
    (dashboardId: string) => starredIds.has(dashboardId),
    [starredIds],
  );

  const toggle = useCallback(
    (dashboardId: string) => {
      const current = readStarred(bag);
      const next = current.includes(dashboardId)
        ? current.filter((id) => id !== dashboardId)
        : [...current, dashboardId];
      // Preserve every other key in the bag — the star feature owns only its
      // own slice, not the whole settings object.
      save.mutate({ ...bag, [STARRED_KEY]: next });
    },
    [bag, save],
  );

  return { starredIds, isStarred, toggle, isSaving: save.isPending };
}
