# Architecture

## The layering rule

The codebase is split at one line: **modules that decide what the record
contains never import egui.**

```text
┌─ decided by egui ───────────────────────────────────────────┐
│  main.rs    Composition root: NativeOptions, run_native     │
│  app.rs     ChainNotesApp: composition + handle(Command)    │
│  ui/        The three panel builders (free functions)       │
└─────────────────────────────────────────────────────────────┘
                │ owns / calls
┌─ decided without egui ──────────────────────────────────────┐
│  state.rs     ChainNotes: the record (notes/selection/ids)  │
│  graph.rs     Link-graph queries and invariants             │
│  history.rs   The one-deep undo slot                        │
│  autosave.rs  SaveMode + AutoSaver (store, debounce, flush) │
│  store.rs     WorkspaceStore: load, save, cap, merge        │
│  command.rs   The Command enum (the write path's vocabulary)│
│  messages.rs  Every user-facing status sentence             │
│  note.rs      The Note model and time helpers               │
└─────────────────────────────────────────────────────────────┘
```

A view that parsed a JSON file, invented a serialization format, or
reached past the store for shared state would cross this line. It does not.

There is a second seam inside the egui half: **panels never mutate the
record directly.** They read it through a borrowed `PanelCtx` and *emit*
`Command`s; the app shell drains them into one write path,
`ChainNotesApp::handle`. That is what lets a future shortcut or toolbar
reuse every mutation without re-implementing it.

## What each module owns

| Module | Owns | Depends on |
| --- | --- | --- |
| `main.rs` | Nothing persistent. Builds `NativeOptions` (window size, title) and hands `ChainNotesApp::new` to `eframe::run_native`. | `app` |
| `app.rs` | `ChainNotesApp`: the `ChainNotes` record, the `AutoSaver`, and the transient UI state (search text, popup). Plus the boot decision (`boot()`, the store-taking constructor `at()`), the single write path `handle(Command)`, and the `eframe::App` impl (menu, status bar, panel layout). | `state`, `autosave`, `command`, `store`, `messages`, `ui` |
| `ui/mod.rs` | `PanelCtx` — what a panel can see and touch this frame — and the `ui/` module wiring. | `command`, `state` |
| `ui/sidebar.rs` | The note list, live search box, and "＋ New" button. Emits `NewNote` / `SelectNote`. | `ui`, `command`, `messages` |
| `ui/editor.rs` | The title/body editor plus Delete / Undo / Link… actions. Text edits are bound directly; discrete actions are emitted as commands. | `ui`, `command`, `messages` |
| `ui/chains.rs` | Outgoing links and derived backlinks; row clicks emit `SelectNote`. | `ui`, `command`, `messages`, `note` (for `fmt_time`) |
| `state.rs` | `ChainNotes`: the `notes` vec, selection, id counter, and status sentence. Delegates link work to `graph` and undo to `history`. Every mutation that should persist goes through a method here. **No egui.** | `graph`, `history`, `store` (for `WorkspaceRecord`), `messages`, `note` |
| `graph.rs` | Pure operations on `&[Note]` / `&mut [Note]`: `outgoing`, `backlinks`, `toggle_link`, `strip_edges_to`, `restore_edges`, `normalize_links`. **No egui, no filesystem.** | `note` |
| `history.rs` | `History` / `DeletedSlot`: the one-deep undo slot (record a delete, take it back). **No egui.** | `note` |
| `autosave.rs` | `SaveMode` (with its status-bar `label()`) and `AutoSaver` — the store, the debounce clock, and `touch` / `flush` / `maybe_flush`. **No egui.** | `state` (for `to_record`), `store`, `messages`, `note` (for `now_secs`) |
| `store.rs` | `WorkspaceRecord` (the serde schema), `WorkspaceStore` (the only filesystem access), `StoreError`, and the boot `merge` rule. `normalize()` delegates link-invariants to `graph`. **No egui.** | `graph`, `messages`, `note`, `serde`, `serde_json`, `dirs` |
| `command.rs` | `Command` (the vocabulary of user intents) and `Effect` (side effects that need the egui context, e.g. `CloseWindow`). **No egui, no state.** | std only |
| `messages.rs` | Every user-facing status sentence, as constants or small functions with arguments. **No egui.** | std only |
| `note.rs` | `Note` (id, title, body, directed links, timestamps), `now_secs()`, `fmt_time()`. **No egui.** | `serde` |

## Who owns shared state

`ChainNotesApp` (in `app.rs`) is the only type that holds both the record
and the autosave controller. `ChainNotes` (in `state.rs`) owns what
reaches disk — the `notes` vec, the `selected` id, and the `next_id`
counter. `AutoSaver` (in `autosave.rs`) owns the store and the debounce
bookkeeping. The UI holds only what does not need saving: the search box
text and whether the link popup is open.

A piece of state that both need has one owner: `ChainNotes`. Panels read
it and call methods on it; they never reach into the store, and they never
mutate it except through a command (or an in-place `TextEdit` binding).

