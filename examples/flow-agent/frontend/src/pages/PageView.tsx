// `/pages/:id` — read-only SDUI render of a saved page.
//
// Wraps the same `<Renderer>` the builder canvas uses inside the
// project-wide `<SduiHost>` shim so a saved tree round-trips through
// the exact same provider it was built under (SCOPE D3 / R4). The
// host's `dispatchAction` is a no-op, so interactive nodes don't
// reach out to any backend.

import { useNavigate, useParams } from "react-router-dom"
import {
  IconPencil,
  IconCopy,
  IconTrash,
  IconArrowLeft,
} from "@tabler/icons-react"

import { Renderer } from "@nube/starter-sdui-react"

import { Button } from "@/components/ui/button"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty"
import { SduiHost } from "@/lib/sdui-shim"
import { deletePage, getPage, savePage } from "@/lib/pages-store"
import { useDateFormatters } from "@/hooks/use-date-formatters"

export function PageView() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const dates = useDateFormatters()
  const page = id ? getPage(id) : undefined

  if (!page) {
    return (
      <div className="px-4 py-6 lg:px-6">
        <Empty className="border border-dashed bg-card/30">
          <EmptyHeader>
            <EmptyTitle>Page not found</EmptyTitle>
            <EmptyDescription>
              This page may have been deleted. Head back to the list.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
        <div className="mt-4">
          <Button variant="secondary" onClick={() => navigate("/pages")}>
            <IconArrowLeft className="size-4" />
            Back to pages
          </Button>
        </div>
      </div>
    )
  }

  function handleDuplicate() {
    if (!page) return
    const copy = savePage({
      name: `${page.name} (copy)`,
      tree: page.tree,
    })
    navigate(`/pages/${copy.id}`)
  }

  function handleDelete() {
    if (!page) return
    deletePage(page.id)
    navigate("/pages")
  }

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-2xl font-semibold tracking-tight">{page.name}</h2>
          <p className="text-xs text-muted-foreground">
            Saved {dates.dateTime(page.updatedAt)}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            onClick={() => navigate(`/pages/${page.id}/edit`)}
          >
            <IconPencil className="size-4" />
            Edit
          </Button>
          <Button variant="outline" onClick={handleDuplicate}>
            <IconCopy className="size-4" />
            Duplicate
          </Button>
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button
                variant="ghost"
                className="text-muted-foreground hover:text-destructive"
              >
                <IconTrash className="size-4" />
                Delete
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Delete this page?</AlertDialogTitle>
                <AlertDialogDescription>
                  This removes “{page.name}” from local storage. There is no
                  server copy.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction onClick={handleDelete}>
                  Delete
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </div>

      <div className="rounded-xl border border-border/60 bg-background p-4">
        <SduiHost>
          <Renderer node={page.tree.root} />
        </SduiHost>
      </div>
    </div>
  )
}
