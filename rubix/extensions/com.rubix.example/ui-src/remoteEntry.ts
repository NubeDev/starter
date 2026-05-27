// `remoteEntry.ts` — what the rubix-frontend host's federation
// runtime loads at `/api/v1/extensions/com.rubix.example/ui/
// remoteEntry.js`.
//
// The default export matches `ExtensionRemoteFactory` from
// `@nube/starter-ext-ui`: a record of singletons this remote was
// built against, plus `init(handle)` the host calls once singleton
// negotiation succeeds. The host hands us back its React, ReactDOM,
// etc. via `handle.singletons` — the SDK wraps each registered
// component in a `HostBindingsProvider` seeded from that handle, so
// `useHostTheme` / `useSlotContext` / `useHostTranslate` resolve
// against the host's instances even though our components were
// authored in isolation.
//
// Component names MUST match `contributes.ui.exposes[*].name` in
// `block.yaml` (`Main`, `Sidebar`) — that is how the host looks
// them up when mounting an `<ExtensionSlot/>`.

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";

import Main from "./main";
import NavTree from "./nav-tree";
import Sidebar from "./sidebar";

interface RemoteFactoryShape {
  singletons: Record<string, { version: string }>;
  init(handle: ExtensionRemoteHandle): Promise<void> | void;
}

const factory: RemoteFactoryShape = {
  // The host enforces matching-majors. Declare the React / ReactDOM
  // we authored against; the host will refuse to load this remote
  // if it ships a different React major.
  // Host (rubix-frontend) ships React 19. The host's singleton
  // gate compares majors only, so any 19.x works.
  singletons: {
    react: { version: "19.1.0" },
    "react-dom": { version: "19.1.0" },
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { Main, NavTree, Sidebar },
    });
  },
};

export default factory;
