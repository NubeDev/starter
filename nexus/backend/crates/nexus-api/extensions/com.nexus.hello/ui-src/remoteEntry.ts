// `remoteEntry.ts` — what nexus-ui's federation runtime loads at
// `/api/v1/extensions/com.nexus.hello/ui/remoteEntry.js`.
//
// The default export matches `ExtensionRemoteFactory` from
// `@nube/starter-ext-ui`: the singletons this remote was built against, plus
// `init(handle)` the host calls once singleton negotiation succeeds. The SDK's
// `registerExtensionContributions` wraps each component in a
// `HostBindingsProvider` seeded from the handle, so `useSlotContext` /
// `useHostClient` resolve against the host's instances.
//
// Component names MUST match `contributes.ui.exposes[*].name` in `block.yaml`
// (`HelloPanel`) — that is how the host looks them up when mounting an
// `<ExtensionSlot/>`.

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";

import HelloPanel from "./panel";

interface RemoteFactoryShape {
  singletons: Record<string, { version: string }>;
  init(handle: ExtensionRemoteHandle): Promise<void> | void;
}

const factory: RemoteFactoryShape = {
  // The host enforces matching majors; nexus-ui ships React 19, so any 19.x
  // declaration negotiates.
  singletons: {
    react: { version: "19.1.0" },
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { HelloPanel },
    });
  },
};

export default factory;
