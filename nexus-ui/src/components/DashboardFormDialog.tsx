import { useEffect, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { DashIcon, ICON_NAMES } from "@/lib/icon";
import { cn } from "@/lib/utils";
import type { Dashboard } from "@/data/types";

const ACCENTS = ["152 76% 44%", "199 90% 56%", "263 80% 66%", "38 95% 56%", "346 84% 60%"];

export interface DashboardFormValues {
  name: string;
  description: string;
  icon: string;
  accent: string;
}

interface Props {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  initial?: Dashboard;
  onSubmit: (values: DashboardFormValues) => void;
}

export function DashboardFormDialog({ open, onOpenChange, initial, onSubmit }: Props) {
  const editing = Boolean(initial);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [icon, setIcon] = useState("Activity");
  const [accent, setAccent] = useState(ACCENTS[0]);

  useEffect(() => {
    if (open) {
      setName(initial?.name ?? "");
      setDescription(initial?.description ?? "");
      setIcon(initial?.icon ?? "Activity");
      setAccent(initial?.accent ?? ACCENTS[0]);
    }
  }, [open, initial]);

  const submit = () => {
    if (!name.trim()) return;
    onSubmit({ name: name.trim(), description: description.trim(), icon, accent });
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{editing ? "Edit dashboard" : "New dashboard"}</DialogTitle>
          <DialogDescription>
            {editing ? "Update the page details." : "Create a page — it appears in the sidebar instantly."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-1.5">
          <Label htmlFor="d-name">Name</Label>
          <Input
            id="d-name"
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Pump Station 4"
            onKeyDown={(e) => e.key === "Enter" && submit()}
          />
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="d-desc">Description</Label>
          <Input
            id="d-desc"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Optional subtitle"
          />
        </div>

        <div className="space-y-2">
          <Label>Icon</Label>
          <div className="flex flex-wrap gap-2">
            {ICON_NAMES.map((n) => (
              <button
                key={n}
                onClick={() => setIcon(n)}
                aria-label={n}
                className={cn(
                  "flex h-9 w-9 cursor-pointer items-center justify-center rounded-lg border transition-all",
                  icon === n
                    ? "border-primary/50 bg-primary/10 text-primary"
                    : "border-white/8 bg-white/[0.02] text-muted-foreground hover:border-white/20 hover:text-foreground"
                )}
              >
                <DashIcon name={n} className="h-4 w-4" />
              </button>
            ))}
          </div>
        </div>

        <div className="space-y-2">
          <Label>Accent</Label>
          <div className="flex items-center gap-2.5">
            {ACCENTS.map((a) => (
              <button
                key={a}
                onClick={() => setAccent(a)}
                aria-label={`accent ${a}`}
                className={cn(
                  "h-7 w-7 cursor-pointer rounded-full transition-transform hover:scale-110",
                  accent === a && "ring-2 ring-white/80 ring-offset-2 ring-offset-background"
                )}
                style={{ background: `hsl(${a})` }}
              />
            ))}
          </div>
        </div>

        <div className="mt-1 flex justify-end gap-2">
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={!name.trim()}>
            {editing ? "Save changes" : "Create dashboard"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
