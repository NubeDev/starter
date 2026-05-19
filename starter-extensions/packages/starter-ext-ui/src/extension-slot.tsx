// `<ExtensionSlot id="sidebar"/>` — mounts every contribution whose
// manifest sets `slot: sidebar`.
//
// The slot resolves contributions in declared source order: insertion
// order of `registerExtensionRemote` × declaration order in
// `contributes.ui.exposes`. The host shell can wrap the slot in
// whatever container component it wants (a list, a tab strip, a
// vertical stack).
//
// Every mounted contribution is wrapped in `SlotContextProvider` so
// extensions can call `useSlotContext()` without each one wiring up
// the same context themselves.

import * as React from "react";

import { SlotContextProvider } from "@nube/starter-ext-sdk-ts";
import type { HostThemeMode, HostThemeTokens } from "@nube/starter-ext-sdk-ts";

import { useExtensionHostManager } from "./host-context.js";

export interface ExtensionSlotProps {
  /**
   * Slot id every contribution declares in `contributes.ui.exposes[*].slot`.
   * Slot ids are free-form strings — the host owns the namespace.
   */
  id: string;
  /**
   * Host theme mode passed down to the per-slot context. Defaults to
   * `"light"`; consumers wiring real theming pipe their current
   * mode (typically `useTheme().resolved` from
   * `@nube/starter-ui-kit`) through.
   */
  theme?: HostThemeMode;
  /**
   * Resolved theme token map for the current mode. When supplied,
   * extensions can read it programmatically via `useHostTheme()`;
   * when omitted, the hook falls back to live `getComputedStyle` on
   * `document.documentElement`. The host's theme editor in
   * `@nube/starter-ui-core/theme-editor` exposes the map via its
   * `useThemeEditorStore`; consumers wiring the editor through pipe
   * `state.styles[state.mode]` here.
   */
  themeTokens?: HostThemeTokens;
  /**
   * Feature flags passed to every contribution mounted in this slot.
   * The host's flag store is opaque to this package; the consumer
   * decides which flags surface where.
   */
  flags?: Readonly<Record<string, boolean>>;
}

export function ExtensionSlot(props: ExtensionSlotProps): React.ReactElement {
  const mgr = useExtensionHostManager();
  // `useSyncExternalStore` is the v0.1 React story for subscribing
  // to non-React state without tearing. The snapshot is the
  // slot-resolution array; `resolveSlot` returns a fresh array each
  // call so React's reference-equality compare correctly invalidates.
  const resolved = React.useSyncExternalStore(
    React.useCallback((cb) => mgr.subscribe(cb), [mgr]),
    React.useCallback(() => mgr.resolveSlot(props.id), [mgr, props.id]),
    React.useCallback(() => mgr.resolveSlot(props.id), [mgr, props.id]),
  );

  return (
    <div data-ext-slot={props.id} className="starter-ext-slot">
      {resolved.map((r) => {
        const Comp = r.component;
        if (!Comp) {
          // `init` not done yet, or registered no component by that
          // name. Render nothing — the SlotContext is irrelevant.
          return null;
        }
        return (
          <SlotContextProvider
            // Key is extensionId+exposeName: an extension can expose
            // more than one component into the same slot, and React
            // needs a stable identity.
            key={`${r.extensionId}::${r.expose.name}`}
            value={{
              slotId: props.id,
              extensionId: r.extensionId,
              theme: props.theme ?? "light",
              themeTokens: props.themeTokens ?? null,
              flags: props.flags ?? {},
            }}
          >
            <Comp />
          </SlotContextProvider>
        );
      })}
    </div>
  );
}
