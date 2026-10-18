# chainnotes

[![CI](https://github.com/naranyala/rust-egui-chainnotes/actions/workflows/ci.yml/badge.svg)](https://github.com/naranyala/rust-egui-chainnotes/actions/workflows/ci.yml)

**chainnotes is a small Rust/egui desktop workspace for notes that link to one
another.** Notes, editing, chains, and a monthly metadata calendar operate on
one shared record saved to a single JSON file.

All the logic that decides what the record contains (the note model, the
link graph, the atomic store, the persistence merge) lives in widget-free
modules with tests. The egui views only draw.

## Quick start

```sh
cargo run
```

On first run the app seeds two linked demo notes and writes them to the
conventional per-user data directory. Edits are debounced 250 ms, written
atomically (temp file + rename), and a pending write is flushed — not
dropped — when the window closes.

Requires the pinned Rust toolchain in [`rust-toolchain.toml`](rust-toolchain.toml):
rustc **1.98.1**, with edition 2024 support, rustfmt, and clippy.

```sh
cargo build                # debug build
cargo test                 # 58 tests
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings # lints, warnings are errors
```

## What you get

| Pane | What it does |
| --- | --- |
| **Notes** (left) | The list. Create, delete, live-search across titles and bodies. |
| **Editor** (center) | The active note in read-only View mode or staged, editable Edit mode. |
| **Chains** (right) | Outgoing links and derived backlinks for the selected note; click a row to jump. |
| **Calendar** (right, below Chains) | Browse months, see created/updated note counts per day, and click a day or note to navigate. |

The editor has explicit **View** and **Edit** modes. File > **Export active
note as PDF…** opens a native save picker and exports the note title, UTC
created/updated dates, and wrapped body text.

* **Links are directed edges.** A note stores the ids of the notes it points
  to; backlinks are derived, never stored twice. Deleting a note strips its
  id from every other note so nothing dangles — and an undo restores the
  note *and* the links that pointed at it, at their original positions.
* **One record, one file.** Everything that reaches disk is a single
  `WorkspaceRecord` written to `…/chainnotes/workspace.json`. The store
  checks a 4 MiB cap before reading, writes via temp-file + rename, and on
  boot merges so the on-disk copy wins unless it is strictly older than the
  in-memory snapshot.
* **Failures are sentences, not panics.** A corrupt or oversized file, a
  failed write, a missing data directory — each maps to a distinct status
  line. No user-data path calls `unwrap`.

## Dependencies

Six direct crates:

| Crate | Version | Why it is here |
| --- | --- | --- |
| [`eframe`](https://crates.io/crates/eframe) | 0.36 | The application framework: the native window, the event loop, and the bridge between egui and the platform. We use its `App` trait (`ui()` per frame, `on_exit()` to flush a pending save) and `NativeOptions` for the window size and title. It pulls in a rendering backend and windowing automatically — see below. |
| [`egui`](https://crates.io/crates/egui) | 0.36 | The immediate-mode GUI library itself: `Panel`, `TextEdit`, `ScrollArea`, `Popup`, `MenuBar`, and the rest of the widgets. eframe depends on it, but we list it directly because our code names `egui::` types. Both crates move in lockstep — keep them on the same version. |
| [`serde`](https://crates.io/crates/serde) | 1 (with `derive`) | Serialization framework. The `derive` feature lets `#[derive(Serialize, Deserialize)]` generate the JSON conversion for `Note` and `WorkspaceRecord`, so the on-disk schema stays in step with the Rust types. |
| [`serde_json`](https://crates.io/crates/serde_json) | 1 | The JSON encoder/decoder `serde` writes through. Used in `store.rs` for `to_string_pretty` on save and `from_str` on load. |
| [`dirs`](https://crates.io/crates/dirs) | 7 | Resolves the platform's conventional data directory so the workspace file lands where users expect: `$XDG_DATA_HOME` or `~/.local/share` on Linux, `~/Library/Application Support` on macOS, `%APPDATA%` on Windows. One call, `dirs::data_dir()`. |
| [`rfd`](https://crates.io/crates/rfd) | 0.17 | Opens the native save picker used by PDF export. |

### What eframe brings in

`eframe` is a facade; its **default features** decide the rest of the tree:

* **`wgpu`** — the GPU rendering backend (via `egui-wgpu`). This is why a
  debug build takes a while the first time: `wgpu` and its shader/compiler
  dependencies are large. The alternative is the `glow` (OpenGL) backend,
  enabled with `default-features = false, features = ["glow", …]`; we do
  not use it.
* **`accesskit`** — accessibility scaffolding (screen-reader support) is
  compiled in by default. We have not done a pass on it yet (TODO-015), but
  the plumbing is present.
* **`wayland` + `x11`** — both Linux display protocols, so the binary runs
  under either without a rebuild.
* **`default_fonts`** — egui's built-in font atlas, so no font files need
  to be shipped.
* **`web_screen_reader`** — only relevant if you build for WASM
  (TODO-013); inert on native.

If a smaller binary or a faster first build matters more than defaults,
the features are the knob — but the default set is what this project is
tested against.

## Architecture

```text
src/main.rs      Composition root: NativeOptions, run_native
src/app.rs       ChainNotesApp: composition, boot, and the single write
                 path handle(Command) for every record mutation
src/ui/          The panel builders as free functions over a
                 borrowed PanelCtx (mod, sidebar, editor, chains, calendar)
src/command.rs   The Command enum — the vocabulary of user intents
src/state.rs     ChainNotes: the record (notes, selection, ids, status)
src/graph.rs     Link-graph queries and invariants over &[Note]
src/calendar.rs  Dependency-free UTC calendar/date helpers for metadata views
src/export.rs    Widget-free single-note PDF export
src/history.rs   The one-deep undo slot
src/autosave.rs  SaveMode + AutoSaver: the store, debounce, and flush
src/store.rs     WorkspaceStore: the one filesystem seam — load, atomic
                 save, size cap, boot merge, StoreError
src/messages.rs  One home for every user-facing status sentence
src/note.rs      Note model: id, title, body, directed links, timestamps
```

Three rules hold the shape together:

* **The core/UI boundary.** `state.rs`, `graph.rs`, `calendar.rs`,
  `export.rs`, `history.rs`, `autosave.rs`, `store.rs`, `command.rs`,
  `messages.rs`, and `note.rs`
  contain no egui. Anything that decides whether a link resolves, how the
  record is shaped, or what survives a restart lives there, where it can
  be unit-tested without a window.
* **One record owns shared state.** `ChainNotesApp` owns a `ChainNotes`,
  which owns the `notes` vec, the selection, and the id counter — exactly
  what reaches disk. The UI holds only transient state (the search box
  text, whether a popup is open).
* **One write path.** Panels and menus emit `Command`s; `handle()` is the
  only place that dispatches record mutations. Editor text is staged in the
  widget and applied through `Command::NoteEdited`, while a future keyboard
  shortcut or toolbar reuses every mutation by emitting a command.

For the full picture — layering, the boot sequence, and the persistence
contract — see [`docs/architecture.md`](docs/architecture.md) and
[`docs/persistence.md`](docs/persistence.md).

## Where the file lives

`WorkspaceStore::default_location()` resolves to:

| Platform | Path |
| --- | --- |
| Linux | `$XDG_DATA_HOME/chainnotes/workspace.json`, defaulting to `~/.local/share/chainnotes/workspace.json` |
| macOS | `~/Library/Application Support/chainnotes/workspace.json` |
| Windows | `%APPDATA%\chainnotes\workspace.json` |

If the platform reports no data directory, the app stays in memory and says
so in the status bar rather than guessing a path.

## Documents

| Document | What you'll find |
| --- | --- |
| [`docs/README.md`](docs/README.md) | Index of the guides below |
| [`docs/overview.md`](docs/overview.md) | What the project is, how it feels to use, what it is not |
| [`docs/architecture.md`](docs/architecture.md) | The layers, what owns what, and what happens from launch to first paint |
| [`docs/persistence.md`](docs/persistence.md) | The store: the file format, the debounce, the merge rule, and the power-loss limit |
| [`docs/development.md`](docs/development.md) | Setup, commands, where the file lands, and fixes for common problems |
| [`docs/testing.md`](docs/testing.md) | The 58 tests, what each covers, and what is not tested |
| [`PYRAMID-OF-INTENTS.md`](./PYRAMID-OF-INTENTS.md) | The rules a change should respect, plus a checked snapshot of what works now |
| [`TODOS.md`](./TODOS.md) | What's left. Every item points at an intent and says what "done" means |

## Known gaps

What's still open lives in [`TODOS.md`](./TODOS.md), highest priority first.
Two things that might look unfinished are deliberate decisions, written
down so nobody re-litigates them:

* A visual **graph view** of the chains and **import/export** of a workspace
  file are not built yet — they are open backlog items (TODO-003, TODO-004),
  not omissions.
* Heavier tools (a PDF reader, an image viewer, a map) and a WebAssembly
  build are **out of scope for this iteration by decision**
  (pyramid I4.4, TODO-013/TODO-014).

## Notes

* **Durability, stated honestly:** the store writes via temp-then-rename
  and flushes but does **not** `fsync` the file or its directory. A crash
  mid-write is safe (you get the old file or the new one, never a torn
  file); a power cut in the rename window can still lose the last write.
  This limit is documented in `src/store.rs` and in
  [`docs/persistence.md`](docs/persistence.md).
* **Platforms:** Linux is built and verified locally and in CI. Windows and
  macOS are intended desktop targets but are not built in CI yet.
* **Accessibility:** eframe compiles AccessKit by default, but the manual
  screen-reader and contrast pass remains TODO-015.
