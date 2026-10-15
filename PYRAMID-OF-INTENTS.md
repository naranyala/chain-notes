# Pyramid of Intents

This document defines why the Rust project exists, what it must enable, and
the constraints that should guide implementation decisions. `TODOS.md` is the
execution backlog; every backlog item must reference an intent ID from this
document.

The project is a Rust + egui reimagining of the desktop workspace in
`../dart-flutter-chainnotes` (Flutter). The port changes the technology and
narrows the scope, not the product shape: notes, an editor, and chains over
one persisted record. The Flutter original's other tools — TOC Manager, PDF
Reader, Image Viewer, Map Explorer — are deliberately out of scope for this
iteration; see I4.4.

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

PDF reading, image viewing, and map exploration from the Flutter original
are out of scope for this iteration; so is WebAssembly compilation. Building
any of them here is a product decision, not an oversight, and would need new
intents before new code. Recorded as decisions, not omissions — see
TODOS TODO-013 and TODO-014.

## Current state snapshot

Verified against the tree rather than against these entries, on 2026-10-15
with rustc 1.98.1 and eframe/egui 0.36.2 — `cargo build` clean,
`cargo clippy` clean (one `useless_format` warning found and fixed). Every
claim below was checked by reading the tree and building it on that date.

Already present and working:

- A runnable eframe window (1000×700, min 640×480) with a top menu bar
  (File: New note / Quit; Help: About), a bottom status bar, a left note
  list with live search, a central title+body editor, and a right chains
  panel — **I1.1**.
- The `Note` model (`src/note.rs`): id, title, body, directed `links`
  (outgoing edge ids), `created_at`/`updated_at`, `touch()` on edit, and a
  relative-time formatter — **I1.4**.
- Link management from the editor toolbar: a "Link…" popup toggles edges to
  other notes; the chains panel shows outgoing links and derived backlinks
  and navigates on click — **I1.4**, **I2.1**.
- Note create/delete with dangling-link cleanup (deleting a note strips its
  id from every other note's links) — **I1.4**, **I3.2**.
- 361 lines across 3 source files; the only external crates are eframe and
  egui.

Known gaps, in the order they should be closed (the full backlog with
definitions of done lives in `TODOS.md`):

- **Persistence does not exist.** Nothing reaches disk; a restart loses every
  note — **I1.3**, **I2.1**. TODO-001.
- **No automated tests.** The model, the link graph, and the app logic have
  zero coverage — **I4.1**. TODO-002.
- **No graph view.** Chains are lists, not a visual graph — **I1.4**
  (deferred by I4.4). TODO-003.
- **No import/export.** A workspace cannot be moved between machines —
  **I1.1**, **I2.2**. TODO-004.
- **Corrupt-file handling unproven.** The store does not exist yet, so its
  failure story is unverified — **I3.2**, **I2.2**. TODO-005.
- **No undo for delete.** A mis-click loses a note with no recovery —
  **I2.1**, **I2.2**. TODO-006.
- **No keyboard shortcuts.** New/delete/search are mouse-only — **I2.4**.
  TODO-007.

## Appendix — how the app is shaped: input → process → output

Every tool follows one pipeline: a gesture in a view, a decision in the app
state, a write through the single record, and a visible confirmation.
Nothing reaches disk except through the store, and nothing reaches the
filesystem except through that one seam (once TODO-001 lands).

| Tool | Input (gesture) | Process (where the decision lives) | Output (record + visible feedback) |
| --- | --- | --- | --- |
| Notes | "＋ New" button; Delete button; search keystroke; list-row click | `ChainNotesApp::add_note` / `delete_note` allocate the id, strip dangling links, set the selection and the status sentence; `visible_notes` filters by case-insensitive title+body substring | `notes` vec + `selected` + status line; sidebar rows and count |
| Editor | Title/body `TextEdit` change; "Link…" popup click | `Note::touch` stamps `updated_at`; the popup toggles an edge in `links` (add or remove) | `notes[i].title/body/links`; status line "Edited note #N" |
| Chains | Outgoing/backlink row click | Selecting a row sets `selected` to the target id | `selected`; the editor switches to that note |

Global flows:

```text
boot:    main() → NativeOptions (viewport, title) → run_native
         → ChainNotesApp::new (seeds two demo notes) → first ui() pass
         output: status "Ready"; persistence not yet wired (TODO-001)

keystroke: view → TextEdit::changed() → notes[i].touch()
           → status "Edited note #N"
           output: in-memory only until the store exists (TODO-001)

shutdown: eframe drops the app; no flush exists yet (TODO-001)
          output: edits since boot are lost
```

**One thing this snapshot exists to prevent.** The Flutter original's pyramid
carries a warning that a "current state" section once claimed a restart
restored work before the wiring that made it true existed. This snapshot
holds the same line for the Rust port: it lists the two demo notes that
`ChainNotesApp::new` seeds, the three source files that compile, and the
absence of persistence, tests, and a graph — nothing more. A document
describing intent can be aspirational; a document labelled *current state*
cannot, and it is the label that makes people stop checking.
