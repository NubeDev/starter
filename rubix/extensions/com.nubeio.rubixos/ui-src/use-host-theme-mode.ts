// `useHostThemeMode` — reactive read of the host's colour mode.
//
// Why not the SDK's `useHostTheme()`: it reads from `SlotContext`,
// which the host populates from the `theme` prop passed to
// `<ExtensionSlot>`. The current rubix-frontend route
// (`extensions.$extId.$.tsx`) does not pass `theme`, so the SDK
// hook reports `"light"` even when the host is in dark mode.
//
// Until the host wires that through, the most reliable cross-
// theme signal is the `data-mode="dark|light"` attribute the
// host writes on `<html>` (the same attribute the host's own
// theme provider sets from `useLayoutPreferences()`, and the
// same one our `:root[data-mode="dark"]` CSS reads). We
// subscribe with a `MutationObserver` on `attributes` so a live
// theme toggle in the host's settings drawer flips the map
// basemap without a refresh.

import * as React from "react";

export type HostThemeMode = "light" | "dark";

function readMode(): HostThemeMode {
  if (typeof document === "undefined") return "dark";
  return document.documentElement.getAttribute("data-mode") === "dark"
    ? "dark"
    : "light";
}

export function useHostThemeMode(): HostThemeMode {
  const [mode, setMode] = React.useState<HostThemeMode>(() => readMode());

  React.useEffect(() => {
    if (typeof document === "undefined") return;
    // Re-read on every attribute mutation to <html>; covers manual
    // toggle, system-pref change, theme-store hydration.
    const update = () => setMode(readMode());
    const obs = new MutationObserver(update);
    obs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-mode", "class"],
    });
    // Catch any change that happened between initial state and
    // effect mount (theme store hydration can race the first
    // paint).
    update();
    return () => obs.disconnect();
  }, []);

  return mode;
}
