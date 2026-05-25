// # @nube/starter-ui-sdui-react
//
// Headless renderer for starter SDUI trees. Mount one `<SduiPage>`,
// supply an `SduiTransport` via `<SduiProvider>`, and the package
// dispatches one renderer per IR variant. Zero I/O — every network
// call rides on the transport.

export { SduiPage } from "./sdui-page.js";
export type { SduiPageProps } from "./sdui-page.js";

export {
  SduiProvider,
  useSduiContext,
  useSduiTransport,
} from "./provider/sdui-provider.js";
export type {
  SduiContextValue,
  SduiProviderProps,
} from "./provider/sdui-provider.js";

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

export * from "./renderer/index.js";
