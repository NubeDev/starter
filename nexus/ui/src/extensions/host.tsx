import * as React from "react";
import * as ReactDOM from "react-dom";
import * as ReactDOMClient from "react-dom/client";
import * as ReactJsxRuntime from "react/jsx-runtime";
import * as ReactQuery from "@tanstack/react-query";
import * as Zustand from "zustand";
import {
  ExtensionHostManager,
  type ExtensionHostManagerOptions,
} from "@nube/starter-ext-ui";

import { getNexusClient } from "@/api/client";

// Extension `remoteEntry` bundles externalise React and resolve it
// through the importmap in `index.html`, whose shims read these globals.
// They must be published before any remote `import()`. The in-repo
// `com.nubeio.ce` remote's shims read the `__rubix*` names specifically,
// so we keep those names to mount it unchanged.
function publishReactGlobals(): void {
  const g = globalThis as unknown as Record<string, unknown>;
  g.__rubixReact = React;
  g.__rubixReactDom = ReactDOM;
  g.__rubixReactDomClient = ReactDOMClient;
  g.__rubixReactJsxRuntime = ReactJsxRuntime;
}

let cached: ExtensionHostManager | null = null;

// The host registers the federation singletons (one React, one
// QueryClient/cache, one zustand) with versions; a major mismatch is a
// hard refusal, not a silent second copy. Built lazily so the React
// globals are published exactly once, on first access.
export function getExtensionHost(): ExtensionHostManager {
  if (cached) return cached;
  publishReactGlobals();
  // Neither `@tanstack/react-query` nor `zustand` exports a runtime
  // version constant, and the host's negotiation only compares majors.
  // We declare the majors this host is built against; bumping the dep
  // major is a deliberate change that must be reflected here.
  const reactQueryVersion = "5";
  const zustandVersion = "5";
  const opts: ExtensionHostManagerOptions = {
    client: getNexusClient(),
    singletons: {
      react: { version: React.version, instance: React },
      "react-dom": { version: ReactDOM.version, instance: ReactDOM },
      "@tanstack/react-query": {
        version: reactQueryVersion,
        instance: ReactQuery,
      },
      zustand: { version: zustandVersion, instance: Zustand },
    },
    telemetry: (ev) => {
      const log = ev.severity === "error" ? console.error : console.warn;
      log("[nexus.extensions]", ev);
    },
  };
  cached = new ExtensionHostManager(opts);
  return cached;
}
