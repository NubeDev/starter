// Importmap shim — see `react.mjs` for the contract.
const RD = /** @type {any} */ (globalThis).__rubixReactDom;
if (!RD) {
  throw new Error(
    "rubix react-dom-shim: globalThis.__rubixReactDom is unset."
  );
}
export default RD;
export const {
  createPortal, flushSync, version,
} = RD;
