import { useEffect, useRef } from "react";
import { useSearchParams } from "react-router-dom";

import { useVariableStore } from "@/store/variables";
import {
  parseVariableParams,
  writeVariableParams,
} from "@/features/variables/urlState";

// Two-way bind variable selections to the URL `?var-<name>=…` params so a
// dashboard link carries its picked values and survives a reload (item 8).
// On mount it restores any `var-*` into the store; thereafter it writes
// selection changes back (replacing history, not pushing — a variable tweak
// shouldn't spam the back button). A mount-once guard separates "restore"
// from "write" so the initial restore doesn't immediately rewrite itself.
export function useVariableUrlSync(): void {
  const [searchParams, setSearchParams] = useSearchParams();
  const selections = useVariableStore((s) => s.selections);
  const applySelections = useVariableStore((s) => s.applySelections);
  const restored = useRef(false);

  // Restore once on mount: the URL is authoritative for a freshly opened
  // link. Resolution (in `useDashboardVariables`) treats these as overrides.
  useEffect(() => {
    if (restored.current) return;
    restored.current = true;
    const parsed = parseVariableParams(searchParams);
    if (Object.keys(parsed).length > 0) applySelections(parsed);
    // Mount-only: later URL edits come from this hook's own writes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Write store -> URL after restore. Only touch the params if they'd
  // change, to avoid a redundant history replace on every render.
  useEffect(() => {
    if (!restored.current) return;
    const plain: Record<string, string[]> = {};
    for (const [name, values] of Object.entries(selections)) {
      if (values.length > 0) plain[name] = [...values];
    }
    const next = writeVariableParams(searchParams, plain);
    if (next.toString() !== searchParams.toString()) {
      setSearchParams(next, { replace: true });
    }
  }, [selections, searchParams, setSearchParams]);
}
