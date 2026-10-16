# Pyramid of Intents

This document defines why the Rust project exists, what it must enable, and
the constraints that should guide implementation decisions. `TODOS.md` is the
execution backlog; every backlog item must reference an intent ID from this
document.

The project is a small desktop workspace for notes that link to each
other: notes, an editor, and chains over one persisted record. Heavier
tools — a TOC manager, a PDF reader, an image viewer, a map explorer —
were considered and are deliberately out of scope for this iteration; see
I4.4.

## How to use this pyramid

- Keep the intent IDs stable. Add a new intent instead of silently changing
  the meaning of an existing one.
- When proposing work, identify the highest-level intent it serves.
- A TODO is valid only when it has an `Intent:` reference and a verifiable
  outcome.
- If a task conflicts with a higher-level intent, resolve the conflict here
  before implementing it.

## I0 — North-star intent

### I0.1 — Ship a readable Rust desktop workspace that is easy to run, read, and extend

Provide a small eframe/egui application in which the correctness-critical
logic — the note model, the link graph, the atomic store, the persistence
merge — lives in widget-free modules with tests, while the views stay thin
and a contributor can understand the data flow from one composition root.

**Success looks like:** a contributor can run `cargo build`, `cargo test`,
and `cargo clippy` on a clean checkout, open `src/main.rs`, and follow a
keystroke from a view to the shared state to the debounced write without
crossing a layering rule to do it.

## I1 — Product intents

### I1.1 — Three tools over one persisted record

Notes, Editor, and Chains are not isolated panes. One record owns the note
list, the selected note, each note's body, and the cross-note links (a note
pointing at other notes). The chains panel reads outgoing links and
backlinks from that same record; the editor mutates it; the sidebar lists
it.

**Success looks like:** a contributor can point at one store that owns the
shared state, and a user moves from a note to a linked note and back without
re-declaring or re-losing anything.

### I1.2 — Keep the core/UI boundary explicit

Anything that decides whether a link is valid, how state is clamped, what
survives a restart, or how an id maps to a note belongs in a non-UI module,
not in an egui widget closure. A view that parses a file itself, invents a
serialization format, or reaches past the store for shared state has crossed
this boundary without saying so.

### I1.3 — Durability a user can rely on

The workspace is written through one path: a debounce, a synchronous
write-to-temp then rename, a size cap checked before reading, and a merge on
boot in which the durable copy wins unless it is strictly older than the
in-memory snapshot. A pending debounce is flushed — not dropped — when the
app exits. A successful boot restore sets the save mode so the status bar
reads truthfully.

Power-loss limit, stated not implied: the file is flushed but NOT fsynced
before the rename, and the parent directory is NOT fsynced after it. A
crash-during-write is safe (old or new file, never half-written); power loss
in the rename window may still lose the last write.

**Success looks like:** type a sentence, close the window, reopen, and the
sentence is there; corrupt the file and the app still starts with a readable
explanation instead of a panic.

### I1.4 — Chains are a first-class concept, not a tag

A link is a directed edge between two note ids, stored on the source note.
Backlinks are derived, never stored twice. Deleting a note removes its
outgoing edges and every incoming edge that points at it. The chains panel
shows both directions. A visual graph view is a later intent (I4.4 defers
it), not a requirement of the first iteration.

**Success looks like:** link A→B, see B in A's outgoing list and A in B's
backlinks; delete B and A's outgoing list no longer dangles.

### I1.5 — Approachable as a starter

Favor clear code paths, a short dependency list, reproducible commands, and
documentation that matches the code that actually runs.

## I2 — User intents

### I2.1 — A user does not lose work when switching notes or restarting

The selected note, the edited body, and the scroll position survive a note
switch; the record survives a restart.

### I2.2 — A user receives useful feedback for invalid input

A missing title, an unreadable or corrupt workspace file, or a link that no
longer resolves must produce a clear, actionable sentence in the status bar
rather than silence or a panic.

### I2.3 — A user can trust the displayed state

The status bar, the note count, the outgoing/backlink lists, and the save
mode must reflect what actually happened. A label that claims work was not
saved when it was, or shows a link that no longer resolves, is a defect even
if nothing crashes.

### I2.4 — A developer can run and extend the project quickly

