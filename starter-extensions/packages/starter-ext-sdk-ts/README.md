# @nube/starter-ext-sdk-ts

What UI extension authors import.

Forked from `rubix-workspace/extension-ui-sdk` main entry, with the
rubix-specific graph hooks (`useNode`, `useSlot`, `useKinds`) stripped
— those belong in rubix-agent and have no analogue in a generic
starter (DOCS/extensions/scope/SCOPE.md §"UI package source").

## v0.1 surface

- `useHostClient()` — typed wrapper over `@nube/starter-client-ts`.
  UI extensions never issue raw `fetch` calls (SCOPE R11); every host
  call goes through the injected client so auth, tracing, and retry
  are uniform.
- `BlockShell` — standard panel wrapper providing the slot context,
  error boundary, and loading state.
- `useSlotContext()` — read the slot id, host theme, and feature
  flags relevant to the place the extension is mounted in.
- `registerExtensionContributions({ components })` — single
  registration entry point called from the extension's `remoteEntry`
  `init`.

## Author-side example

```tsx
// remoteEntry.ts
import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "@nube/starter-ext-sdk-ts";
import HelloPanel from "./Panel.js";

export default {
  singletons: { react: { version: "18.3.1" } },
  async init(handle: ExtensionRemoteHandle) {
    registerExtensionContributions(handle, {
      components: { HelloPanel },
    });
  },
};
```

## Localizing `BlockShell` chrome

The two strings the shell renders itself — the Suspense skeleton
("Loading…") and the error-boundary header ("Extension failed:") —
are overridable. The SDK does not bundle `react-intl`; extensions
that already pull a translator from `useHostTranslate()` pass
pre-translated strings in:

```tsx
import { BlockShell, type BlockShellMessages } from "@nube/starter-ext-sdk-ts";

const messages: Partial<BlockShellMessages> = {
  loading:    t("ext.shell.loading"),
  errorTitle: t("ext.shell.errorTitle"),
};

<BlockShell messages={messages}>
  <YourContent />
</BlockShell>
```

`DEFAULT_BLOCK_SHELL_MESSAGES` and `mergeBlockShellMessages` are
exported for consumers building on top of the English fallback.
