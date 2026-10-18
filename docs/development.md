# Development

## Prerequisites

* The pinned Rust toolchain in [`../rust-toolchain.toml`](../rust-toolchain.toml):
  rustc 1.98.1 with rustfmt and clippy.
* A Linux display (X11 or Wayland). The default eframe features compile
  support for both. CI runs the format, clippy, build, and test checks on
  Ubuntu; it does not launch the native window.

## Commands

```sh
cargo run                   # build and start the app
cargo build                 # debug build only
cargo build --release       # optimized build
cargo test                  # run the 58 tests
cargo clippy --all-targets -- -D warnings  # lints, including test code
cargo fmt                   # format (check with --check)
```

The first `cargo build` takes a while: eframe's default features pull in
`wgpu` and its shader/compiler dependencies. Subsequent builds are
incremental and fast.

## Where the workspace file lands

The app writes to the platform's conventional data directory (see
[`persistence.md`](./persistence.md#the-file)). On Linux:

```sh
ls ~/.local/share/chainnotes/workspace.json
```

To start from scratch, delete that file — the app will seed the demo notes
again on next launch. To inspect what is on disk:

```sh
python3 -m json.tool ~/.local/share/chainnotes/workspace.json | less
```

Set `XDG_DATA_HOME` to redirect it (useful for testing without touching
your real notes):

```sh
XDG_DATA_HOME=/tmp/chainnotes-test cargo run
```

## Project layout

```text
src/main.rs      Composition root
src/app.rs       ChainNotesApp: composition, boot, handle(Command), eframe::App
src/ui/          The panel builders (mod, sidebar, editor, chains, calendar)
src/calendar.rs  UTC calendar arithmetic, no egui
src/export.rs    Single-note PDF writer, no egui
src/command.rs   The Command enum (the write path's vocabulary)
src/state.rs     ChainNotes: the record, no egui
src/graph.rs     Link-graph queries and invariants, no egui
src/history.rs   The one-deep undo slot, no egui
src/autosave.rs  SaveMode + AutoSaver (store, debounce, flush), no egui
src/store.rs     WorkspaceStore: the filesystem seam, no egui
src/messages.rs  Status sentences, no egui
src/note.rs      The Note model, no egui
docs/            This directory
PYRAMID-OF-INTENTS.md   The rules a change must respect
TODOS.md               The backlog
```

The line between the egui half (`main`, `app`, `ui`) and the rest is the
core/UI boundary: everything below it must never import egui. Within the
egui half, panels never mutate the record directly — they emit
`Command`s that `handle()` runs. See
[`architecture.md`](./architecture.md).

## Making a change

1. Identify the highest-level intent it serves in
   [`../PYRAMID-OF-INTENTS.md`](../PYRAMID-OF-INTENTS.md). If it conflicts
   with a higher intent, resolve the conflict in that document first.
2. If it is a backlog item, find it in [`../TODOS.md`](../TODOS.md) and
   check its definition of done.
3. If it changes user-facing wording, edit `src/messages.rs` — not the
   call sites.
4. Add or extend a test in the module you touched. Logic that lives below
   the UI boundary can be tested without a window.
5. Run `cargo clippy --all-targets` and `cargo test` before you commit.

## Common problems

**The build hangs or takes many minutes on the first run.**
That is `wgpu` compiling its shader stack. Let it finish; it is cached
afterward. If you never want it, the alternative is eframe's `glow`
(OpenGL) backend, but this project is only tested against the defaults.

**The app opens with two demo notes even though I had my own.**
The workspace file was not found — check
`~/.local/share/chainnotes/workspace.json` exists, or that `XDG_DATA_HOME`
is not pointing somewhere unexpected.

**The status bar says the file is corrupt.**
The JSON did not parse. The app starts empty rather than crashing and shows
the parser's message. Your previous file is still on disk (the app only
overwrites once you make an edit and the debounce fires), so you can
recover it by hand. If you were editing it by hand, check for a trailing
comma or an unquoted key.

**The status bar says the file is over the cap.**
The workspace exceeded 4 MiB (`MAX_WORKSPACE_BYTES` in `src/store.rs`).
The app will not read it. Split the notes or raise the cap deliberately —
the constant is the one place to change.

**Clippy warns after a dependency bump.**
`cargo update` can surface new lints. Run
`cargo clippy --all-targets --fix` and review the diff; do not blanket-
allow lints to make a warning disappear.
