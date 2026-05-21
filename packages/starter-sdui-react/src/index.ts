// # @nube/starter-sdui-react
//
// React renderer for starter SDUI trees. Ports rubix-ui-core/src/sdui/
// verbatim and projects against @nube/starter-ui-kit shadcn primitives
// (divergence D2 — see DOCS/frontend/sdui/DIVERGENCE.md).

export { SduiProvider, useSdui, globalCustomRegistry, registerCustomRenderer } from "./context.js";
export type { SduiCtx, ActionFn, CustomRegistry } from "./context.js";

export { Renderer, RendererList } from "./Renderer.js";
export { SduiPage } from "./SduiPage.js";
export type { SduiPageProps, SduiResolver, SduiActionDispatcher } from "./SduiPage.js";
export { SduiRenderPage } from "./SduiRenderPage.js";
export type { SduiRenderPageProps } from "./SduiRenderPage.js";
export { SduiDialogHost } from "./SduiDialogHost.js";

export { SUPPORTED_IR_VERSION, checkIrVersion } from "./capability.js";
export type { CapabilityMismatch } from "./capability.js";

export { mergeAt, replaceAt } from "./applyPatch.js";
export { bindRow } from "./row-bind.js";
export {
  pushDialog,
  popDialog,
  subscribeDialogStack,
  dialogStackSize,
} from "./dialog-bus.js";

export { useActionResponse } from "./useActionResponse.js";
export { useSubscriptions } from "./useSubscriptions.js";
export type { SubscriptionTransport } from "./useSubscriptions.js";
export { useBoundWrite } from "./useBoundWrite.js";
export type { BoundWrite } from "./useBoundWrite.js";

export {
  useStreaming,
  registerStreamingTransport,
  getStreamingTransport,
  useStreamingTransport,
} from "./useStreaming.js";
export type {
  StreamEndReason,
  StreamEndSentinel,
  UseStreamingOptions,
  StreamingState,
} from "./useStreaming.js";

export { evaluateShowWhen } from "./show-when.js";

export {
  builtinComponentRegistry,
  lookupSpec,
} from "./registry/index.js";
export type {
  ComponentRegistry,
  ComponentSpec,
  Kind,
} from "./registry/index.js";

export type {
  UiComponent,
  UiComponentTree,
  UiResolveResponse,
  UiResolveResponseOk,
  UiResolveResponseDryRun,
  UiActionResponse,
  UiTableRow,
  WritePlanEntry,
  SubscriptionPlan,
  SubscriptionSubject,
  Diagnostic,
  NodeStyle,
  ShowWhen,
} from "./types.js";
