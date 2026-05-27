// Importmap shim — re-exports the host's bundled React via a global
// the host publishes in `main.tsx`. Loaded by extension `remoteEntry`
// bundles whose imports of `react` resolve through the importmap in
// `index.html`. See rubix/frontend/README.md § "Extension React sharing".
const R = /** @type {any} */ (globalThis).__rubixReact;
if (!R) {
  throw new Error(
    "rubix react-shim: globalThis.__rubixReact is unset. The host did not publish React before the extension bundle was imported."
  );
}
export default R;
export const {
  Children, Component, Fragment, Profiler, PureComponent, StrictMode, Suspense,
  cloneElement, createContext, createElement, createRef, forwardRef, isValidElement,
  lazy, memo, startTransition,
  useCallback, useContext, useDebugValue, useDeferredValue, useEffect, useId,
  useImperativeHandle, useInsertionEffect, useLayoutEffect, useMemo, useReducer,
  useRef, useState, useSyncExternalStore, useTransition, version,
} = R;