The README, the clippy lints, the test suite, and the build commands should
agree on the supported workflow.

## I3 — System intents

### I3.1 — One record owns shared state

The app state owns what reaches disk (the notes, the selection, the next
id); the UI holds only transient state (the search box text, the open
popup). A piece of state that both need must have one owner and one write
path.

### I3.2 — Failures are bounded and observable

The store returns a `Result` instead of panicking, a corrupt file degrades
to an empty workspace with a reported reason, and no failure path leaves a
half-written workspace file.

### I3.3 — Platform affordances are declared

The data directory, the supported desktop targets, and the toolchain
assumptions are stated in the README and, where the platform requires it, in
code — rather than assumed at runtime.

### I3.4 — File access sits behind one seam

`WorkspaceStore` (or a `Storage` trait) is the only place that touches the
filesystem for the workspace, so it can be swapped, faked, or bounded in one
edit.

## I4 — Quality and evolution intents

### I4.1 — Test behavior at each boundary

The note model, the link graph, the atomic store, the merge rules, and the
boot composition have proportionate automated verification.

### I4.2 — Make toolchain and platform assumptions explicit

The rustc/edition version, the platforms that are actually built, and the
dependency list should be documented and checked.

### I4.3 — Keep the architecture replaceable at the seams

The storage backend, the view modules, and the app state should be
changeable independently behind small contracts.

### I4.4 — Defer the heavy tools and web by decision

PDF reading, image viewing, and map exploration are out of scope for this
iteration; so is WebAssembly compilation. Building any of them here is a
product decision, not an oversight, and would need new intents before new
code. Recorded as decisions, not omissions — see TODOS TODO-013 and
TODO-014.

## Current state snapshot

Verified against the tree rather than against these entries, on 2026-10-16
with rustc 1.98.1 and eframe/egui 0.36.2 — `cargo build` clean,
`cargo clippy --all-targets` clean, `cargo test` 52/52 passing. Every claim
below was checked by running it on that date.

Already present and working:

- A runnable eframe window (1000×700, min 640×480) with a top menu bar
  (File: New note / Undo delete / Quit; Help: About), a bottom status bar
  with a save-mode label, a left note list with live search, a central
  title+body editor, and a right chains panel — **I1.1**.
- **Persistence** through one seam: `WorkspaceStore` (`src/store.rs`) owns
  the only filesystem path (`dirs::data_dir()/chainnotes/workspace.json`),
  checks a 4 MiB cap before reading, writes atomically (temp + rename), and
  applies the boot merge (`store::merge`: durable wins unless strictly
  older than the in-memory snapshot). `AutoSaver` (`src/autosave.rs`)
  debounces at 250 ms and `on_exit` flushes a pending write instead of
  dropping it. A first run starts the demo marked dirty, so the boot
  debounce writes the file instead of leaving the status bar claiming
  "Saved to disk" for a file that does not exist — **I1.3**, **I3.4**,
  **I2.1**, **I2.3**.
- **Bounded failures**: `StoreError` (TooLarge / Corrupt / Io /
  WriteFailed / RenameFailed) maps every failure to a distinct sentence via
  `messages.rs`; a corrupt or oversized file degrades to a readable status
  line, never a panic; no user-data path calls `unwrap` — **I3.2**,
  **I2.2**.
- The widget-free core: `state.rs` (the `ChainNotes` record), `graph.rs`
  (link-graph queries and invariants), `history.rs` (the undo slot),
  `autosave.rs` (debounce/flush), `store.rs`, `command.rs`,
  `messages.rs`, `note.rs` — **I1.2**.
- **One write path**: every record mutation enters through
  `ChainNotesApp::handle(Command)` (`src/app.rs`); panels and menus emit
  `Command`s (`src/command.rs`) rather than mutating state directly, so a
  future shortcut or toolbar reuses every mutation — **I1.2**, **I4.3**.
- **Undo delete**: a one-deep slot (`src/history.rs`) restores the deleted
  note *and* every incoming edge at its original position — **I2.1**.
- The `Note` model: id, title, body, directed `links`, `created_at`/
  `updated_at`, `touch()` on edit, relative-time formatting — **I1.4**.
