import { Link } from "react-router-dom";
import { Users, Boxes, FolderKanban, MapPin, ArrowRight } from "lucide-react";
import { useStore } from "@/store/store";
import { Card, SectionTitle } from "@/components/ui";

export function Dashboard() {
  const { state } = useStore();
  const gateways = state.templates.filter((t) => t.role === "gateway").length;
  const endDevices = state.templates.filter((t) => t.role === "end_device").length;

  const stats = [
    { label: "Clients", value: state.clients.length, icon: Users, to: "/clients" },
    { label: "Sites", value: state.sites.length, icon: MapPin, to: "/clients" },
    { label: "Templates", value: `${gateways}gw / ${endDevices}dev`, icon: Boxes, to: "/templates" },
    { label: "Projects", value: state.projects.length, icon: FolderKanban, to: "/projects" },
  ];

  return (
    <>
      <SectionTitle
        title="Dashboard"
        sub="BMS / Electrical-EMS project builder — POC. Admin loads clients, sites & templates; clients build projects and export to PDF / Excel / provision JSON."
      />
      <div className="grid grid-cols-4 gap-4 mb-8">
        {stats.map((s) => (
          <Link key={s.label} to={s.to}>
            <Card className="hover:border-accent transition-colors">
              <s.icon size={18} className="text-accent mb-3" />
              <div className="text-2xl font-semibold">{s.value}</div>
              <div className="text-sm text-muted">{s.label}</div>
            </Card>
          </Link>
        ))}
      </div>

      <div className="grid grid-cols-3 gap-4">
        <Step n={1} title="Admin: Clients & Sites" to="/clients" body="Add the client organisations and their physical sites." />
        <Step n={2} title="Admin: Templates" to="/templates" body="Load gateway and end-device blueprints (network, settings, points)." />
        <Step n={3} title="Build a Project" to="/projects" body="Pick a site, drop gateways & devices from templates, then export." />
      </div>
    </>
  );
}

function Step({ n, title, body, to }: { n: number; title: string; body: string; to: string }) {
  return (
    <Link to={to}>
      <Card className="h-full hover:border-accent transition-colors">
        <div className="flex items-center gap-2 mb-2">
          <span className="grid place-items-center w-6 h-6 rounded-full bg-accent text-white text-xs font-bold">
            {n}
          </span>
          <span className="font-medium">{title}</span>
        </div>
        <p className="text-sm text-muted mb-3">{body}</p>
        <span className="text-accent text-sm inline-flex items-center gap-1">
          Open <ArrowRight size={14} />
        </span>
      </Card>
    </Link>
  );
}
