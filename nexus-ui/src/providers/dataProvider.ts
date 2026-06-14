import type { DataProvider } from "@refinedev/core";
import { store } from "./store";

// Minimal in-memory / localStorage data provider for the `dashboards` resource.
// Enough surface for Refine's useList / useOne / useCreate / useUpdate / useDelete.
const delay = () => new Promise((r) => setTimeout(r, 120));

export const dataProvider: DataProvider = {
  getApiUrl: () => "memory://nexus",

  getList: async () => {
    await delay();
    const data = store.all();
    return { data: data as any, total: data.length };
  },

  getOne: async ({ id }) => {
    const found = store.get(String(id));
    return { data: (found ?? null) as any };
  },

  getMany: async ({ ids }) => {
    const set = new Set(ids.map(String));
    return { data: store.all().filter((d) => set.has(d.id)) as any };
  },

  create: async ({ variables }) => {
    await delay();
    const created = store.create(variables as any);
    return { data: created as any };
  },

  update: async ({ id, variables }) => {
    await delay();
    const updated = store.update(String(id), variables as any);
    return { data: (updated ?? null) as any };
  },

  deleteOne: async ({ id }) => {
    await delay();
    const prev = store.get(String(id));
    store.remove(String(id));
    return { data: (prev ?? null) as any };
  },
};
