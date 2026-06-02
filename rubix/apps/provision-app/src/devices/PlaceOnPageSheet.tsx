import { useEffect, useState } from 'react'
import { Loader2, LayoutDashboard } from 'lucide-react'
import { BottomSheet } from '../components/BottomSheet'
import { Field, Picker, TextInput } from '../components/FormKit'
import { PrimaryButton } from '../components/ui'
import { useToast } from '../components/toastContext'
import { useLook } from '../theme/useLook'
import { assignPage, pagesList } from '../api/bc'
import type { DeviceRow, PageRow } from '../api/bc-types'

// Place-on-page sheet for a pending (or pageless) device. Pick an existing page
// scoped to the device's site, or type a new page name → bc.assignPage, which
// generates the widgets and flips status to provisioned. Closes + refreshes
// (assignPage bumps the shared refresh signal) on success.
export function PlaceOnPageSheet({
  device,
  open,
  onClose,
}: {
  device: DeviceRow
  open: boolean
  onClose: () => void
}) {
  const look = useLook()
  const toast = useToast()
  const [pages, setPages] = useState<ReadonlyArray<PageRow>>([])
  const [pageId, setPageId] = useState('')
  const [newPage, setNewPage] = useState('')
  const [busy, setBusy] = useState(false)

  // Reload the site-scoped page list each time the sheet opens. Selection state
  // is reset in the async callback (not synchronously) to avoid cascading
  // renders; a stale closure can't leak because the picker re-options anyway.
  useEffect(() => {
    if (!open) return
    let live = true
    pagesList(device.site_id ?? undefined)
      .then((list) => {
        if (!live) return
        setPages(list)
        setPageId('')
        setNewPage('')
      })
      .catch(() => {
        if (live) setPages([])
      })
    return () => {
      live = false
    }
  }, [open, device.site_id])

  const ready = Boolean(pageId || newPage.trim())

  const submit = () => {
    if (!ready || busy) return
    setBusy(true)
    const input = pageId
      ? { device_id: device.device_id, page_id: pageId }
      : { device_id: device.device_id, new_page: { name: newPage.trim() } }
    assignPage(input)
      .then((r) => {
        toast.show(`Placed — ${r.widgets} tile${r.widgets === 1 ? '' : 's'}`, look.accent)
        onClose()
      })
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Could not place device', '#ff5a52'))
      .finally(() => setBusy(false))
  }

  return (
    <BottomSheet open={open} onClose={onClose} title="Place on page">
      <div className="flex flex-col gap-4">
        <p className="flex items-center gap-2 text-sm text-ink-variant">
          <LayoutDashboard className="h-4 w-4" style={{ color: look.accent }} />
          {device.name ?? device.device_id}
        </p>

        <Field label="Dashboard page">
          <Picker
            value={pageId}
            placeholder={pages.length ? '+ New page for this site' : '+ Create a page'}
            options={pages.map((p) => ({ value: p.page_id, label: p.name }))}
            onChange={(v) => {
              setPageId(v)
              setNewPage('')
            }}
          />
          {!pageId && (
            <TextInput
              value={newPage}
              onChange={setNewPage}
              onEnter={submit}
              placeholder="New page name (e.g. Floor 3 dashboard)"
              ariaLabel="New page name"
            />
          )}
        </Field>

        <PrimaryButton accent={look.accent} disabled={!ready || busy} onClick={submit}>
          {busy ? (
            <span className="inline-flex items-center justify-center gap-2">
              <Loader2 className="h-5 w-5 animate-spin" /> Placing…
            </span>
          ) : (
            'Place device'
          )}
        </PrimaryButton>
      </div>
    </BottomSheet>
  )
}
