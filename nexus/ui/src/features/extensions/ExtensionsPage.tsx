import { useRef, useState } from "react";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Switch } from "@nube/starter-ui-kit/components/switch";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@nube/starter-ui-kit/components/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@nube/starter-ui-kit/components/table";

import type {
  CleanupPreview,
  ContributesSummary,
  ExtensionSummary,
  LifecycleState,
} from "@/api/extensions/types";
import { usePrincipal } from "@/auth/usePrincipal";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import {
  useCleanupPreview,
  useDisableExtension,
  useEnableExtension,
  useInstallExtension,
  usePurgeExtension,
  useRestartExtension,
} from "@/features/extensions/useExtensionMutations";
import { useExtensions } from "@/features/extensions/useExtensions";

// The admin extensions screen (WS-14): every installed extension with its
// lifecycle state, enable/disable, restart, install-from-bundle and the
// dry-run-then-purge uninstall flow. The server admin-gates the API; we
// also gate the screen so a non-admin sees an honest notice rather than a
// failed request.
export function ExtensionsPage() {
  const principal = usePrincipal();

  if (principal.isPending) return <Loading label="Loading…" />;
  if (principal.isError) {
    return (
      <ErrorState
        message={
          principal.error instanceof Error ? principal.error.message : undefined
        }
      />
    );
  }

  if (principal.data?.role !== "admin") {
    return (
      <ErrorState
        title="Admin only"
        message="Extensions are managed by tenant admins."
      />
    );
  }

  return <ExtensionsAdmin />;
}

function stateVariant(
  state: LifecycleState,
): "default" | "secondary" | "destructive" {
  if (state === "failed") return "destructive";
  if (state === "running") return "default";
  return "secondary";
}

// Human size for the cleanup manifest's byte counts.
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = "B";
  for (const next of units) {
    if (value < 1024) break;
    value /= 1024;
    unit = next;
  }
  return `${value.toFixed(1)} ${unit}`;
}

// Non-zero contribution counts as small pills ("tools 3", "ui").
function contributePills(c: ContributesSummary | undefined): string[] {
  if (!c) return [];
  const counted: [string, number][] = [
    ["tools", c.tools],
    ["cli", c.cli],
    ["rest", c.rest],
    ["grpc", c.grpc],
    ["workers", c.workers],
    ["nodes", c.nodes],
    ["skills", c.skills],
  ];
  const pills = counted
    .filter(([, n]) => n > 0)
    .map(([label, n]) => `${label} ${n}`);
  if (c.ui) pills.push("ui");
  return pills;
}

function ExtensionsAdmin() {
  const query = useExtensions();
  const [installOpen, setInstallOpen] = useState(false);

  return (
    <div className="flex h-full flex-col gap-4">
      <header className="flex flex-wrap items-start justify-between gap-2">
        <div className="flex flex-col gap-1">
          <h1 className="text-lg font-semibold">Extensions</h1>
          <p className="text-sm text-muted-foreground">
            Installed extensions, their lifecycle and what they contribute.
          </p>
        </div>
        <Button onClick={() => setInstallOpen(true)}>Install</Button>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {query.isPending ? (
          <Loading label="Loading extensions…" />
        ) : query.isError ? (
          <ErrorState
            message={
              query.error instanceof Error ? query.error.message : undefined
            }
          />
        ) : query.data.length === 0 ? (
          <Empty
            title="No extensions installed"
            description="Install a .tar.gz bundle to add tools, workers, nodes or UI to this tenant."
          />
        ) : (
          <ExtensionTable extensions={query.data} />
        )}
      </div>
      <InstallDialog open={installOpen} onOpenChange={setInstallOpen} />
    </div>
  );
}

function ExtensionTable({ extensions }: { extensions: ExtensionSummary[] }) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Name</TableHead>
          <TableHead>Version</TableHead>
          <TableHead>Runtime</TableHead>
          <TableHead>State</TableHead>
          <TableHead>Enabled</TableHead>
          <TableHead>Restarts</TableHead>
          <TableHead>Contributes</TableHead>
          <TableHead className="text-right">Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {extensions.map((ext) => (
          <ExtensionRow key={ext.id} ext={ext} />
        ))}
      </TableBody>
    </Table>
  );
}

