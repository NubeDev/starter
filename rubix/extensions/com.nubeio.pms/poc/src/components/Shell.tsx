import { NavLink, Outlet } from "react-router-dom";
import {
  LayoutDashboard,
  Users,
  Boxes,
  FolderKanban,
  Zap,
  RotateCcw,
} from "lucide-react";
import { useStore } from "@/store/store";

const nav = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, end: true },
  { to: "/clients", label: "Clients & Sites", icon: Users },
  { to: "/templates", label: "Templates", icon: Boxes },
  { to: "/projects", label: "Projects", icon: FolderKanban },
];

export function Shell() {
  const { dispatch } = useStore();
  return (
    <div className="flex h-full">
      <aside className="w-60 shrink-0 border-r border-border bg-panel flex flex-col">
        <div className="flex items-center gap-2 px-5 py-5">
          <div className="grid place-items-center w-8 h-8 rounded-lg bg-accent">
            <Zap size={18} className="text-white" />
          </div>
          <div>
            <div className="font-semibold leading-tight">Rubix PMS</div>
            <div className="text-[11px] text-muted">Project Builder · POC</div>
          </div>
        </div>
        <nav className="flex-1 px-3 space-y-1">
          {nav.map((n) => (
            <NavLink
              key={n.to}
              to={n.to}
              end={n.end}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
                  isActive ? "bg-panel-2 text-white" : "text-muted hover:text-white hover:bg-panel-2"
                }`
              }
            >
              <n.icon size={17} />
              {n.label}
            </NavLink>
          ))}
        </nav>
        <button
          className="btn btn-ghost m-3 justify-center"
          onClick={() => {
            if (confirm("Reset all data to seed?")) dispatch({ type: "RESET" });
          }}
        >
          <RotateCcw size={15} /> Reset demo data
        </button>
      </aside>
      <main className="flex-1 overflow-y-auto">
        <div className="max-w-6xl mx-auto px-8 py-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
