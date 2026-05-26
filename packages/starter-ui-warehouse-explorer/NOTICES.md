# Third-party notices

## sql-studio

This package is a fork of [frectonz/sql-studio](https://github.com/frectonz/sql-studio),
licensed MIT.

- **Upstream commit pinned:** `1a0736055a4647c18d0be19347e4325007c7bd52`
- **Upstream license:** MIT (see https://github.com/frectonz/sql-studio/blob/main/LICENSE)

### Local edits

- Re-skinned via `src/theme.css` — shadcn CSS variables map to rubix
  semantic tokens. No component markup changes.
- Data layer (`api.ts`) dropped; replaced with typed hooks in
  `src/hooks/use-*.ts` against `@nube/rubix-client-react`.
- TanStack Router scaffolding (`main.tsx`, `routeTree.gen.ts`,
  `createRootRoute`, `createFileRoute`) removed; routes are exported
  as plain React components from `src/views/`.
- `theme.provider.tsx` dropped — the rubix shell owns theme; the
  `.dark` class on `<html>` continues to drive upstream's dark
  variants.
