import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { createNavNode } from "@/api/nav/create";
import { removeNavNode } from "@/api/nav/remove";
import { updateNavNode } from "@/api/nav/update";
import type {
  CreateNavNodeRequest,
  NavNodeDetail,
  UpdateNavNodeRequest,
} from "@/api/types";
import { navKey } from "@/features/nav/useNavTree";

// CRUD mutations for the navigation tree, each invalidating the nav query so
// the sidebar + builder re-fetch the access-filtered tree after a change.

export function useCreateNavNode() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<NavNodeDetail, Error, CreateNavNodeRequest>({
    mutationFn: (request) => createNavNode(client, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: navKey });
    },
  });
}

export function useUpdateNavNode() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<
    NavNodeDetail,
    Error,
    { id: string; patch: UpdateNavNodeRequest }
  >({
    mutationFn: ({ id, patch }) => updateNavNode(client, id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: navKey });
    },
  });
}

export function useRemoveNavNode() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: (id) => removeNavNode(client, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: navKey });
    },
  });
}
