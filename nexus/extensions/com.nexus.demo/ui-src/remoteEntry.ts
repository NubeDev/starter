// `remoteEntry.ts` — what nexus-ui's federation runtime loads at
// `/api/v1/extensions/com.nexus.demo/ui/remoteEntry.js`.
//
// The default export matches the SDK's `ExtensionRemoteFactory`: the singletons
// this remote was built against, plus `init(handle)` the host calls once
// singleton negotiation succeeds. `registerExtensionContributions` wraps each
// component in a `HostBindingsProvider` seeded from the handle, so
// `useSlotContext` / `useHostClient` / `useExtensionRoute` resolve against the
// host's instances.
//
// Component names MUST match `contributes.ui.exposes[*].name` in `block.yaml`:
//   - `Main`    → slot `main` (the page)
//   - `DemoNav` → slot `sidebar-nav` (the nav entry)

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";

import Main from "./main";
import DemoNav from "./nav";

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
      components: { Main, DemoNav },
    });
  },
};

export default factory;