function ExtensionRow({ ext }: { ext: ExtensionSummary }) {
  const enable = useEnableExtension();
  const disable = useDisableExtension();
  const restart = useRestartExtension();
  const preview = useCleanupPreview();
  const purge = usePurgeExtension();
  // The cleanup manifest fetched on Uninstall; non-null opens the confirm.
  const [cleanup, setCleanup] = useState<CleanupPreview | null>(null);

  const enabled = ext.enabled === "enabled";
  const toggling = enable.isPending || disable.isPending;
  const pills = contributePills(ext.contributes);

  function onToggle(next: boolean) {
    if (next) enable.mutate(ext.id);
    else disable.mutate(ext.id);
  }

  function onUninstall() {
    preview.mutate(ext.id, { onSuccess: (manifest) => setCleanup(manifest) });
  }

  return (
    <TableRow>
      <TableCell>
        <div className="flex flex-col">
          <span className="font-medium">{ext.display_name ?? ext.id}</span>
          <span className="font-mono text-xs text-muted-foreground">
            {ext.id}
          </span>
        </div>
      </TableCell>
      <TableCell className="text-muted-foreground">
        {ext.version ?? "—"}
      </TableCell>
      <TableCell className="text-muted-foreground">
        {ext.runtime_kind ?? "—"}
      </TableCell>
      <TableCell>
        <div className="flex flex-wrap items-center gap-1">
          <Badge variant={stateVariant(ext.state)}>{ext.state}</Badge>
          {ext.restart_required ? (
            <Badge variant="outline">restart required</Badge>
          ) : null}
        </div>
      </TableCell>
      <TableCell>
        <Switch
          checked={enabled}
          disabled={toggling}
          onCheckedChange={onToggle}
          aria-label={`${enabled ? "Disable" : "Enable"} ${ext.id}`}
        />
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-1">
          <span>{ext.restart_count}</span>
          {ext.capability_violations > 0 ? (
            <Badge variant="destructive">
              {ext.capability_violations} violations
            </Badge>
          ) : null}
        </div>
      </TableCell>
      <TableCell>
        {pills.length === 0 ? (
          <span className="text-muted-foreground">—</span>
        ) : (
          <div className="flex flex-wrap gap-1">
            {pills.map((pill) => (
              <Badge key={pill} variant="outline">
                {pill}
              </Badge>
            ))}
          </div>
        )}
      </TableCell>
      <TableCell className="text-right">
        <div className="flex justify-end gap-2">
          {ext.runtime_kind === "process" ? (
            <Button
              variant="outline"
              size="sm"
              disabled={restart.isPending}
              onClick={() => restart.mutate(ext.id)}
            >
              {restart.isPending ? "Restarting…" : "Restart"}
            </Button>
          ) : null}
          <Button
            variant="destructive"
            size="sm"
            disabled={preview.isPending || purge.isPending}
            onClick={onUninstall}
          >
            {preview.isPending ? "Checking…" : "Uninstall"}
          </Button>
        </div>
        {preview.isError ? (
          <p role="alert" className="mt-1 text-xs text-destructive">
            Couldn't load the cleanup preview.
          </p>
        ) : null}
        <UninstallConfirm
          cleanup={cleanup}
          pending={purge.isPending}
          onCancel={() => setCleanup(null)}
          onConfirm={() => {
            purge.mutate(ext.id, { onSettled: () => setCleanup(null) });
          }}
        />
      </TableCell>
    </TableRow>
  );
}

// The dry-run-then-purge confirm: lists exactly what `DELETE …?purge=true`
// will remove (from the server's cleanup manifest) before the destructive
// action is offered.
function UninstallConfirm({
  cleanup,
  pending,
  onCancel,
  onConfirm,
}: {
  cleanup: CleanupPreview | null;
  pending: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <AlertDialog
      open={cleanup !== null}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Uninstall {cleanup?.id}?</AlertDialogTitle>
          <AlertDialogDescription>
            Purging removes everything below
            {cleanup && cleanup.total_bytes > 0
              ? ` (${formatBytes(cleanup.total_bytes)} total)`
              : ""}
            . This cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        {cleanup ? (
          <div className="max-h-64 overflow-y-auto">
            <ul className="flex flex-col gap-1 text-sm">
              {cleanup.items.map((item) => (
                <li
                  key={`${item.kind}:${item.label}`}
                  className="flex items-center gap-2"
                >
                  <Badge variant="outline">{item.kind}</Badge>
                  <span className="truncate">{item.label}</span>
                  {item.bytes != null ? (
                    <span className="ml-auto text-xs text-muted-foreground">
                      {formatBytes(item.bytes)}
                    </span>
                  ) : null}
                </li>
              ))}
              <li className="flex items-center gap-2">
                <Badge variant="outline">bundle</Badge>
                <span className="truncate font-mono text-xs">
                  {cleanup.bundle.path}
                </span>
                <span className="ml-auto text-xs text-muted-foreground">
                  {cleanup.bundle.will_delete ? "will delete" : "kept"}
                </span>
              </li>
            </ul>
          </div>
        ) : null}
        <AlertDialogFooter>
          <AlertDialogCancel disabled={pending}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            disabled={pending}
            onClick={(e) => {
              // Keep the dialog open until the purge settles.
              e.preventDefault();
              onConfirm();
            }}
          >
            {pending ? "Purging…" : "Purge everything"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

// Install-from-bundle dialog. The upload is a multipart `file` field; on
// success the extension is installed but only goes live after a server
// restart, so we surface that rather than pretending it's running.
function InstallDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const install = useInstallExtension();
  const fileRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);

  function reset() {
    setFile(null);
    install.reset();
    if (fileRef.current) fileRef.current.value = "";
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        onOpenChange(next);
        if (!next) reset();
      }}
    >
      <DialogContent className="glass">
        <DialogHeader>
          <DialogTitle>Install extension</DialogTitle>
          <DialogDescription>
            Upload a packaged extension bundle (.tar.gz).
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <input
            ref={fileRef}
            type="file"
            accept=".tar.gz,.tgz,application/gzip"
            className="block w-full text-sm text-muted-foreground file:me-3 file:rounded-md file:border-0 file:bg-secondary file:px-3 file:py-1.5 file:text-sm file:font-medium file:text-secondary-foreground"
            onChange={(e) => setFile(e.target.files?.[0] ?? null)}
          />
          {install.isSuccess ? (
            <p className="text-sm text-muted-foreground">
              <span className="font-medium text-foreground">
                {install.data.id}
              </span>{" "}
              {install.data.pending_restart
                ? "installed — live after restart."
                : "installed."}
            </p>
          ) : null}
          {install.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {install.error instanceof Error
                ? install.error.message
                : "Install failed."}
            </p>
          ) : null}
          <DialogFooter>
            <Button
              disabled={!file || install.isPending}
              onClick={() => {
                if (file) install.mutate(file);
              }}
            >
              {install.isPending ? "Installing…" : "Install"}
            </Button>
          </DialogFooter>
        </div>
      </DialogContent>
    </Dialog>
  );
}
