import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { redo, undo } from "@/api/undo/apply";
import type { UndoResponse } from "@/api/types";

// Undo / redo the caller's most recent change group, from `POST /undo|/redo`.
// Undo is per-actor and bodyless. On success we invalidate the whole nexus
// query tree: an undo can touch any registered reversible kind (a dashboard, a
// datasource…), and the response only names the `group_id`, not which queries
// to refresh — so a coarse invalidate is the correct, race-free choice over
// trying to map a group back to specific keys.
function invalidateAll(queryClient: ReturnType<typeof useQueryClient>) {
  queryClient.invalidateQueries({ queryKey: ["nexus"] });
}

export function useUndo() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<UndoResponse, Error, void>({
    mutationFn: () => undo(client),
    onSuccess: () => invalidateAll(queryClient),
  });
}

export function useRedo() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<UndoResponse, Error, void>({
    mutationFn: () => redo(client),
    onSuccess: () => invalidateAll(queryClient),
  });
}
