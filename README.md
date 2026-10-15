# chainnotes

**Chainnotes is a Rust + egui desktop workspace for outline-driven notes:
three tools working over one shared record.**

It is a Rust reimagining of the Flutter workspace in
[`../dart-flutter-chainnotes`](https://github.com/naranyala/dart-flutter-chainnotes),
narrowed to the essence: notes, an editor, and chains (links between notes).
All correctness-critical logic — the note model, the link graph, and (once
it lands) the atomic store and persistence merge — lives in widget-free
modules with tests, while the egui views stay thin.

In daily use, you create notes in the **Notes** list, draft in the **Editor**
bound to the selected note, and link that note outward to other notes; the
**Chains** panel shows both outgoing links and derived backlinks and jumps
between them. Everything reads and writes a single record, so switching
notes never loses your place.

Chainnotes is built as a small, readable starter: one composition root, a
short dependency list (`eframe`, `egui`), and documents that match the code
that actually runs.

| Tool | What it is for |
| --- | --- |
| **Notes** | The list. Create, delete, live-search across titles and bodies. |
| **Editor** | A draft bound to the selected note. Title + body, edited in place. |
| **Chains** | Outgoing links and derived backlinks for the selected note; click to jump. |

## Quick start

```sh
cargo run
```

`cargo build` should come back clean and `cargo clippy` should pass —
checked on 2026-10-15 with rustc 1.98.1 (edition 2024) and eframe/egui
0.36.2. There is not a test suite yet (TODO-002); `cargo test` compiles
nothing but succeeds.

## What works now

Verified against the tree on 2026-10-15 — the full checked snapshot lives
in [`PYRAMID-OF-INTENTS.md`](./PYRAMID-OF-INTENTS.md):

* A runnable window (1000×700, min 640×480) with a menu bar, a status bar,
  a searchable note list, a title+body editor, and a chains panel.
* Notes are created with fresh ids; deleting one strips its id from every
  other note's links so nothing dangles.
* The "Link…" popup in the editor toggles directed edges to other notes;
  the chains panel shows outgoing links and backlinks, and navigates on
  click.
* Each note carries `created_at`/`updated_at`; the chains panel shows a
  relative timestamp.

## What does not work yet

* **Nothing is saved.** There is no persistence — a restart loses every
  note (TODO-001). The record is in-memory only.
* **No tests** cover the model or the link graph yet (TODO-002).
* **No import/export** (TODO-004), **no graph view** (TODO-003), **no
  keyboard shortcuts** (TODO-007), **no undo for delete** (TODO-006).

The ordered backlog with definitions of done lives in
[`TODOS.md`](./TODOS.md).

## Architecture

```text
src/main.rs      Composition root: NativeOptions, run_native
src/app.rs       ChainNotesApp: shared record + the three panels
                 (Notes sidebar, Editor, Chains), menu bar, status bar
src/note.rs      Note model: id, title, body, directed links, timestamps
```

* **One shared record.** `ChainNotesApp` owns the `notes` vec, the
  selection, and the id counter — what will reach disk. The UI holds only
  transient state (search text, open popup).
* **Chains are edges, not tags.** A link is a directed id stored on the
  source note; backlinks are derived, never stored twice
  (`src/app.rs:ui_chains`).
* **The core/UI boundary.** Anything that decides whether a link resolves,
  how the record is shaped, or what survives a restart belongs outside the
  widget closures — the store seam (TODO-001) will live beside `note.rs`,
  not inside `app.rs`.

## Documents

| Document | What you'll find |
| --- | --- |
| [`PYRAMID-OF-INTENTS.md`](./PYRAMID-OF-INTENTS.md) | The rules a change should respect, plus a checked snapshot of what works now and what doesn't |
| [`TODOS.md`](./TODOS.md) | What's left. Every item points at an intent and says what "done" means |

## Known gaps

What's still open lives in [`TODOS.md`](./TODOS.md), highest priority first —
if it's not there, it's either done or a deliberate decision. One item that
looks unfinished falls in the second camp, written down so nobody
re-litigates it:

* PDF reading, image viewing, and map exploration from the Flutter original
  are **out of scope for this iteration by decision** (pyramid I4.4,
  TODO-014). So is WebAssembly compilation (TODO-013). Building any of them
  is a product decision that needs new intents first, not an oversight.

## Notes

* **Toolchain:** edition 2024, built here with rustc 1.98.1. The only
  external crates are `eframe` and `egui` (0.36).
* **Platforms:** Linux is what gets built and verified here. Windows and
  macOS are intended desktop targets but unbuilt in this tree (TODO-012).
* **Durability, when it lands:** the planned store writes a JSON file via
  temp-then-rename and flushes but does **not** `fsync` the file or its
  directory, so the documented guarantee will be crash-safe, not
  power-loss-safe — the same honest limit the Flutter original states
  (pyramid I1.3).
* **Accessibility:** eframe already pulls in `accesskit`, so the scaffolding
  for screen readers is present; a real pass is TODO-015.
