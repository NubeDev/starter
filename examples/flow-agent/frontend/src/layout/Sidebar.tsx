// Phase 1 sidebar — minimal, flat. The proper nested tree with
// persistence + SSE-driven updates lands in Phase 6 once a Sidebar
// primitive ships in `@nube/starter-ui-kit`.

import { NavLink } from "react-router-dom";
import { cn } from "@nube/starter-ui-kit";

const sections: Array<{ to: string; label: string }> = [
  { to: "/flows", label: "Flows" },
  { to: "/agents", label: "Agents" },
  { to: "/settings", label: "Settings" },
];

export function Sidebar() {
  return (
    <aside className="hidden w-60 shrink-0 border-r border-border/60 bg-sidebar/40 md:block">
      <nav className="flex flex-col gap-1 p-3">
        {sections.map((s) => (
          <NavLink
            key={s.to}
            to={s.to}
            className={({ isActive }) =>
              cn(
                "rounded-lg px-3 py-2 text-sm transition-colors duration-150",
                isActive
                  ? "bg-accent/60 text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent/40 hover:text-foreground",
              )
            }
          >
            {s.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
