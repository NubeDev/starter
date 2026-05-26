// PuckBuilder — mounts `<Puck>` with the schema-derived Config and
// adds the §B4 save UX: Save button, conflict modal, error banner.
//
// Two ways to seed the canvas:
//   - `initialTree` (preferred) — an IR `ComponentTree` as it comes
//     out of `rubix.dashboard.get`. The adapter converts it into
//     the Puck `Data` shape on mount and back into IR on save.
//   - `initialData` — raw Puck `Data` for the harness / tests that
//     want to skip the adapter.
//
// Save is opt-in: if `onSave` is omitted (PR1 harness) the Save
// button is hidden and the legacy `window.__rubixPuckLastChange`
// drop is kept for backwards-compat with the harness page.

import { Puck, type Config, type Data } from "@measured/puck";
// Puck ships an unstyled editor — without this CSS the canvas
// renders as an essentially-invisible flex layout. Importing here
// (rather than asking consumers to do it) keeps the package
// self-contained; vite / webpack will dedupe a duplicate import.
import "@measured/puck/puck.css";
import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type ReactElement,
} from "react";

import { PageStateProvider } from "@nube/starter-ui-sdui-react/headless";

import {
  componentTreeToPuckData,
  puckDataToComponentTree,
  type ComponentTree,
} from "./adapter.js";
import { buildPuckConfig } from "./build-puck-config.js";
import { CatalogueProvider, type Catalogue } from "./data-source-field.js";
import { IR_SCHEMA } from "./schema-loader.js";
import type { PuckConfigStub } from "./puck-types.js";
import type { PuckSaveOutcome, PuckSaveTransport } from "./save.js";

export interface PuckBuilderProps {
  /** Page identifier — `"dashboard.<slug>"`. Echoed in `onSave`. */
  pageRef: string;
  /** IR `ComponentTree` from `rubix.dashboard.get`. Preferred. */
  initialTree?: ComponentTree;
  /** Raw Puck `Data`. Bypasses the IR adapter — harness / tests
   *  only. Mutually exclusive with `initialTree`; `initialTree`
   *  wins when both are supplied. */
  initialData?: Data;
  /** Initial optimistic-concurrency token. Captured from
   *  `rubix.dashboard.get`. Updated in place on every successful
   *  save so subsequent saves guard against the new revision. */
  initialRevisionId?: string;
  /** Optional override of the schema-derived Puck config. */
  config?: Config;
  /** Consumer-supplied save transport. When omitted the Save
   *  button is hidden. */
  onSave?: PuckSaveTransport;
  /** §B3 catalogue seam. Wraps the canvas in a `CatalogueProvider`
   *  so the schema-derived data-source pickers
   *  (analytics templates, tool refs, tenants, unit symbols,
   *  page-state keys) can load their options. Without one, each
   *  picker degrades to a free-text input with an inline warning. */
  catalogue?: Catalogue;
}

type SaveState =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved"; revisionId: string }
  | { kind: "error"; message: string };

interface ConflictState {
  open: boolean;
  currentRevisionId: string;
}

