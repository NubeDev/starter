// localStorage-backed page store for the flow-agent Page Builder
// slice. v0.1 keeps every saved page in one `flow-agent:pages` key as
// `{ id, name, tree, createdAt }`; `tree` is the verbatim
// `UiComponentTree` from the wire format so Edit ⇄ View round-trips
// are lossless (SCOPE R6).
//
// Per SCOPE D1 / R2 the sidebar updates live via
// `useSyncExternalStore`. Pure `localStorage` reads do not trigger
// React re-renders, so this module ships its own pub/sub:
//
//   - in-process `Set<Listener>` for same-tab notifications
//     (browsers don't fire `storage` for the tab that wrote)
//   - `window.storage` listener for cross-tab sync
//
// Both sources call the same `emit()`; the React hook subscribes to
// the union.

import { useSyncExternalStore } from "react";
import type { UiComponentTree } from "@nube/starter-sdui-react";

export const PAGES_STORAGE_KEY = "flow-agent:pages";

export interface PageRecord {
  id: string;
  name: string;
  tree: UiComponentTree;
  createdAt: number;
  updatedAt: number;
}

type Listener = () => void;
const listeners = new Set<Listener>();

function emit(): void {
  for (const l of listeners) l();
}

export function subscribe(l: Listener): () => void {
  listeners.add(l);
  return () => {
    listeners.delete(l);
  };
}

// Wire the cross-tab listener exactly once on module load. The guard
// keeps SSR / vitest jsdom-less environments happy.
if (typeof window !== "undefined") {
  window.addEventListener("storage", (e) => {
    if (e.key === PAGES_STORAGE_KEY || e.key === null) {
      // key === null fires on `localStorage.clear()` per the spec.
      cachedSnapshot = null;
      emit();
    }
  });
}

// `useSyncExternalStore` requires a *stable* snapshot — returning a
// fresh array every read would tear-loop React. Cache the parsed
// array and invalidate on any write or cross-tab event.
let cachedSnapshot: PageRecord[] | null = null;

function readRaw(): PageRecord[] {
  if (typeof window === "undefined") return [];
  const raw = window.localStorage.getItem(PAGES_STORAGE_KEY);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    // Light shape filter — drops any record missing required fields
    // rather than throwing, so a hand-edited key in DevTools doesn't
    // brick the sidebar.
    return parsed.filter(
      (r): r is PageRecord =>
        !!r &&
        typeof r === "object" &&
        typeof (r as PageRecord).id === "string" &&
        typeof (r as PageRecord).name === "string" &&
        typeof (r as PageRecord).createdAt === "number" &&
        typeof (r as PageRecord).updatedAt === "number" &&
        !!(r as PageRecord).tree &&
        typeof (r as PageRecord).tree === "object",
    );
  } catch {
    return [];
  }
}

function getSnapshot(): PageRecord[] {
  if (cachedSnapshot === null) cachedSnapshot = readRaw();
  return cachedSnapshot;
}

function writeAll(records: PageRecord[]): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(PAGES_STORAGE_KEY, JSON.stringify(records));
  cachedSnapshot = records;
  emit();
}

export function listPages(): PageRecord[] {
  return getSnapshot();
}

export function getPage(id: string): PageRecord | undefined {
  return getSnapshot().find((p) => p.id === id);
}

export interface SavePageInput {
  id?: string;
  name: string;
  tree: UiComponentTree;
}

export function savePage(input: SavePageInput): PageRecord {
  const now = Date.now();
  const all = [...getSnapshot()];
  if (input.id) {
    const idx = all.findIndex((p) => p.id === input.id);
    const existing = idx >= 0 ? all[idx] : undefined;
    if (existing) {
      const next: PageRecord = {
        ...existing,
        name: input.name,
        tree: input.tree,
        updatedAt: now,
      };
      all[idx] = next;
      writeAll(all);
      return next;
    }
  }
  const next: PageRecord = {
    id: input.id ?? makePageId(),
    name: input.name,
    tree: input.tree,
    createdAt: now,
    updatedAt: now,
  };
  all.unshift(next);
  writeAll(all);
  return next;
}

export function deletePage(id: string): void {
  const all = getSnapshot().filter((p) => p.id !== id);
  writeAll(all);
}

function makePageId(): string {
  // crypto.randomUUID is available in all browsers we target; fall
  // back to a millis+random suffix for the rare jsdom run without it.
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `page-${Date.now().toString(36)}-${Math.random()
    .toString(36)
    .slice(2, 8)}`;
}

/**
 * React hook returning the current list of saved pages. Re-renders
 * on any save/delete in this tab and on any `storage` event from
 * other tabs.
 */
export function usePages(): PageRecord[] {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
