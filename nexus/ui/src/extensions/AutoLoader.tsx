import * as React from "react";
import { useAuth } from "@/auth/AuthProvider";
import { bootstrapExtensions } from "@nube/starter-ext-ui";

import { getExtensionHost } from "@/extensions/host";

// Loads every enabled, UI-contributing remote once a session exists.
// `GET /api/v1/extensions` is admin-gated and 401s before login, so we
// wait for `isAuthenticated`, then run once (the `done` ref guards
// StrictMode's double-effect and later re-renders). On failure we reset
// the ref so a later auth change retries.
export function ExtensionAutoLoader(): null {
  const { isAuthenticated } = useAuth();
  const done = React.useRef(false);

  React.useEffect(() => {
    if (!isAuthenticated || done.current) return;
    done.current = true;
    void bootstrapExtensions(getExtensionHost(), {
      basePath: "/api/v1/extensions",
      onRegistered: (id) => console.info(`[nexus.extensions] loaded ${id}`),
    }).catch((err: unknown) => {
      console.warn("[nexus.extensions] bootstrap failed:", err);
      done.current = false;
    });
  }, [isAuthenticated]);

  return null;
}
