// Shared uninstall dialog — used by both the `/extensions` list (kebab
// menu action) and the per-extension page header. Previews the cleanup
// manifest, then calls `DELETE /extensions/{id}?purge=true`.

import { useIntl } from 'react-intl'
import { Loader2, Trash2, X } from 'lucide-react'
import { Button } from '@nube/starter-ui-kit'
import {
  useExtensionCleanupPreview,
  useExtensionPurge,
  type ExtensionCleanupItem,
} from '@nube/rubix-client-react'

export function formatBytes(bytes?: number | null): string {
  if (bytes == null) return '—'
  const units = ['B', 'KB', 'MB', 'GB']
  let v = bytes
  let u = 0
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024
    u += 1
  }
  return `${v.toFixed(u === 0 ? 0 : 1)} ${units[u]}`
}

export function formatUptime(uptime?: { secs: number }): string {
  if (!uptime) return '—'
  const s = uptime.secs
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = s % 60
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m ${sec}s`
  return `${sec}s`
}

export function UninstallDialog({
  extId,
  onClose,
  onUninstalled,
}: {
  extId: string
  onClose: () => void
  onUninstalled?: () => void
}) {
  const intl = useIntl()
  const tr = (id: string, def: string) => intl.formatMessage({ id, defaultMessage: def })
  const preview = useExtensionCleanupPreview(extId)
  const purge = useExtensionPurge({
    onSuccess: () => {
      onClose()
      onUninstalled?.()
    },
  })

  const items = preview.data?.items ?? []
  // Dev mounts (loaded in-place from a source tree) only have their
  // data purged — the source files are preserved. We swap copy
  // throughout the dialog so the operator knows the working tree is
  // safe before confirming.
  const bundle = preview.data?.bundle
  const isDevBundle = bundle != null && bundle.will_delete === false

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <button
        type="button"
        aria-label={tr('common.close', 'Close')}
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
      />
      <div className="glass relative z-10 w-full max-w-lg rounded-3xl p-6">
        <div className="mb-4 flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-medium text-[color:var(--color-text)]">
              {tr('extensions.uninstall.title', 'Uninstall extension')}
            </h2>
            <p className="mt-1 font-mono text-[11px] text-[color:var(--color-subtle)]">{extId}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full p-1 text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]"
            aria-label={tr('common.close', 'Close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <p className="mb-3 text-sm text-[color:var(--color-muted)]">
          {isDevBundle
            ? tr(
                'extensions.uninstall.body_dev',
                'This purges the data listed below. The bundle is a dev mount — the source files on disk are preserved.',
              )
            : tr(
                'extensions.uninstall.body',
                'This permanently removes the bundle and the data listed below. This cannot be undone.',
              )}
        </p>

        {bundle != null && bundle.path !== '' ? (
          <div className="mb-3 rounded-2xl border border-[color:var(--color-border)]/60 px-3 py-2">
            <div className="text-[10px] uppercase tracking-wider text-[color:var(--color-muted)]">
              {tr('extensions.uninstall.source', 'Source location')}
            </div>
            <div className="truncate font-mono text-xs text-[color:var(--color-text)]">{bundle.path}</div>
            {isDevBundle ? (
              <div className="mt-1 inline-flex rounded-full bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-emerald-400 ring-1 ring-emerald-500/30">
                {tr('extensions.uninstall.dev_badge', 'Dev bundle — source files are safe')}
              </div>
            ) : null}
          </div>
        ) : null}

        <div className="mb-4 max-h-64 overflow-auto rounded-2xl border border-[color:var(--color-border)]/60">
          {preview.isLoading ? (
            <div className="flex items-center gap-2 p-4 text-sm text-[color:var(--color-subtle)]">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading…
            </div>
          ) : preview.isError ? (
            <p className="p-4 text-sm text-red-400">{preview.error.message}</p>
          ) : items.length === 0 ? (
            <p className="p-4 text-sm text-[color:var(--color-subtle)]">
              {tr('extensions.uninstall.nothing', 'No leftover data — only the bundle will be removed.')}
            </p>
          ) : (
            <ul className="divide-y divide-[color:var(--color-border)]/40">
              {items.map((item: ExtensionCleanupItem, i: number) => (
                <li
                  key={`${item.kind}-${item.label}-${i}`}
                  className="flex items-center justify-between gap-3 px-4 py-2"
                >
                  <div className="min-w-0">
                    <span className="inline-flex rounded-full bg-[color:var(--color-surface-2)]/60 px-2 py-0.5 text-[10px] uppercase tracking-wider text-[color:var(--color-muted)] ring-1 ring-[color:var(--color-border)]">
                      {item.kind}
                    </span>
                    <div className="truncate font-mono text-xs text-[color:var(--color-text)]">{item.label}</div>
                  </div>
                  <span className="shrink-0 text-[11px] text-[color:var(--color-subtle)]">{formatBytes(item.bytes)}</span>
                </li>
              ))}
            </ul>
          )}
        </div>

        {preview.data && preview.data.total_bytes > 0 ? (
          <p className="mb-4 text-xs text-[color:var(--color-subtle)]">
            {tr('extensions.uninstall.total', 'Total to reclaim')}: {formatBytes(preview.data.total_bytes)}
          </p>
        ) : null}

        {purge.isError ? <p className="text-sm text-red-400">{purge.error.message}</p> : null}

        <div className="flex justify-end gap-2">
          <Button size="sm" variant="outline" onClick={onClose} disabled={purge.isPending}>
            {tr('common.cancel', 'Cancel')}
          </Button>
          <Button
            size="sm"
            variant="default"
            disabled={purge.isPending || preview.isLoading}
            onClick={() => purge.mutate({ id: extId, purge: true })}
          >
            {purge.isPending ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Trash2 className="h-3.5 w-3.5" />}
            {isDevBundle
              ? tr('extensions.uninstall.confirm_dev', 'Purge data & disable')
              : tr('extensions.uninstall.confirm', 'Uninstall & purge')}
          </Button>
        </div>
      </div>
    </div>
  )
}
