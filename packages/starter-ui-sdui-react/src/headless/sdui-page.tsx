// `<SduiPage>` — the one component a route file mounts. Owns the
// page-state bag, runs `useSduiResolve` keyed by it, opens the
// subscription plan, and dispatches the root node into the registry.

import { useMemo } from "react";
import type { ClientCapabilities, UiResolveResponseOk } from "@nube/starter-ui-ir";
import { IR_VERSION } from "@nube/starter-ui-ir";
import { PageStateProvider } from "./page-state.js";
import { useSduiResolve } from "./hooks/use-resolve.js";
import { useSduiSubscriptions } from "./hooks/use-subscriptions.js";
import { Render } from "./render.js";
import { listRenderers } from "./registry.js";

export interface SduiPageProps {
  /** Logical page name, e.g. `dashboard.overview`. */
  pageRef: string;
  /** Optional binding target (rubix device / driver / etc.). */
  targetRef?: string;
  /** Optional stack scoping (tenant, site, ...). */
  stack?: Record<string, string>;
  /** Seed page-state — re-rendered whenever this object identity changes. */
  initialPageState?: Record<string, unknown>;
  /** Override the auto-built capability payload (test seam). */
  capabilities?: ClientCapabilities;
}

export function SduiPage(props: SduiPageProps) {
  return (
    <PageStateProvider initial={props.initialPageState}>
      <SduiPageInner {...props} />
    </PageStateProvider>
  );
}

function SduiPageInner({
  pageRef,
  targetRef,
  stack,
  capabilities,
}: SduiPageProps) {
  const caps = useMemo<ClientCapabilities>(
    () =>
      capabilities ?? {
        ir_versions: [IR_VERSION],
        custom_renderers: listRenderers(),
      },
    [capabilities],
  );

  const query = useSduiResolve({ pageRef, targetRef, stack, capabilities: caps });
  const data = query.data;
  const ok = data && isOk(data) ? data : undefined;

  // The subscriptions hook is keyed by the resolve queryKey so any
  // subject update invalidates the right query.
  useSduiSubscriptions(ok?.subscriptions, [
    "sdui",
    "resolve",
    {
      page_ref: pageRef,
      target_ref: targetRef,
      stack,
      // The resolve hook keys by the request including page_state —
      // we don't have direct access to the current state here, so we
      // rely on react-query's hashing of the same object identity.
      capabilities: caps,
    },
  ]);

  if (query.isError) {
    return (
      <div data-sdui-state="error" style={{ padding: 16, color: "#c33" }}>
        {query.error?.message ?? "resolve failed"}
      </div>
    );
  }
  if (!data) {
    return (
      <div data-sdui-state="loading" style={{ padding: 16, opacity: 0.6 }}>
        Loading…
      </div>
    );
  }
  if (!ok) {
    const errors = (data as { errors: { location: string; message: string }[] }).errors;
    return (
      <div data-sdui-state="dry-run" style={{ padding: 16 }}>
        <strong>Page is in dry-run state:</strong>
        <ul>
          {errors.map((e, i) => (
            <li key={i}>
              <code>{e.location}</code>: {e.message}
            </li>
          ))}
        </ul>
      </div>
    );
  }

  return <Render node={ok.render.root} />;
}

function isOk(r: unknown): r is UiResolveResponseOk {
  return Boolean(r && typeof r === "object" && "render" in r);
}
