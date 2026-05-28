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
