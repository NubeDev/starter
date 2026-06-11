import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { createInsight } from "@/api/insights/create";
import { updateInsight } from "@/api/insights/update";
import { removeInsight } from "@/api/insights/remove";
import type {
  CreateInsightRequest,
  InsightSummary,
  UpdateInsightRequest,
} from "@/api/types";

const INSIGHTS_KEY = ["nexus", "insights"] as const;

// Create an insight, then refresh the list. The backend compile-checks the
// Rhai script; a non-compiling script comes back as a 400 whose message the
// transport surfaces as the mutation error, so the form can render it inline.
export function useCreateInsight() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<InsightSummary, Error, CreateInsightRequest>({
    mutationFn: (body) => createInsight(client, body),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: INSIGHTS_KEY }),
  });
}

// Update an insight, then refresh the list. Like create, a replaced script is
// compile-checked and a failure surfaces as the mutation error.
export function useUpdateInsight() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<
    InsightSummary,
    Error,
    { id: string; body: UpdateInsightRequest }
  >({
    mutationFn: ({ id, body }) => updateInsight(client, id, body),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: INSIGHTS_KEY }),
  });
}

// Remove an insight, then refresh the list.
export function useRemoveInsight() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: (id) => removeInsight(client, id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: INSIGHTS_KEY }),
  });
}
