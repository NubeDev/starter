// `remoteEntry.ts` — what the rubix-frontend host loads at
// `/api/v1/extensions/com.nubeio.ce/ui/remoteEntry.js`.
//
// The default export matches `ExtensionRemoteFactory` from
// `@nube/starter-ext-sdk-ts`: a record of singletons this remote was
// built against, plus `init(handle)` the host calls once singleton
// negotiation succeeds. Component names must match the
// `contributes.ui.exposes[*].name` in `block.yaml`.

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";

import Main from "./main";
import NavTree from "./nav-tree";

interface RemoteFactoryShape {
  singletons: Record<string, { version: string }>;
  init(handle: ExtensionRemoteHandle): Promise<void> | void;
}

const factory: RemoteFactoryShape = {
  singletons: {
    react: { version: "19.1.0" },
    "react-dom": { version: "19.1.0" },
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { Main, NavTree },
    });
  },
};

export default factory;
