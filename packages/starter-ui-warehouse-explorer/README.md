# @nube/starter-ui-warehouse-explorer

Visual rebuild of the warehouse explorer. Pixel-equivalent to
[frectonz/sql-studio](https://github.com/frectonz/sql-studio) apart
from colours — we re-skin via CSS variables to rubix semantic tokens.

See [`NOTICES.md`](./NOTICES.md) for upstream attribution and the
list of local edits.

See the scope doc:
[`rubix/docs/scope/warehouse-explorer-visual-rebuild.md`](../../rubix/docs/scope/warehouse-explorer-visual-rebuild.md).

## Usage

```tsx
import { Explorer, SqlProvider } from '@nube/starter-ui-warehouse-explorer'
import '@nube/starter-ui-warehouse-explorer/theme.css'

<SqlProvider>
  <Explorer />
</SqlProvider>
```

The host must provide a `QueryClientProvider`. Theme follows the host
— rubix toggles `.dark` on `<html>` and that drives upstream's dark
variants.

## PR status

- **PR 1 (this commit):** scaffold with verbatim file copies, license
  headers, stub hooks, token-mapped `theme.css`. Typecheck passes;
  views render against stub data.
- PR 2: wire real hooks from `@nube/rubix-client-react`.
- PR 3: mount in rubix shell at `/admin/warehouse-explorer`.
- PR 4: repoint demo binary; delete old `starter-ui-ch-explorer`.
