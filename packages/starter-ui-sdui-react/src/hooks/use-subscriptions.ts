// `useSduiSubscriptions` — opens `transport.subscribe()` for the
// subscription plan returned by the latest resolve. On any subject
// update we invalidate the resolve query so the IR refetches.

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { SubscriptionPlan } from "@nube/starter-ui-ir";
import { useSduiTransport } from "../provider/sdui-provider.js";

export function useSduiSubscriptions(
  plan: SubscriptionPlan | undefined,
  resolveQueryKey: readonly unknown[],
): void {
  const transport = useSduiTransport();
  const qc = useQueryClient();
  const subjects = plan?.subjects ?? [];
  // Stable key for the subjects array so re-renders without subject
  // changes don't restart the subscription. We hash by `key`.
  const subjectsKey = subjects.map((s) => s.key).join("|");

  useEffect(() => {
    if (subjects.length === 0) return;
    const off = transport.subscribe(subjects, () => {
      void qc.invalidateQueries({ queryKey: resolveQueryKey });
    });
    return () => off();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [transport, qc, subjectsKey, JSON.stringify(resolveQueryKey)]);
}
