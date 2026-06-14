import { useMemo, useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { useCreate, useUpdate, useDelete } from "@refinedev/core";
import { toast } from "sonner";
import {
  ChevronsLeft,
  Hexagon,
  MoreHorizontal,
  Pencil,
  Plus,
  Search,
  Star,
  Trash2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { DashboardFormDialog, type DashboardFormValues } from "@/components/DashboardFormDialog";
import { DashIcon } from "@/lib/icon";
import { cn } from "@/lib/utils";
import { useDashboards } from "@/providers/useStore";
import type { Dashboard } from "@/data/types";

export function Sidebar({ onCollapse }: { onCollapse?: () => void }) {
  const dashboards = useDashboards();
  const navigate = useNavigate();
  const location = useLocation();
  const [query, setQuery] = useState("");
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Dashboard | undefined>();
  const [toDelete, setToDelete] = useState<Dashboard | undefined>();

  const { mutate: create } = useCreate();
  const { mutate: update } = useUpdate();
  const { mutate: remove } = useDelete();

  const filtered = useMemo(
    () => dashboards.filter((d) => d.name.toLowerCase().includes(query.toLowerCase())),
    [dashboards, query]
  );
  const starred = filtered.filter((d) => d.starred);
  const rest = filtered.filter((d) => !d.starred);
  const activeSlug = location.pathname.split("/d/")[1];

  const openNew = () => {
    setEditing(undefined);
    setFormOpen(true);
  };

  const handleSubmit = (values: DashboardFormValues) => {
    if (editing) {
      update(
        { resource: "dashboards", id: editing.id, values },
        { onSuccess: () => toast.success("Dashboard updated") }
      );
    } else {
      create(
        { resource: "dashboards", values },
        {
          onSuccess: (res) => {
            const d = res.data as unknown as Dashboard;
            toast.success("Dashboard created");
            if (d?.slug) navigate(`/d/${d.slug}`);
          },
        }
      );
    }
  };

  const toggleStar = (d: Dashboard) =>
    update({ resource: "dashboards", id: d.id, values: { starred: !d.starred } });

  const confirmDelete = () => {
    if (!toDelete) return;
    const wasActive = activeSlug === toDelete.slug;
    remove(
      { resource: "dashboards", id: toDelete.id },
      {
        onSuccess: () => {
          toast.success(`Deleted “${toDelete.name}”`);
          if (wasActive) navigate("/");
        },
      }
    );
    setToDelete(undefined);
  };

  return (
    <aside className="flex h-full w-[272px] flex-col glass border-r border-white/[0.06]">
      {/* Brand */}
      <div className="flex items-center justify-between px-4 py-4">
        <div className="flex items-center gap-2.5">
          <div className="relative grid h-9 w-9 place-items-center rounded-xl bg-primary/15 ring-1 ring-primary/30">
            <Hexagon className="h-5 w-5 text-primary" />
            <span className="absolute h-1.5 w-1.5 rounded-full bg-primary shadow-[0_0_10px_hsl(152_76%_44%)]" />
          </div>
          <div className="leading-tight">
            <div className="text-sm font-bold tracking-tight text-foreground">Nexus</div>
            <div className="text-[0.7rem] text-muted-foreground">IoT Control</div>
          </div>
        </div>
        {onCollapse && (
          <Button variant="ghost" size="icon-sm" onClick={onCollapse} className="lg:hidden">
            <ChevronsLeft className="h-4 w-4" />
          </Button>
        )}
      </div>

      {/* Search */}
      <div className="px-3 pb-2">
        <div className="relative">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search dashboards"
            className="h-9 pl-9"
          />
        </div>
      </div>

      {/* List */}
      <nav className="scrollbar-thin flex-1 space-y-4 overflow-y-auto px-3 py-2">
        {starred.length > 0 && (
          <Section title="Starred">
            {starred.map((d) => (
              <Item
                key={d.id}
                d={d}
                active={activeSlug === d.slug}
                onOpen={() => navigate(`/d/${d.slug}`)}
                onEdit={() => { setEditing(d); setFormOpen(true); }}
                onStar={() => toggleStar(d)}
                onDelete={() => setToDelete(d)}
              />
            ))}
          </Section>
        )}

        <Section title={`Dashboards · ${rest.length}`}>
          {rest.map((d) => (
            <Item
              key={d.id}
              d={d}
              active={activeSlug === d.slug}
              onOpen={() => navigate(`/d/${d.slug}`)}
              onEdit={() => { setEditing(d); setFormOpen(true); }}
              onStar={() => toggleStar(d)}
              onDelete={() => setToDelete(d)}
            />
          ))}
          {filtered.length === 0 && (
            <div className="px-2 py-6 text-center text-sm text-muted-foreground">
              No dashboards match “{query}”.
            </div>
          )}
        </Section>
      </nav>

      {/* New + user */}
      <div className="space-y-3 border-t border-white/[0.06] p-3">
        <Button className="w-full" onClick={openNew}>
          <Plus /> New dashboard
        </Button>
        <div className="flex items-center gap-3 rounded-xl border border-white/5 bg-white/[0.02] px-3 py-2">
          <img
            src="https://i.pravatar.cc/64?img=12"
            alt="avatar"
            className="h-8 w-8 rounded-full ring-1 ring-white/10"
          />
          <div className="min-w-0 flex-1 leading-tight">
            <div className="truncate text-sm font-medium text-foreground">Avery Chen</div>
            <div className="truncate text-xs text-muted-foreground">Operations · Acme</div>
          </div>
          <span className="h-2 w-2 rounded-full bg-success shadow-[0_0_8px_hsl(152_76%_44%)]" />
        </div>
      </div>

      <DashboardFormDialog
        open={formOpen}
        onOpenChange={setFormOpen}
        initial={editing}
        onSubmit={handleSubmit}
      />

      <Dialog open={Boolean(toDelete)} onOpenChange={(v) => !v && setToDelete(undefined)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Delete dashboard?</DialogTitle>
            <DialogDescription>
              “{toDelete?.name}” and its {toDelete?.widgets.length} widget(s) will be removed. This can’t be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setToDelete(undefined)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={confirmDelete}>
              <Trash2 /> Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </aside>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <div className="px-2 pb-1.5 text-[0.68rem] font-semibold uppercase tracking-wider text-muted-foreground/70">
        {title}
      </div>
      <div className="space-y-0.5">{children}</div>
    </div>
  );
}

function Item({
  d,
  active,
  onOpen,
  onEdit,
  onStar,
  onDelete,
}: {
  d: Dashboard;
  active: boolean;
  onOpen: () => void;
  onEdit: () => void;
  onStar: () => void;
  onDelete: () => void;
}) {
  return (
    <div
      onClick={onOpen}
      className={cn(
        "group/item relative flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-2 transition-all",
        active
          ? "bg-white/[0.06] text-foreground"
          : "text-muted-foreground hover:bg-white/[0.035] hover:text-foreground"
      )}
    >
      {active && (
        <span
          className="absolute left-0 top-1/2 h-5 w-[3px] -translate-y-1/2 rounded-full"
          style={{ background: `hsl(${d.accent})`, boxShadow: `0 0 10px hsl(${d.accent})` }}
        />
      )}
      <span
        className="grid h-7 w-7 shrink-0 place-items-center rounded-lg"
        style={{ background: `hsl(${d.accent} / 0.12)`, color: `hsl(${d.accent})` }}
      >
        <DashIcon name={d.icon} className="h-4 w-4" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{d.name}</div>
      </div>
      {d.starred && <Star className="h-3.5 w-3.5 fill-warning text-warning" />}
      <DropdownMenu>
        <DropdownMenuTrigger
          onClick={(e) => e.stopPropagation()}
          className="rounded-md p-1 opacity-0 transition-opacity hover:bg-white/10 focus:outline-none group-hover/item:opacity-100 data-[state=open]:opacity-100 cursor-pointer"
        >
          <MoreHorizontal className="h-4 w-4" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" onClick={(e) => e.stopPropagation()}>
          <DropdownMenuItem onClick={onEdit}>
            <Pencil /> Edit
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onStar}>
            <Star /> {d.starred ? "Unstar" : "Star"}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            onClick={onDelete}
            className="text-destructive focus:text-destructive [&>svg]:text-destructive"
          >
            <Trash2 /> Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
