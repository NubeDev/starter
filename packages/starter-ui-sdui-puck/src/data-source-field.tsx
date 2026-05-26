// §B3 catalogue-backed Puck fields.
//
// The package itself is rubix-agnostic — the consumer (rubix-frontend
// edit route, or the harness) provides a `Catalogue` via the
// `CatalogueProvider` context. Each picker fires its lookup on mount,
// caches the result, and on individual failure **degrades to a free
// text input with an inline warning** per scope §B3 "Fetch lifecycle".
//
// No live picker refresh in v1 — the cache is per editor session.

import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactElement,
  type ReactNode,
} from "react";

import type { CatalogueKind } from "./curation/data-sources.js";

/** One option in a catalogue dropdown. */
export interface CatalogueEntry {
  /** Wire value written to the IR field. */
  value: string;
  /** Human label rendered in the dropdown. */
  label: string;
  /** Optional secondary line — e.g. tool description. */
  hint?: string;
}

/** Consumer-supplied catalogue. One async lookup per `CatalogueKind`.
 *  Each method must reject on failure so the picker can degrade. A
 *  missing method is treated as "this host can't satisfy that kind"
 *  and the picker shows the fallback text input. */
export interface Catalogue {
  list(kind: CatalogueKind): Promise<readonly CatalogueEntry[]>;
}

/** Convenience helper for hosts that want a per-kind function map. */
export function catalogueFromMap(
  map: Partial<Record<CatalogueKind, () => Promise<readonly CatalogueEntry[]>>>,
): Catalogue {
  return {
    async list(kind) {
      const fn = map[kind];
      if (!fn) throw new Error(`catalogue: no provider for "${kind}"`);
      return fn();
    },
  };
}

const CatalogueCtx = createContext<Catalogue | undefined>(undefined);

export interface CatalogueProviderProps {
  catalogue: Catalogue;
  children: ReactNode;
}

export function CatalogueProvider({
  catalogue,
  children,
}: CatalogueProviderProps): ReactElement {
  return (
    <CatalogueCtx.Provider value={catalogue}>{children}</CatalogueCtx.Provider>
  );
}

export function useCatalogue(): Catalogue | undefined {
  return useContext(CatalogueCtx);
}

/** Special sentinel — when ChartSource arrives as an object the
 *  analytics-template picker writes back the whole `{ type:
 *  "analytics_template", name: <pick>, map: {…} }` shape. For now we
 *  just write back the picked `name` and the caller is expected to
 *  hold an analytics_template-shaped source object. The full union
 *  picker (other ChartSource arms) lands in B3 PR2. */
const ANALYTICS_TEMPLATE_SHAPE = "analytics_template";

interface DataSourceFieldProps {
  kind: CatalogueKind;
  name: string;
  value: unknown;
  onChange: (next: unknown) => void;
}

/**
 * The actual picker component. Used as the `render` payload of a
 * Puck `custom` field — see `makeDataSourceField` below.
 */
export function DataSourceField({
  kind,
  name,
  value,
  onChange,
}: DataSourceFieldProps): ReactElement {
  const catalogue = useCatalogue();
  const [state, setState] = useState<
    | { kind: "loading" }
    | { kind: "ready"; entries: readonly CatalogueEntry[] }
    | { kind: "error"; message: string }
    | { kind: "no_provider" }
  >({ kind: "loading" });

  // The catalogue ref never changes per editor session in v1; use a
  // ref so the effect doesn't re-fire on parent re-renders.
  const catRef = useRef(catalogue);
  catRef.current = catalogue;

  useEffect(() => {
    let cancelled = false;
    if (!catRef.current) {
      setState({ kind: "no_provider" });
      return;
    }
    catRef.current
      .list(kind)
      .then((entries) => {
        if (!cancelled) setState({ kind: "ready", entries });
      })
      .catch((e: { message?: string }) => {
        if (!cancelled)
          setState({ kind: "error", message: e?.message ?? String(e) });
      });
    return () => {
      cancelled = true;
    };
  }, [kind]);

  // Pull the wire value out of whatever shape the IR uses for this
  // kind. For analytics_template the field carries a ChartSource
  // object — we read/write `.name` on the analytics_template arm.
  const currentValue = readWireValue(kind, value);
  const wrap = (picked: string) => onChange(writeWireValue(kind, value, picked));

  if (state.kind === "loading") {
    return (
      <FieldShell label={name}>
        <input
          type="text"
          defaultValue={currentValue}
          onChange={(e) => wrap(e.currentTarget.value)}
          placeholder="loading catalogue…"
          style={inputStyle}
        />
      </FieldShell>
    );
  }

  if (state.kind === "error" || state.kind === "no_provider") {
    const msg =
      state.kind === "error"
        ? `couldn't load ${kind} list — typing the name still works (${state.message})`
        : `no catalogue provider for ${kind} — typing the name still works`;
    return (
      <FieldShell label={name}>
        <input
          type="text"
          value={currentValue}
          onChange={(e) => wrap(e.currentTarget.value)}
          style={inputStyle}
        />
        <small style={warnStyle}>{msg}</small>
      </FieldShell>
    );
  }

  return (
    <FieldShell label={name}>
      <select
        value={currentValue}
        onChange={(e) => wrap(e.currentTarget.value)}
        style={inputStyle}
      >
        <option value="">— pick —</option>
        {state.entries.map((entry) => (
          <option key={entry.value} value={entry.value}>
            {entry.label}
          </option>
        ))}
      </select>
    </FieldShell>
  );
}

function readWireValue(kind: CatalogueKind, value: unknown): string {
  if (kind === "analytics_template") {
    if (value && typeof value === "object" && "name" in value) {
      const n = (value as { name?: unknown }).name;
      return typeof n === "string" ? n : "";
    }
    return "";
  }
  if (typeof value === "string") return value;
  return "";
}

function writeWireValue(
  kind: CatalogueKind,
  prev: unknown,
  picked: string,
): unknown {
  if (kind === "analytics_template") {
    const base =
      prev && typeof prev === "object"
        ? (prev as Record<string, unknown>)
        : {};
    return { ...base, type: ANALYTICS_TEMPLATE_SHAPE, name: picked };
  }
  return picked;
}

function FieldShell({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}): ReactElement {
  return (
    <label
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "0.25rem",
        fontSize: "0.8125rem",
      }}
    >
      <span style={{ color: "#475569" }}>{label}</span>
      {children}
    </label>
  );
}

const inputStyle: CSSProperties = {
  padding: "0.375rem 0.5rem",
  border: "1px solid #cbd5e1",
  borderRadius: "0.25rem",
  fontSize: "0.8125rem",
  background: "white",
};

const warnStyle: CSSProperties = {
  color: "#b45309",
  fontSize: "0.75rem",
};

/**
 * Build a Puck `custom` field bound to one catalogue kind. The
 * returned object is structurally compatible with
 * `PuckFieldStub.custom`.
 */
export function makeDataSourceField(kind: CatalogueKind): {
  type: "custom";
  render: (props: {
    name: string;
    value: unknown;
    onChange: (v: unknown) => void;
  }) => ReactElement;
  /** Carried as a hint for tests / debugging — Puck ignores extra keys. */
  catalogueKind: CatalogueKind;
} {
  return {
    type: "custom",
    catalogueKind: kind,
    render: ({ name, value, onChange }) => (
      <DataSourceField kind={kind} name={name} value={value} onChange={onChange} />
    ),
  };
}

// Re-export the kind type so consumers don't need a second import.
export type { CatalogueKind } from "./curation/data-sources.js";
