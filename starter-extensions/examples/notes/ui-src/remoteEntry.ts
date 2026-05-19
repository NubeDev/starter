// `remoteEntry.ts` — the Module-Federation entry the host's
// `@nube/starter-ext-ui` runtime loads at `/extensions/com.nube.notes/ui/
// remoteEntry.js`. A production build would compile this through
// rspack/webpack with the Module-Federation plugin; the host's
// federation runtime negotiates the React singleton and calls
// `init(handle)`.

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";

import NotesPanel from "./Panel.js";

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
      components: { NotesPanel },
    });
  },
};

export default factory;
