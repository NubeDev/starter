// `remoteEntry.ts` — what nexus-ui's federation runtime loads at
// `/api/v1/extensions/com.acme.devices/ui/remoteEntry.js`.
//
// The default export matches `ExtensionRemoteFactory` from
// `@nube/starter-ext-sdk-ts`: the singletons this remote was built against,
// plus `init(handle)` the host calls once singleton negotiation succeeds.
//
// Component names MUST match `contributes.ui.exposes[*].name` in `block.yaml`
// (`DevicesMain`, `DevicesNav`). `DevicesMain` is the single `main`-slot router
// that dispatches the dashboard / provision sub-pages (see `main.tsx`).

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";

import DevicesMain from "./main";
import DevicesNav from "./nav";

interface RemoteFactoryShape {
  singletons: Record<string, { version: string }>;
  init(handle: ExtensionRemoteHandle): Promise<void> | void;
}

const factory: RemoteFactoryShape = {
  singletons: {
    react: { version: "19.1.0" },
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { DevicesMain, DevicesNav },
    });
  },
};

export default factory;
