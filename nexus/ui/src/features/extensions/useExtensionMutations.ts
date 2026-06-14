import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { installExtension } from "@/api/extensions/install";
import {
  cleanupPreview,
  disableExtension,
  enableExtension,
  purgeExtension,
  restartExtension,
} from "@/api/extensions/lifecycle";
import type {
  CleanupPreview,
  EnablementResponse,
  InstallResponse,
  PurgeResponse,
} from "@/api/extensions/types";
import { EXTENSIONS_KEY } from "@/features/extensions/useExtensions";

// Mutations over the extension admin API. Every state change invalidates
// the list so the row re-renders from the server's truth.

export function useEnableExtension() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<EnablementResponse, Error, string>({
    mutationFn: (id) => enableExtension(client, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: EXTENSIONS_KEY });
    },
  });
}

export function useDisableExtension() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<EnablementResponse, Error, string>({
    mutationFn: (id) => disableExtension(client, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: EXTENSIONS_KEY });
    },
  });
}

export function useRestartExtension() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id) => restartExtension(client, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: EXTENSIONS_KEY });
    },
  });
}

// Dry-run cleanup manifest, fetched on demand before the destructive
// confirm. A mutation (not a query) because it only runs on click.
export function useCleanupPreview() {
  const client = useStarterClient();
  return useMutation<CleanupPreview, Error, string>({
    mutationFn: (id) => cleanupPreview(client, id),
  });
}

// `DELETE …?purge=true` — confirmed against the cleanup manifest first.
export function usePurgeExtension() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<PurgeResponse, Error, string>({
    mutationFn: (id) => purgeExtension(client, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: EXTENSIONS_KEY });
    },
  });
}

export function useInstallExtension() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<InstallResponse, Error, File>({
    mutationFn: (file) => installExtension(client, file),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: EXTENSIONS_KEY });
    },
  });
}
