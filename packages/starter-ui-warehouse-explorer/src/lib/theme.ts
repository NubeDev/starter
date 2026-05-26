// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: 1a0736055a4647c18d0be19347e4325007c7bd52.
// Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.
//
// Upstream's theme.provider.tsx owned theme state. In the rubix shell the
// host owns theme — the `.dark` class on <html> drives upstream's dark
// variants. This hook just observes that class so components that
// branch on `currentTheme === "light"` still work.

import { useEffect, useState } from "react";

export type Theme = "light" | "dark";

function read(): Theme {
  if (typeof document === "undefined") return "light";
  return document.documentElement.classList.contains("dark") ? "dark" : "light";
}

export function useTheme(): Theme {
  const [theme, setTheme] = useState<Theme>(read);

  useEffect(() => {
    if (typeof MutationObserver === "undefined") return;
    const root = document.documentElement;
    const obs = new MutationObserver(() => setTheme(read()));
    obs.observe(root, { attributes: true, attributeFilter: ["class"] });
    return () => obs.disconnect();
  }, []);

  return theme;
}