- 52 tests across eight modules (note 4, graph 5, history 3, state 12,
  store 12, autosave 6, app 8, messages 2): id allocation, delete
  stripping both edge directions, undo + edge-position restore, search,
  record round-trip, corrupt/empty/over-cap loads, atomic-write failure
  leaving the previous file intact, error-sentence distinctness, the merge
  tiebreak, the boot wiring that restores a previously saved record, the
  save-mode truthfulness of first-run / loaded / corrupt boots, the
  debounce/flush controller, and the command write path — **I4.1**,
  **I2.3**.
- Fourteen source files (ten in `src/` plus the four in `src/ui/`), every
  one under 300 production lines — **I0.1**.

Known gaps, in the order they should be closed (the full backlog with
definitions of done lives in `TODOS.md`):

- **No graph view.** Chains are lists, not a visual graph — **I1.4**
  (deferred by I4.4). TODO-003.
- **No import/export.** A workspace cannot be moved between machines by
  file yet — **I1.1**, **I2.2**. TODO-004.
- **No keyboard shortcuts.** New/delete/save are mouse-only — **I2.4**.
  TODO-007.
- **CI and README refresh.** A workflow and a docs pass to keep every
  claim checkable — **I4.1**, **I1.5**. TODO-009, TODO-010.

## Appendix — how the app is shaped: input → process → output

Every tool follows one pipeline: a gesture in a view, a `Command` through
the single write path (`handle`), a decision in the app state, a write
through the single record, and a visible confirmation. Nothing reaches
disk except through the store, and nothing reaches the filesystem except
through that one seam.

| Tool | Input (gesture) | Process (where the decision lives) | Output (record + visible feedback) |
| --- | --- | --- | --- |
| Notes | "＋ New" button; Delete button; search keystroke; list-row click | Panels emit `Command::NewNote` / `DeleteSelected` / `SelectNote`; `handle()` calls `ChainNotes::add_note` / `delete_note` (which allocate the id, strip dangling links via `graph`, set the selection and the status sentence); `visible_notes` filters by case-insensitive title+body substring | `notes` vec + `selected` + status line; sidebar rows and count |
| Editor | Title/body `TextEdit` change; "Link…" popup click | `TextEdit` binds the fields directly; a change emits `Command::NoteEdited` (stamps `updated_at`); the popup emits `Command::ToggleLink` to add or remove an edge | `notes[i].title/body/links`; status line "Edited note #N" |
| Chains | Outgoing/backlink row click | Emits `Command::SelectNote` with the target id | `selected`; the editor switches to that note |

Global flows:

```text
boot:    main() → NativeOptions (viewport, title) → run_native
         → ChainNotesApp::new → WorkspaceStore::load
            Ok(Some) → merge(boot_default, durable) → "Loaded N notes."
            Ok(None) → demo record → "New workspace — no file yet."
                       marked dirty → first debounce writes the file
            Err(e)   → demo record → e.sentence() (corrupt/over-cap/unreadable)
                       not dirty → the unreadable file is left on disk
         output: save mode = Saved (loaded file), Dirty (first run, until
                 the boot flush lands), or None (load error: disk does not
                 match memory)

keystroke: view → TextEdit::changed() → Command::NoteEdited
           → handle() → state.touch_selected() → saver.touch()
           → 250 ms debounce → saver.flush()
           → store.save (temp + rename) → SaveMode::Saved | Failed
           output: status sentence kept; save-mode label in the status bar

shutdown: on_exit → saver.flush() (a pending write is flushed, not
          dropped); Quit menu runs handle(Command::Quit) which also flushes
          before ViewportCommand::Close
          output: the last edit survives a window close
```

**One thing this snapshot exists to prevent.** A "current state" section
once claimed a restart restored work before the wiring that made it true
existed — the claim was written from the intent's point of view, not the
code's. This snapshot holds the line: it claims persistence only because
`WorkspaceStore` exists, the boot path calls it, `on_exit` flushes it, and
tests lock the round-trip, the merge rule, and the boot wiring itself
(TODO-001 and TODO-002 are closed with that evidence in `TODOS.md`). It
does not claim a graph view, import/export, shortcuts, or a CI run,
because none of those exist. A document describing intent can be
aspirational; a document labelled *current state* cannot, and it is the
label that makes people stop checking.
