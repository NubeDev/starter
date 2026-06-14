import { useSyncExternalStore } from "react";
import { store } from "./store";
import type { Dashboard } from "@/data/types";

export function useDashboards(): Dashboard[] {
  return useSyncExternalStore(store.subscribe, store.all);
}

export function useDashboard(slug?: string): Dashboard | undefined {
  const all = useDashboards();
  return slug ? all.find((d) => d.slug === slug) : undefined;
}