```text
ChainNotesApp
├── notes: ChainNotes              ← reaches disk (via to_record)
│   ├── notes: Vec<Note>
│   ├── selected: Option<u64>
│   ├── next_id: u64
│   ├── status: String             ← the status-bar sentence
│   └── history: History           ← the one-deep undo slot
│       └── slot: Option<DeletedSlot>
├── saver: AutoSaver               ← the only filesystem seam + debounce
│   ├── store: WorkspaceStore
│   ├── mode: SaveMode             ← status-bar right side
│   ├── dirty / last_edit          ← debounce bookkeeping
│   └── debounce: Duration
├── search: String                 ← transient
└── link_popup_open: bool          ← transient
```

## The command layer

Every user intent that changes the record is a `Command`
(`src/command.rs`). Menus and panels *emit*; `handle()` *runs*:

| Command | What `handle()` does |
| --- | --- |
| `NewNote` | `add_note("Untitled")` + `saver.touch()` |
| `DeleteSelected` | `delete_note(selected)` + `saver.touch()` |
| `UndoDelete` | `undo_delete()` (when there is a slot) + `saver.touch()` |
| `SelectNote(id)` | set `selected` (view-only, no save) |
| `ToggleLink { from, to }` | `toggle_link()` (when it succeeds) + `saver.touch()` |
| `NoteEdited` | `touch_selected()` + `saver.touch()` |
| `About` | set the status sentence (view-only) |
| `Quit` | `saver.flush()` + return `Effect::CloseWindow` |

This is the seam that makes TODO-007 (keyboard shortcuts) cheap: a
keymap is a `Key → Command` table; no new mutation logic. TODO-004
(import/export) becomes `Command::Import(path)` / `Export(path)` plus a
`handle` arm; TODO-003 (graph view) becomes a fourth panel that emits
`SelectNote`.

## What happens from launch to first paint

```text
main()
  └─ eframe::run_native("chainnotes", options, |cc| ChainNotesApp::new(cc))
       │
       ├─ WorkspaceStore::default_location()
       │    Some(path) → ChainNotesApp::at(store)
       │    None       → in-memory app, status = "No data directory…"
       │
       ├─ boot(&store)                                 ← testable core of new()
       │    Ok(Some(durable)) → merge(default_boot, durable)   ← I1.3 rule
       │                         status = "Loaded N notes."
       │                         save_mode = Saved (disk matches memory)
       │    Ok(None)           → WorkspaceRecord::demo()
       │                         status = "New workspace — no file yet."
       │                         save_mode = Dirty, dirty = true
       │                         (the first debounce writes the demo, so
       │                          "Saved to disk" never claims a missing file)
       │    Err(e)             → demo + e.sentence()            ← corrupt/over-cap
       │                         save_mode = None, dirty = false
       │                         (the unreadable file is left on disk
       │                          until the user actually edits)
       │
       ├─ ChainNotes::from_record(record, status)
       └─ AutoSaver::new(store, save_mode, first_run)
```

`new()` cannot be called from a test — eframe's `CreationContext` has
`pub(crate)` fields — so the decision logic lives in the free function
`boot()` and the store-taking constructor `at()`, both exercised directly
by the `app::tests` suite (see [`testing.md`](./testing.md)).

Then eframe calls `App::ui()` every frame:

```text
ui(ui, frame)
  ├─ saver.maybe_flush(&notes)  ← if dirty and ≥250 ms since last edit, save
  ├─ Panel::top("menu")         ← File / Help; buttons emit commands
  ├─ Panel::bottom("status")    ← sentence left, saver.mode().label() right
  ├─ Panel::left("sidebar")     ← panel(ui, ui::sidebar)
  ├─ Panel::right("chains")     ← panel(ui, ui::chains)
  └─ CentralPanel               ← panel(ui, ui::editor)
       where panel() builds a PanelCtx, runs the builder, then drains
       the emitted commands through handle()
```

## One frame of an edit

```text
keystroke in TextEdit
  → TextEdit::changed() reports true
  → ui/editor.rs emits Command::NoteEdited        (the TextEdit already
                                                   mutated the note's title/body)
  → app.rs panel() drains it → handle(NoteEdited)
      → notes.touch_selected()                    (state.rs stamps updated_at,
                                                   sets "Edited note #N.")
      → saver.touch()                             (autosave.rs: dirty = true,
                                                   mode = Dirty, last_edit = now)
  … 250 ms pass with no further edits …
  → maybe_flush() sees the deadline → flush()
      → notes.to_record(now_secs()) builds a WorkspaceRecord
      → store.save() writes temp file, renames over workspace.json
      → mode = Saved                              ("Saved to disk" on the right)
```

## Shutdown

`App::on_exit()` calls `saver.flush(&notes)` — a pending debounce is
written, never dropped. The Quit menu item runs `handle(Command::Quit)`,
which also flushes, before sending `ViewportCommand::Close`, so both exit
paths leave the file current.
