// `nav.ts` — canonical URLs/routes for the Provision device page, used
// both for in-app navigation and for the shareable QR/link. Keeping this
// in one place means the link you scan and the link you click are always
// the same shape.
import { EXTENSION_ID } from "../types";

/** The host pathname for a device's detail page. */
export function deviceHref(deviceId: string): string {
  const q = new URLSearchParams({ id: deviceId });
  return `/extensions/${EXTENSION_ID}/provision/device?${q.toString()}`;
}

/** Absolute, shareable URL for a device's detail page (for QR / copy-link). */
export function deviceShareUrl(deviceId: string): string {
  if (typeof window === "undefined") return deviceHref(deviceId);
  return new URL(deviceHref(deviceId), window.location.origin).toString();
}

/**
 * Navigate to a device page. Uses History API + a popstate dispatch so the
 * host's route hook re-reads `window.location` without a full reload.
 */
export function gotoDevice(deviceId: string): void {
  if (typeof window === "undefined") return;
  window.history.pushState(window.history.state, "", deviceHref(deviceId));
  window.dispatchEvent(new PopStateEvent("popstate"));
}

/** Navigate back to the devices list tab. */
export function gotoDevicesList(): void {
  if (typeof window === "undefined") return;
  const href = `/extensions/${EXTENSION_ID}/provision/devices`;
  window.history.pushState(window.history.state, "", href);
  window.dispatchEvent(new PopStateEvent("popstate"));
}
