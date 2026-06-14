import type { Dashboard, Widget } from "@/data/types";
import { SEED_DASHBOARDS } from "@/data/seed";

const KEY = "nexus.dashboards.v1";

function load(): Dashboard[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return JSON.parse(raw) as Dashboard[];
  } catch {
    /* ignore */
  }
  const seeded = structuredClone(SEED_DASHBOARDS);
  localStorage.setItem(KEY, JSON.stringify(seeded));
  return seeded;
}

let cache: Dashboard[] = load();
const listeners = new Set<() => void>();

function persist() {
  localStorage.setItem(KEY, JSON.stringify(cache));
  listeners.forEach((l) => l());
}

export const store = {
  subscribe(fn: () => void) {
    listeners.add(fn);
    return () => listeners.delete(fn);
  },
  all(): Dashboard[] {
    return cache;
  },
  get(id: string): Dashboard | undefined {
    return cache.find((d) => d.id === id);
  },
  getBySlug(slug: string): Dashboard | undefined {
    return cache.find((d) => d.slug === slug);
  },
  create(input: Partial<Dashboard>): Dashboard {
    const base = (input.name ?? "Untitled Dashboard").trim();
    const slug = slugify(base, cache);
    const d: Dashboard = {
      id: slug,
      name: base,
      slug,
      description: input.description ?? "",
      icon: input.icon ?? "Activity",
      accent: input.accent ?? "152 76% 44%",
      starred: false,
      widgets: input.widgets ?? [],
      updatedAt: new Date().toISOString(),
    };
    cache = [d, ...cache];
    persist();
    return d;
  },
  update(id: string, patch: Partial<Dashboard>): Dashboard | undefined {
    const idx = cache.findIndex((d) => d.id === id);
    if (idx < 0) return undefined;
    cache[idx] = { ...cache[idx], ...patch, updatedAt: new Date().toISOString() };
    cache = [...cache];
    persist();
    return cache[idx];
  },
  setWidgets(id: string, widgets: Widget[]) {
    return this.update(id, { widgets });
  },
  remove(id: string) {
    cache = cache.filter((d) => d.id !== id);
    persist();
  },
  reset() {
    cache = structuredClone(SEED_DASHBOARDS);
    persist();
  },
};

function slugify(name: string, existing: Dashboard[]): string {
  const base =
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/(^-|-$)/g, "") || "dashboard";
  let slug = base;
  let i = 2;
  const taken = new Set(existing.map((d) => d.slug));
  while (taken.has(slug)) slug = `${base}-${i++}`;
  return slug;
}
