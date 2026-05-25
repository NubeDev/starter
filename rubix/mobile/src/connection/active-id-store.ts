// connection/active-id-store.ts — the one place where the
// "multi-instance" promise is actually made.
//
// `starterQueryKey` from `@nube/starter-ui-core/query` is a pure helper
// that prefixes a key array with an active-connection scope; it does not
// read from React context (APP-SHELL.md §Active-connection publication).
// Mobile publishes the active id through this module-level zustand atom
// instead.
//
// The store is WRITTEN BY EXACTLY ONE PLACE — `ConnectionProvider`'s
// `setActiveId` — so two readers can never disagree. If you add a second
// writer, the multi-instance guarantee silently breaks; update this
// comment and APP-SHELL.md to match.

import { create } from 'zustand';

interface ActiveIdState {
  id: string | null;
  setId: (id: string | null) => void;
}

const activeIdStore = create<ActiveIdState>((set) => ({
  id: null,
  setId: (id) => set({ id }),
}));

/** Read the active-connection id. Returns `null` until ConnectionProvider
 *  has resolved + selected one (fresh install: no connections). */
export function useActiveConnectionId(): string | null {
  return activeIdStore((s) => s.id);
}

/** Internal — called by ConnectionProvider's setActiveId only. */
export function _setActiveIdInternal(id: string | null): void {
  activeIdStore.getState().setId(id);
}
