// # PageContainer + PageHeader + PageBody
//
// Page-level layout primitives. Replaces the hand-rolled
// `<section className="relative mx-auto max-w-Nxl px-4 pb-24 pt-6 sm:px-6 lg:px-8">`
// pattern that was copy-pasted across every route with a single
// component whose width is a named token.
//
// Width tokens (named, not pixel values, not booleans):
//
// - `prose`   — `max-w-2xl`  long-form reading, single column
// - `narrow`  — `max-w-4xl`  settings, account pages, single-form
// - `default` — `max-w-6xl`  admin lists, tables
// - `wide`    — `max-w-7xl`  dashboards, marketing grids
// - `full`    — 100%, no max  canvases, editors, maps
// - `bleed`   — 100% + no horizontal padding (kiosk / embed / video)
//
// Padding tokens:
//
// - `default` — `px-4 sm:px-6 lg:px-8 pb-24 pt-6`
// - `compact` — `px-4 sm:px-6 py-4`
// - `none`    — no padding (use with `width="full"` for full-bleed canvases)
//
// Example:
//
//     <PageContainer width="narrow">
//       <PageHeader
//         eyebrow="Admin"
//         title="Users"
//         description="Manage accounts and permissions."
//         actions={<Button>New user</Button>}
//       />
//       <PageBody>{children}</PageBody>
//     </PageContainer>

import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "../../lib/utils"

const pageContainerVariants = cva("relative mx-auto w-full", {
  variants: {
    width: {
      prose: "max-w-2xl",
      narrow: "max-w-4xl",
      default: "max-w-6xl",
      wide: "max-w-7xl",
      full: "max-w-none",
      bleed: "max-w-none",
    },
    padding: {
      default: "px-4 pb-24 pt-6 sm:px-6 lg:px-8",
      compact: "px-4 py-4 sm:px-6",
      none: "",
    },
  },
  compoundVariants: [
    // `bleed` collapses padding by default — explicit `padding` still wins.
    { width: "bleed", padding: "default", className: "px-0 pb-0 pt-0" },
  ],
  defaultVariants: {
    width: "wide",
    padding: "default",
  },
})

export type PageWidth = NonNullable<
  VariantProps<typeof pageContainerVariants>["width"]
>
export type PagePadding = NonNullable<
  VariantProps<typeof pageContainerVariants>["padding"]
>

export interface PageContainerProps
  extends React.ComponentProps<"section">,
    VariantProps<typeof pageContainerVariants> {
  /** Render as a different element. Defaults to `<section>`. */
  asChild?: boolean
}

function PageContainer({
  className,
  width,
  padding,
  ...props
}: PageContainerProps) {
  return (
    <section
      data-slot="page-container"
      data-width={width ?? "wide"}
      data-padding={padding ?? "default"}
      className={cn(pageContainerVariants({ width, padding }), className)}
      {...props}
    />
  )
}

export interface PageHeaderProps
  extends Omit<React.ComponentProps<"header">, "title"> {
  /** Small accent label above the title (e.g. section name). */
  eyebrow?: React.ReactNode
  title: React.ReactNode
  description?: React.ReactNode
  /** Trailing slot — buttons, links, action dock. */
  actions?: React.ReactNode
}

function PageHeader({
  eyebrow,
  title,
  description,
  actions,
  className,
  ...props
}: PageHeaderProps) {
  return (
    <header
      data-slot="page-header"
      className={cn("mb-6 flex items-end justify-between gap-4", className)}
      {...props}
    >
      <div className="min-w-0">
        {eyebrow ? (
          <div className="flex items-center gap-3">
            <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
              {eyebrow}
            </span>
          </div>
        ) : null}
        <h1
          className={cn(
            "text-3xl font-medium tracking-[-0.03em] text-[color:var(--color-text)]",
            eyebrow ? "mt-3" : undefined,
          )}
        >
          {title}
        </h1>
        {description ? (
          <p className="mt-2 max-w-2xl text-sm text-[color:var(--color-muted)]">
            {description}
          </p>
        ) : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
    </header>
  )
}

function PageBody({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="page-body"
      className={cn("flex flex-col gap-6", className)}
      {...props}
    />
  )
}

export { PageContainer, PageHeader, PageBody, pageContainerVariants }
