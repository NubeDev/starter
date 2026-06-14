import { useCallback, useRef, useState } from "react";
import { useIsMutating, useQueryClient } from "@tanstack/react-query";

import { dashboardKey } from "@/features/dashboards/useDashboard";

/** The visible state of the explicit Save action. */
export type SaveState = "idle" | "saving" | "saved";

/**
 * Backs the toolbar's explicit **Save** button.
 *
 * The dashboard already autosaves: every panel edit (`useUpdatePanel`) and
 * layout move (`useSaveLayout`) PATCHes immediately, so there is no unsaved
 * client-side draft to flush. What an explicit Save *does* give the user is
 * reassurance and a definite sync point — so this action:
 *
 *  1. waits for any in-flight panel/layout mutations to settle (so "Saved"
 *     never shows while a PATCH is still on the wire), then
 *  2. refetches the dashboard so the canvas reflects exactly what the server
 *     persisted, then
 *  3. shows a transient "Saved" confirmation.
 *
 * It is intentionally idempotent and safe to click at any time.
 */
export function useSaveDashboard(slug: string) {
  const queryClient = useQueryClient();
  // Count of in-flight mutations across the app; panel/layout saves bump this.
  const mutating = useIsMutating();
  const [state, setState] = useState<SaveState>("idle");
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const save = useCallback(async () => {
    if (savedTimer.current) clearTimeout(savedTimer.current);
    setState("saving");
    // Settle pending autosaves before we declare success. We poll the mutation
    // count rather than hold mutation references so this stays decoupled from
    // which hooks are writing — a moved panel and an edited title both count.
    await waitForMutationsToSettle(queryClient);
    await queryClient.invalidateQueries({ queryKey: dashboardKey(slug) });
    setState("saved");
    savedTimer.current = setTimeout(() => setState("idle"), 1800);
  }, [queryClient, slug]);

  return { save, state, isBusy: state === "saving" || mutating > 0 };
}

/** Resolve once no mutations are in flight (or after a safety timeout, so a
 *  stuck request never wedges the button). */
function waitForMutationsToSettle(
  queryClient: ReturnType<typeof useQueryClient>,
): Promise<void> {
  return new Promise((resolve) => {
    if (queryClient.isMutating() === 0) {
      resolve();
      return;
    }
    const deadline = Date.now() + 5000;
    const unsubscribe = queryClient.getMutationCache().subscribe(() => {
      if (queryClient.isMutating() === 0 || Date.now() > deadline) {
        unsubscribe();
        resolve();
      }
    });
  });
}
