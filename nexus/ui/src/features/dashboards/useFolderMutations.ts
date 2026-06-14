import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { createFolder } from "@/api/folders/create";
import { removeFolder } from "@/api/folders/remove";
import { updateFolder } from "@/api/folders/update";
import type {
  CreateFolderRequest,
  FolderSummary,
  UpdateFolderRequest,
} from "@/api/types";
import { DASHBOARDS_KEY } from "@/features/dashboards/useDashboards";
import { FOLDERS_KEY } from "@/features/dashboards/useFolders";

// Create a folder. Invalidates the folder tree so the new node appears.
export function useCreateFolder() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<FolderSummary, Error, CreateFolderRequest>({
    mutationFn: (request) => createFolder(client, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: FOLDERS_KEY });
    },
  });
}

// Rename or reparent a folder. Invalidates the tree.
export function useUpdateFolder() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<
    FolderSummary,
    Error,
    { id: string; patch: UpdateFolderRequest }
  >({
    mutationFn: ({ id, patch }) => updateFolder(client, id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: FOLDERS_KEY });
    },
  });
}

// Delete a folder. Its children and filed dashboards are re-rooted server-side,
// so both the tree and the dashboard list may change — invalidate both.
export function useDeleteFolder() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: (id) => removeFolder(client, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: FOLDERS_KEY });
      queryClient.invalidateQueries({ queryKey: DASHBOARDS_KEY });
    },
  });
}
