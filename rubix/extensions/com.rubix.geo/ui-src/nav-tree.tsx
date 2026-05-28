// TODO: implement with shadcn/ui
import * as React from "react";
import { BlockShell } from "@nube/starter-ext-sdk-ts";
import { EXTENSION_ID } from "./types";

const TREE = [
  { title: "Map", href: `/extensions/${EXTENSION_ID}` },
  { title: "Layers", href: `/extensions/${EXTENSION_ID}/layers` },
  { title: "Pins", href: `/extensions/${EXTENSION_ID}/pins` },
];

export default function NavTree(): React.ReactElement {
  return (
    <BlockShell>
      <nav className="mx-2 text-[0.8125rem] text-foreground">
        <div className="px-2 py-1 text-[0.7rem] font-semibold uppercase tracking-wider text-muted-foreground">
          Geo
        </div>
        <ul className="m-0 p-0 list-none">
          {TREE.map((item) => (
            <li key={item.href}>
              <a
                href={item.href}
                className="block py-1 px-2 pl-4 no-underline text-foreground rounded-md hover:bg-accent hover:text-accent-foreground transition-colors"
              >
                {item.title}
              </a>
            </li>
          ))}
        </ul>
      </nav>
    </BlockShell>
  );
}
