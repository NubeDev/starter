// # @nube/starter-ui-sdui-react/headless
//
// Platform-neutral SDUI building blocks. Mount `<SduiPage>` under an
// `<SduiProvider>`, supply renderers via `customRenderers`, and walk
// IR trees with `Render`. This subpath MUST NOT pull in any
// `@nube/starter-ui-kit` (web-only) imports — the mobile UI kit will
// supply its own renderers via the same registry seam.

export { SduiPage } from "./sdui-page.js";
export type { SduiPageProps } from "./sdui-page.js";

export {
  SduiProvider,
  useSduiContext,
  useSduiTransport,
} from "./sdui-provider.js";
export type {
  SduiContextValue,
  SduiProviderProps,
} from "./sdui-provider.js";

export {
  PageStateProvider,
  usePageState,
  usePageStateKey,
} from "./page-state.js";
export type { PageState, SetPageState } from "./page-state.js";

export { useSduiResolve } from "./hooks/use-resolve.js";
export type { UseSduiResolveOptions } from "./hooks/use-resolve.js";
export { useSduiAction } from "./hooks/use-action.js";
export { useSduiSubscriptions } from "./hooks/use-subscriptions.js";

export {
  createHttpSduiTransport,
} from "./transport/index.js";
export type {
  SduiTransport,
  HttpSduiTransportOptions,
} from "./transport/index.js";

export { Render, RenderChildren } from "./render.js";
export { PlaceholderRender } from "./placeholder-render.js";
export {
  registerRenderer,
  lookupRenderer,
  listRenderers,
} from "./registry.js";
export type { Renderer, RenderProps, CustomRendererRegistry } from "./registry.js";
