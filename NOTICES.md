# Third-party notices

This file collects attribution for third-party source that has been
copied into this repository or that we redistribute alongside our own
code. The main project licence lives in [`LICENSE`](LICENSE) and is
unaffected by anything below.

---

## sql-studio (MIT)

Files derived from [`frectonz/sql-studio`](https://github.com/frectonz/sql-studio)
ship with a `// Forked from sql-studio (MIT)` header pointing back at
this notice. Derived paths:

- `crates/starter-warehouse/src/explorer/queries.rs`
- `crates/starter-warehouse/src/explorer/types.rs`
- `packages/starter-ui-ch-explorer/` (Vite + React app forked from
  the upstream `ui/` directory; per-file MIT headers, narrowed to
  ClickHouse and rewired against `/api/warehouse/ch/*`)

Original MIT licence text follows verbatim.

```
MIT License

Copyright (c) 2024 frectonz

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
