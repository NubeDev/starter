// "View" = the user's current scope (meter kind + time range +
// selected sites) under a name.  Persisted to localStorage so
// users can flip between recurring scopes without rebuilding the
// filter each time.

import * as React from "react";

import { EXTENSION_ID } from "../types";
import { KINDS, RANGES, MAX_SAVED_VIEWS } from "./presets";
import { Field, PillBtn } from "./prims";

export interface SavedView {
  /** Stable id (timestamp ms) — used as React key and for
   *  delete. Display name is `name`. */
  id: string;
  name: string;
  kindIdx: number;
  rangeIdx: number;
  selectedHosts: ReadonlyArray<string>;
}

const SAVED_VIEWS_KEY = `${EXTENSION_ID}.dashboard.savedViews.v1`;

export function loadSavedViews(): ReadonlyArray<SavedView> {
  try {
    const raw = localStorage.getItem(SAVED_VIEWS_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (v): v is SavedView =>
        typeof v === "object" && v !== null
        && typeof (v as SavedView).id === "string"
        && typeof (v as SavedView).name === "string"
        && typeof (v as SavedView).kindIdx === "number"
        && typeof (v as SavedView).rangeIdx === "number"
        && Array.isArray((v as SavedView).selectedHosts),
    );
  } catch {
    return [];
  }
}

export function saveSavedViews(views: ReadonlyArray<SavedView>): void {
  try {
    localStorage.setItem(SAVED_VIEWS_KEY, JSON.stringify(views));
  } catch {
    /* quota / disabled — fail silently */
  }
}

export function SavedViewsField({
  kindIdx, rangeIdx, selectedHosts, allHosts, onApply,
}: {
  kindIdx: number;
  rangeIdx: number;
  selectedHosts: ReadonlyArray<string>;
  allHosts: ReadonlyArray<{ uuid: string; name: string }>;
  onApply: (v: SavedView) => void;
}): React.ReactElement {
  const [views, setViews] = React.useState<ReadonlyArray<SavedView>>(() => loadSavedViews());
  const [naming, setNaming] = React.useState(false);
  const [draft, setDraft] = React.useState("");
  const inputRef = React.useRef<HTMLInputElement | null>(null);

  React.useEffect(() => {
    if (naming) inputRef.current?.focus();
  }, [naming]);

  const persist = (next: ReadonlyArray<SavedView>) => {
    setViews(next);
    saveSavedViews(next);
  };

  const commitSave = () => {
    const name = draft.trim();
    if (!name) { setNaming(false); return; }
    // Drop any same-name entry (rename / overwrite semantics).
    const filtered = views.filter((v) => v.name !== name);
    const next: SavedView = {
      id: String(Date.now()),
      name,
      kindIdx,
      rangeIdx,
      // Persist only hosts that still exist in the catalog, so
      // stale uuids don't accumulate when sites are removed.
      selectedHosts: selectedHosts.filter((u) => allHosts.some((h) => h.uuid === u)),
    };
    persist([next, ...filtered].slice(0, MAX_SAVED_VIEWS));
    setDraft("");
    setNaming(false);
  };

  const remove = (id: string) => persist(views.filter((v) => v.id !== id));

  const kindLabel = KINDS[kindIdx]?.label ?? "?";
  const rangeLabel = RANGES[rangeIdx]?.label ?? "?";
  const defaultName = `${kindLabel} · ${rangeLabel} · ${selectedHosts.length}/${allHosts.length}`;

  return (
    <Field label={`Saved views · ${views.length}/${MAX_SAVED_VIEWS}`}>
      <div className="flex flex-wrap items-center gap-1">
        {naming ? (
          <>
            <input
              ref={inputRef}
              type="text"
              value={draft}
              placeholder={defaultName}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") { e.preventDefault(); commitSave(); }
                if (e.key === "Escape") { setNaming(false); setDraft(""); }
              }}
              className={
                "ext-glass px-2 py-1 text-xs rounded-md w-44 bg-transparent " +
                "focus:outline-none focus-visible:ring-2 focus-visible:ring-primary"
              }
              aria-label="View name"
            />
            <PillBtn onClick={() => { if (!draft.trim()) setDraft(defaultName); commitSave(); }}>
              save
            </PillBtn>
            <PillBtn onClick={() => { setNaming(false); setDraft(""); }}>cancel</PillBtn>
          </>
        ) : (
          <PillBtn onClick={() => { setDraft(""); setNaming(true); }}>+ save current</PillBtn>
        )}
        {views.map((v) => (
          <span
            key={v.id}
            className={
              "inline-flex items-center gap-1 pl-2 pr-1 py-0.5 text-xs " +
              "rounded-full border border-border/40 bg-transparent " +
              "text-muted-foreground hover:bg-accent transition-colors"
            }
          >
            <button
              type="button"
              onClick={() => onApply(v)}
              className="cursor-pointer hover:text-foreground"
              title={`${KINDS[v.kindIdx]?.label ?? "?"} · ${RANGES[v.rangeIdx]?.label ?? "?"} · ${v.selectedHosts.length} sites`}
            >
              {v.name}
            </button>
            <button
              type="button"
              onClick={() => remove(v.id)}
              aria-label={`Delete view ${v.name}`}
              className={
                "inline-flex h-4 w-4 items-center justify-center rounded-full " +
                "text-muted-foreground/60 hover:text-foreground hover:bg-accent cursor-pointer"
              }
            >
              <svg width="8" height="8" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
                <path d="M1.5 1.5 L8.5 8.5 M8.5 1.5 L1.5 8.5" />
              </svg>
            </button>
          </span>
        ))}
      </div>
    </Field>
  );
}
