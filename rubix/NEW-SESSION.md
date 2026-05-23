# NEW SESSION — read this first

You are starting a coding session on the **rubix** project. Before you
touch a single file, internalise the rules below. They are not style
preferences. They are load-bearing — breaking one creates work for the
next session (which is also you, with no memory of this one).

---

## 1. Read these, in this order

1. [HOW-TO-CODE.md](HOW-TO-CODE.md) — the contributor entry point.
   Hard rules, decision tree, crate map.
2. [SCOPE.md](SCOPE.md) — what rubix is and is not.
3. [docs/design/](docs/design/) — the canonical description of the
   system as it exists today. **This is the only doc tier code
   comments may reference.**

Do not start typing until you can answer:

- Which crate does my change belong in?
- Which design doc(s) describe the area I'm about to touch?
- Will my change require updating one of those design docs in the
  same PR?

If any answer is "I don't know", ask the user.

---

## 2. The non-negotiables

### Rule Zero — one responsibility per file

- ≤ 400 lines per file. ≤ 50 lines per function. ~10 public items per module.
- No `utils.rs`, `helpers.rs`, `common.rs`, `misc.rs`. Name the concept.
- If you're about to write more than ~150 lines into one new file,
  stop and split first.
- One **verb** per file. `user/create.rs`, `user/get.rs` — never
  one big `user.rs` doing every verb. See [FILE-LAYOUT.md](FILE-LAYOUT.md).

### Doc tiers — code comments reference `docs/design/` only

| Folder | Code may reference? |
|---|---|
| `docs/sessions/` | **Never.** Throwaway working notes. |
| `docs/scope/` | **Never.** Plans, not the system. |
| `docs/adr/` | Rarely — only to justify a non-obvious choice. |
| `docs/design/` | **Yes — the canonical reference.** |
| `HOW-TO-CODE.md`, `SCOPE.md`, `NEW-SESSION.md` | **Never.** Contributor meta-docs. |

If the design doc you need doesn't exist yet, **create it** as part of
the same PR. If a session note has settled into real code, promote it
to `docs/design/` per HOW-TO-CODE §0a, then update any references.

### Comments — present tense, why-not-what, no narrative

- Doc-comments (`///`) on every public item: purpose, defaults, edge cases.
- Explain *why*, not *what*.
- **No session-progress markers.** Forbidden:
  ```
  // Phase 0 only
  // STAGE-1 done
  // FIXED:
  // Previously this used X, now we use Y
  // Later phases will add Z
  ```
  Comments describe the code as it is **now**. If you need to record
  how we got here, that's an ADR.
- No emojis. No ASCII banners. No decorative comments.
- `// TODO(name): …` — never bare TODOs.
- Stale comment found → fix it in the same diff.

### Layer separation — transport carries zero logic

```
transport (REST / gRPC / CLI / MCP / SSE)
    ↓ calls
domain (pure business logic)
    ↓ calls
data (storage, external APIs)
```

Never the other way. No SQL in handlers. No HTTP in domain. No
clap types in domain.

Transport handlers do four things: **extract → call domain → shape DTO
→ return.** Twenty-line ceiling. Every transport file's first
doc-comment ends with `LAYER: transport.` See
[docs/design/layering/](docs/design/layering/README.md).

If you're asked to "add a check before saving", the wrong move is to
put it in the handler. The right move is to add it to the domain
function the handler calls.

### Smoke test

If you swap REST for gRPC tomorrow, how much of this file changes?
More than route wiring and DTO shaping → layering is wrong.

---

## 3. Workflow for this session

1. **Restate the task** in your own words. Confirm with the user.
2. **Locate the work** using the decision tree in HOW-TO-CODE §2.
3. **Read the relevant design docs** under `docs/design/`. If none
   exist, ask whether to draft one first.
4. **Plan before typing.** For multi-file work, write a brief todo list.
5. **Implement incrementally.** Small commits, one responsibility each.
6. **Tests live with the code.** Same PR, not later.
7. **Update design docs** in the same PR if behaviour changed.
8. **Do not commit** until the user explicitly asks.

---

## 4. When stuck — ask

Do not guess at:

- **Crate placement** — moving code later is expensive.
- **Trait seam changes** (anything in `*-spi`) — cascades to every consumer.
- **Feature-gate decisions** — accidental default-features pull deps consumers didn't ask for.
- **Design-doc structure** — better to ask "should this go in `AUTH/` or `EVENTS/`?" than to invent a new folder.

One sentence of "which of these two did you want?" beats two hours
of refactoring the wrong direction.

---

## 5. Hazards specific to AI sessions

- **Stale editor buffers.** If `read_file` shows your edit but
  `cargo`/`grep` see old content, verify on disk with a terminal
  command before chasing phantom errors.
- **Big-file temptation.** Generating a 1,200-line "complete solution"
  is a liability. Split before you finish.
- **Plan rot.** A session note from a previous session may describe
  intent that no longer matches reality. Trust `docs/design/` and the
  code over any session note.

---

## One-line summary

**Read HOW-TO-CODE → pick the right crate → consult `docs/design/`
→ small files, present-tense comments, tests alongside → update the
design doc in the same PR → commit only when asked.**
