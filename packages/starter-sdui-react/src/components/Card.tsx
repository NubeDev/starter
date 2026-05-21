/**
 * `card` — shadcn Card primitive. Optional `title` + `description`
 * render in the header; `children` go in the content section.
 */
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import { RendererList } from "../Renderer.js";
import type { UiComponent } from "../types.js";

export interface CardNode extends UiComponent {
  type: "card";
  title?: string;
  description?: string;
  children: UiComponent[];
}

export const cardSpec: ComponentSpec<CardNode> = {
  kind: "card",
  Component: ({ node }) => (
    <Card className={node.style?.className}>
      {node.title || node.description ? (
        <CardHeader>
          {node.title ? <CardTitle>{node.title}</CardTitle> : null}
          {node.description ? (
            <CardDescription>{node.description}</CardDescription>
          ) : null}
        </CardHeader>
      ) : null}
      <CardContent>
        <RendererList nodes={node.children ?? []} parentId={node.id} parentType="card" />
      </CardContent>
    </Card>
  ),
};
