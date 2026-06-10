import { useStarterClient } from "@nube/starter-client-react";

import { useVariableStore } from "@/store/variables";
import { useDashboard } from "@/features/dashboards/useDashboard";
import { VariableControl } from "@/features/variables/VariableControl";
import { useDashboardVariables } from "@/features/variables/useDashboardVariables";
import { usePageContext } from "@/features/variables/usePageContext";
import { useVariableUrlSync } from "@/features/variables/useVariableUrlSync";
import { updateVariable } from "@/api/variables/update";

// The variable bar above the canvas (item 4): one control per non-hidden
// variable, mounted on a dashboard to drive its panels. Loading, resolving
// (incl. cascading), URL sync, and cycle reporting are delegated to the
// hooks; this component is the visible bar plus the selection-change
// handler that (a) updates the in-memory store so panels re-query at once,
// and (b) persists the new `current` so the pick survives a reload even
// without a URL param.
//
// Mounting nothing when a dashboard has no variables keeps a plain
// dashboard visually unchanged (backwards-compatible).
export function VariableBar({ slug }: { slug: string }) {
  const client = useStarterClient();
  // Assemble the page context (WS-13 §1) — the nav node from `?nav=`, the bare
  // URL params, and this dashboard's tags — and thread it into resolution so a
  // `context` variable resolves and navigating between two mounts of one page
  // re-resolves + re-queries (§5). The dashboard id (for its tags) comes from
  // the already-cached dashboard query, so this adds no extra round-trip.
  const { data: dashboard } = useDashboard(slug);
  const pageContext = usePageContext(slug, dashboard?.id);
  const { cycle } = useDashboardVariables(slug, pageContext);
  useVariableUrlSync();

  const resolved = useVariableStore((s) => s.resolved);
  const setSelection = useVariableStore((s) => s.setSelection);

  // A cycle is an authoring error: show it plainly rather than loop. The
  // bar still renders whatever resolved before the cycle was hit (empty on
  // a hard cycle), so the page stays usable.
  if (cycle) {
    return (
      <div
        role="alert"
        className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
      >
        {cycle.message}
      </div>
    );
  }

  const visible = resolved.filter((v) => !v.hidden);
  if (visible.length === 0) return null;

  function onChange(name: string, id: string, values: string[]) {
    // Update the store first so panels re-query immediately; persist in the
    // background. A failed persist doesn't block the live selection — the
    // value is also reflected in the URL, so the session stays consistent.
    setSelection(name, values);
    void updateVariable(client, id, { current: values }).catch(() => {
      /* best-effort persist; URL + store keep the live selection */
    });
  }

  return (
    <div className="flex flex-wrap items-end gap-3" data-testid="variable-bar">
      {visible.map((v) => (
        <VariableControl
          key={v.id}
          variable={v}
          onChange={(values) => onChange(v.name, v.id, values)}
        />
      ))}
    </div>
  );
}
