// `remoteEntry.ts` — what the host's federation runtime loads.
//
// The default export matches `ExtensionRemoteFactory` from
// `@nube/starter-ext-ui`: a record of singletons the extension was
// built against, plus an `init(handle)` the runtime calls once
// singleton negotiation succeeds.
//
// In a real consumer build this file is compiled by webpack /
// rspack / rsbuild with the Module Federation plugin enabled,
// emitting `dist/ui/remoteEntry.js`. The bundle exposes `./Panel`
// (mapped to `Panel.tsx`) and externalises `react` so the host's
// singleton is the one that runs.
//
// For host-side testing (and the `two-extensions-no-React-
// duplication` smoke test), the factory object is consumed directly
// — no bundler involved.

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";

import HelloPanel from "./Panel.js";

interface RemoteFactoryShape {
  singletons: Record<string, { version: string }>;
  init(handle: ExtensionRemoteHandle): Promise<void> | void;
}

const factory: RemoteFactoryShape = {
  singletons: {
    react: { version: "18.3.1" },
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { HelloPanel },
    });
  },
};

export default factory;
