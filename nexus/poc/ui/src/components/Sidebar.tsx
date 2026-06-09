// Left navigation between the POC sections.

export type Route =
  | "builder"
  | "sql"
  | "inputs"
  | "outputs"
  | "processors"
  | "buffers"
  | "plugins";

const ITEMS: { route: Route; label: string }[] = [
  { route: "builder", label: "Stream Builder" },
  { route: "sql", label: "SQL Playground" },
  { route: "inputs", label: "Inputs" },
  { route: "outputs", label: "Outputs" },
  { route: "processors", label: "Processors" },
  { route: "buffers", label: "Buffers" },
  { route: "plugins", label: "Plugins" },
];

interface Props {
  active: Route;
  onNavigate: (route: Route) => void;
}

export function Sidebar({ active, onNavigate }: Props) {
  return (
    <nav className="sidebar">
      <div className="brand">
        <span className="dot" /> Nexus · ArkFlow
      </div>
      {ITEMS.map((item) => (
        <button
          key={item.route}
          className={item.route === active ? "nav-item active" : "nav-item"}
          onClick={() => onNavigate(item.route)}
        >
          {item.label}
        </button>
      ))}
      <div className="sidebar-foot">POC · embeds the ArkFlow engine</div>
    </nav>
  );
}