export function PuckBuilder({
  pageRef,
  initialTree,
  initialData,
  initialRevisionId,
  config,
  onSave,
  catalogue,
}: PuckBuilderProps): ReactElement {
  const resolvedConfig = useMemo<Config>(() => {
    if (config) return config;
    return buildPuckConfig({ schema: IR_SCHEMA }) as unknown as Config;
  }, [config]);

  // Compute the starting Puck Data exactly once. Updates to
  // `initialTree` after mount are intentionally ignored — the
  // canvas owns the state once the operator starts editing.
  const initialPuckData = useMemo<Data>(() => {
    if (initialTree) return componentTreeToPuckData(initialTree);
    return initialData ?? ({ content: [], root: { props: {} } } as Data);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The live Puck Data lives in a ref so the Save handler can read
  // it without re-rendering on every keystroke. `<Puck>` is an
  // uncontrolled component and pushes changes through `onChange`.
  const dataRef = useRef<Data>(initialPuckData);
  const revisionRef = useRef<string | undefined>(initialRevisionId);

  const [saveState, setSaveState] = useState<SaveState>({ kind: "idle" });
  const [conflict, setConflict] = useState<ConflictState>({
    open: false,
    currentRevisionId: "",
  });

  const handleSave = useCallback(async () => {
    if (!onSave) return;
    setSaveState({ kind: "saving" });
    const body = puckDataToComponentTree(dataRef.current);
    const titleProp = body.root.title;
    const tagsProp = body.root.tags;
    let outcome: PuckSaveOutcome;
    try {
      outcome = await onSave({
        pageRef,
        body,
        expectedRevisionId: revisionRef.current,
        title: typeof titleProp === "string" ? titleProp : undefined,
        tags: Array.isArray(tagsProp) ? (tagsProp as string[]) : undefined,
      });
    } catch (e) {
      const err = e as { message?: string };
      outcome = { kind: "error", message: err.message ?? String(e) };
    }
    switch (outcome.kind) {
      case "saved":
        revisionRef.current = outcome.revisionId;
        setSaveState({ kind: "saved", revisionId: outcome.revisionId });
        break;
      case "conflict":
        setConflict({
          open: true,
          currentRevisionId: outcome.currentRevisionId,
        });
        setSaveState({ kind: "idle" });
        break;
      case "error":
        setSaveState({ kind: "error", message: outcome.message });
        break;
    }
  }, [onSave, pageRef]);

  const keepEditing = useCallback(() => {
    // Adopt the server's current revision id so the next save
    // immediately 409s again until the operator either copies
    // their work out or hits Discard. This matches the scope:
    // "stay on the in-editor tree, see a persistent warning
    // banner, and accept that the next Save will 409 again".
    revisionRef.current = conflict.currentRevisionId;
    setConflict({ open: false, currentRevisionId: "" });
  }, [conflict.currentRevisionId]);

  const discardEdits = useCallback(() => {
    // No-op on the data — the route loader is responsible for
    // re-fetching the live revision and re-mounting <PuckBuilder>
    // with the fresh `initialTree`. We just close the modal and
    // drop a window flag the route can subscribe to.
    setConflict({ open: false, currentRevisionId: "" });
    if (typeof window !== "undefined") {
      (window as unknown as Record<string, unknown>).__rubixPuckDiscardRequested =
        { pageRef, ts: Date.now() };
    }
  }, [pageRef]);

  return (
    <div
      data-puck-builder={pageRef}
      style={{ display: "flex", flexDirection: "column", height: "100%" }}
    >
      {onSave ? (
        <div
          data-puck-builder-toolbar=""
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.75rem",
            padding: "0.5rem 0.75rem",
            borderBottom: "1px solid #e5e7eb",
            background: "#f8fafc",
            fontFamily: "ui-sans-serif, system-ui",
            fontSize: "0.875rem",
          }}
        >
          <button
            type="button"
            onClick={handleSave}
            disabled={saveState.kind === "saving"}
            data-puck-builder-save=""
            style={{
              padding: "0.375rem 0.75rem",
              borderRadius: "0.375rem",
              border: "1px solid #2563eb",
              background: saveState.kind === "saving" ? "#bfdbfe" : "#2563eb",
              color: "white",
              cursor: saveState.kind === "saving" ? "wait" : "pointer",
              fontWeight: 500,
            }}
          >
            {saveState.kind === "saving" ? "Saving…" : "Save"}
          </button>
          {saveState.kind === "saved" ? (
            <span style={{ color: "#16a34a" }} data-puck-builder-save-status="saved">
              Saved — revision <code>{saveState.revisionId.slice(0, 8)}</code>
            </span>
          ) : null}
          {saveState.kind === "error" ? (
            <span style={{ color: "#dc2626" }} data-puck-builder-save-status="error">
              Save failed: {saveState.message}
            </span>
          ) : null}
        </div>
      ) : null}
      <div style={{ flex: 1, minHeight: 0 }}>
        <MaybeCatalogueProvider catalogue={catalogue}>
        <PageStateProvider>
        <Puck
          config={resolvedConfig}
          data={initialPuckData}
          onChange={(next) => {
            dataRef.current = next;
            if (typeof window !== "undefined") {
              (window as unknown as Record<string, unknown>).__rubixPuckLastChange =
                {
                  pageRef,
                  data: next,
                  ts: Date.now(),
                };
            }
          }}
        />
        </PageStateProvider>
        </MaybeCatalogueProvider>
      </div>
      {conflict.open ? (
        <ConflictModal
          currentRevisionId={conflict.currentRevisionId}
          onDiscard={discardEdits}
          onKeepEditing={keepEditing}
        />
      ) : null}
    </div>
  );
}

function ConflictModal({
  currentRevisionId,
  onDiscard,
  onKeepEditing,
}: {
  currentRevisionId: string;
  onDiscard: () => void;
  onKeepEditing: () => void;
}): ReactElement {
  return (
    <div
      data-puck-builder-conflict-modal=""
      role="dialog"
      aria-modal="true"
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(15, 23, 42, 0.5)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9999,
        fontFamily: "ui-sans-serif, system-ui",
      }}
    >
      <div
        style={{
          background: "white",
          padding: "1.25rem 1.5rem",
          borderRadius: "0.5rem",
          maxWidth: "32rem",
          boxShadow: "0 10px 25px rgba(0, 0, 0, 0.2)",
        }}
      >
        <h2 style={{ margin: 0, marginBottom: "0.5rem", fontSize: "1.125rem" }}>
          This page was edited elsewhere
        </h2>
        <p style={{ margin: 0, marginBottom: "1rem", color: "#475569" }}>
          Someone else (or the AI assistant) saved a new revision
          (<code>{currentRevisionId.slice(0, 8)}</code>) since you started
          editing. You can either reload the server's revision and lose
          your in-editor changes, or keep editing — every subsequent
          Save will fail with the same conflict until you reload.
        </p>
        <div style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end" }}>
          <button
            type="button"
            onClick={onKeepEditing}
            data-puck-builder-conflict-keep=""
            style={{
              padding: "0.375rem 0.75rem",
              borderRadius: "0.375rem",
              border: "1px solid #cbd5e1",
              background: "white",
              cursor: "pointer",
            }}
          >
            Keep editing
          </button>
          <button
            type="button"
            onClick={onDiscard}
            data-puck-builder-conflict-discard=""
            style={{
              padding: "0.375rem 0.75rem",
              borderRadius: "0.375rem",
              border: "1px solid #dc2626",
              background: "#dc2626",
              color: "white",
              cursor: "pointer",
            }}
          >
            Discard my edits
          </button>
        </div>
      </div>
    </div>
  );
}

function MaybeCatalogueProvider({
  catalogue,
  children,
}: {
  catalogue: Catalogue | undefined;
  children: ReactElement;
}): ReactElement {
  if (!catalogue) return children;
  return <CatalogueProvider catalogue={catalogue}>{children}</CatalogueProvider>;
}

export type { PuckConfigStub };
