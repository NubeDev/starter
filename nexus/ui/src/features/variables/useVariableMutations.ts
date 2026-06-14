import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { createVariable } from "@/api/variables/create";
import { removeVariable } from "@/api/variables/remove";
import { updateVariable } from "@/api/variables/update";
import type {
  CreateVariableRequest,
  UpdateVariableRequest,
  VariableDetail,
} from "@/api/types";
import { variablesKey } from "@/features/variables/useDashboardVariables";

// CRUD mutations for a dashboard's variables, each invalidating the
// dashboard's variable query so the bar and editor re-resolve from the
// server after a definition change. Keyed by slug so one dashboard's edits
// never refetch another's.

export function useCreateVariable(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<VariableDetail, Error, CreateVariableRequest>({
    mutationFn: (request) => createVariable(client, slug, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: variablesKey(slug) });
    },
  });
}

export function useUpdateVariable(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<
    VariableDetail,
    Error,
    { id: string; patch: UpdateVariableRequest }
  >({
    mutationFn: ({ id, patch }) => updateVariable(client, id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: variablesKey(slug) });
    },
  });
}

export function useRemoveVariable(slug: string) {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: (id) => removeVariable(client, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: variablesKey(slug) });
    },
  });
}
