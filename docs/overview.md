# Overview

## What chainnotes is

A small desktop app for taking notes that link to each other. Three panes
over one saved record:

* **Notes** — the list. Create, delete, live-search across titles and
  bodies.
* **Editor** — a title and body bound to whichever note is selected.
* **Chains** — for the selected note, the notes it links to (outgoing) and
  the notes that link to it (backlinks). Click a row to jump there.

The point of the "chains" is that links are directed edges stored on the
source note, not tags on both ends. Backlinks are derived at display time,
so the graph is never stored twice and can never disagree with itself.

## How it feels to use

You open the app, and on the first run there are two demo notes already
linked to each other so you can see the shape of the thing. You type in the
editor; after about a quarter of a second the status bar's right side flips
from "Saving…" to "Saved to disk". You click "Link…" in the editor
toolbar, pick another note, and it appears in the Chains panel. You delete
a note by mistake, click "Undo delete", and it comes back — along with the
links that pointed at it, in their original positions.

Close the window. Reopen it. Everything is where you left it.

## What it is not

* **Not a knowledge graph tool.** There is no graph visualization, no
  embedding, no search engine. The chains are two lists in a side panel
  (a graph view is an open backlog item, not a design goal).
* **Not a sync service.** One JSON file, one machine. No cloud, no
  accounts, no conflict resolution — importing and exporting a file by hand
  is an open backlog item.
* **Not a plugin platform.** Five dependencies, a dozen small modules, one
  purpose. The architecture is meant to be readable end to end in an
  afternoon, not extended indefinitely.
* **Not yet portable beyond Linux.** The desktop targets Windows and macOS
  via the same code, but only Linux is built and verified in this tree.

## The shape of the code, in one paragraph

Fourteen source files, each with one job. `main.rs` is the composition
root. `app.rs` owns the shared record, the autosave controller, and the
single write path (`handle(Command)`) that every menu, panel, and future
shortcut goes through. `ui/` draws the three panels as free functions that
emit commands rather than mutating state. Below that line — and this is
the boundary that matters — `state.rs` (the record), `graph.rs` (the link
rules), `history.rs` (the undo slot), `autosave.rs` (debounce/flush),
`store.rs` (the filesystem seam), `command.rs`, `messages.rs`, and
`note.rs` contain no egui at all. That split is what lets the logic be
unit-tested without opening a window. See
[`architecture.md`](./architecture.md).

## Current state

Verified against the tree on 2026-10-16 with rustc 1.98.1 and eframe/egui
0.36.2: `cargo build` clean, `cargo clippy --all-targets` clean, `cargo
test` 52/52.

Working: persistence with debounce, atomic writes, a 4 MiB cap, boot merge,
and flush-on-exit — including a first run that writes the demo notes to
disk about a quarter second after launch, with the save-mode label
reporting only what is actually true; undo-delete that restores links at
their original positions; directed links with derived backlinks; live
search; distinct status sentences for every store failure.

Not built yet (see [`../TODOS.md`](../TODOS.md)): graph view, import/export,
keyboard shortcuts, CI, a platform/toolchain statement. Out of scope by
decision: PDF/image/map tools and a WebAssembly build.
