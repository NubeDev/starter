// `ui/nav-tree.tsx` — Sidebar nav-tree contribution for `com.rubix.example`.

import * as React from "react";

import { BlockShell } from "@nube/starter-ext-sdk-ts";
import { EXTENSION_ID } from "./types";
import { cn } from "./lib/utils";

interface NavLeaf {
  title: string;
  href: string;
}

interface NavBranch {
  title: string;
  children: NavLeaf[];
}

type NavItem = NavLeaf | NavBranch;

function isBranch(item: NavItem): item is NavBranch {
  return "children" in item;
}

const TREE: NavItem[] = [
  { title: "Overview", href: `/extensions/${EXTENSION_ID}/overview` },
  {
    title: "Customers",
    children: [
      { title: "By country", href: `/extensions/${EXTENSION_ID}/customers/by-country` },
      { title: "Quality issues", href: `/extensions/${EXTENSION_ID}/customers/quality` },
    ],
  },
  {
    title: "Products",
    children: [
      { title: "Low stock", href: `/extensions/${EXTENSION_ID}/products/low-stock` },
      { title: "Catalog", href: `/extensions/${EXTENSION_ID}/products/catalog` },
    ],
  },
];

export default function NavTree(): React.ReactElement {
  return (
    <BlockShell>
      <NavTreeInner />
    </BlockShell>
  );
}

function NavTreeInner(): React.ReactElement {
  return (
    <nav aria-label="Rubix Example" className="mx-2 text-[0.8125rem] text-foreground">
      <div className="px-2 py-1 text-[0.7rem] font-semibold uppercase tracking-wider text-muted-foreground">
        Rubix Example
      </div>
      <ul className="m-0 p-0 list-none">
        {TREE.map((item) =>
          isBranch(item) ? (
            <Branch key={item.title} branch={item} />
          ) : (
            <TopLeaf key={item.href} leaf={item} />
          ),
        )}
      </ul>
    </nav>
  );
}

function TopLeaf({ leaf }: { leaf: NavLeaf }): React.ReactElement {
  return (
    <li>
      <a
        href={leaf.href}
        className="block py-1 px-2 pl-4 no-underline text-foreground rounded-md hover:bg-accent hover:text-accent-foreground transition-colors"
      >
        {leaf.title}
      </a>
    </li>
  );
}

function Branch({ branch }: { branch: NavBranch }): React.ReactElement {
  const [open, setOpen] = React.useState(true);
  return (
    <li>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        className="w-full flex items-center gap-1.5 py-1 px-2 bg-transparent border-0 text-foreground font-inherit cursor-pointer rounded-md text-left hover:bg-accent hover:text-accent-foreground transition-colors"
      >
        <Chevron open={open} />
        <span>{branch.title}</span>
      </button>
      {open ? (
        <ul className="m-0 pl-5 list-none border-l border-border ml-4">
          {branch.children.map((leaf) => (
            <li key={leaf.href}>
              <a
                href={leaf.href}
                className="block py-1 px-2 no-underline text-foreground/85 rounded-md hover:bg-accent hover:text-accent-foreground transition-colors"
              >
                {leaf.title}
              </a>
            </li>
          ))}
        </ul>
      ) : null}
    </li>
  );
}

function Chevron({ open }: { open: boolean }): React.ReactElement {
  return (
    <svg
      width="10"
      height="10"
      viewBox="0 0 10 10"
      aria-hidden="true"
      className={cn(
        "shrink-0 opacity-70 transition-transform duration-150",
        open && "rotate-90",
      )}
    >
      <path d="M3 1.5 L7 5 L3 8.5" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
