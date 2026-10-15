# TODOs

This is the implementation backlog for the intents in
[`PYRAMID-OF-INTENTS.md`](./PYRAMID-OF-INTENTS.md). Every item has an intent
reference, a priority, and a definition of done. Items closed later are kept
at the bottom **with evidence**: they stay here rather than moving to a
changelog, so a reader can verify a claim without leaving this file.

Priority levels:

- **P0** — blocks the intended happy path or makes the project misleading.
- **P1** — important for a dependable project.
- **P2** — improves maintainability, portability, or teaching value.
- **P3** — optional polish or a future extension.

## P0 — Make the documented product work end to end

Open: TODO-001, TODO-002.

### TODO-001 — Persist the workspace to disk

- **Intent:** I1.3, I2.1, I3.1, I3.2, I3.4
- **Priority:** P0
- **Work:** Introduce a `WorkspaceStore` (or `Storage` trait) that owns the
  only filesystem path: locate a data dir (e.g. `dirs::data_dir()`/
  `chainnotes/`), serialize the record (notes, selected id, next id) to
  JSON, debounce writes (~250 ms), write to a temp file then rename, cap the
  file size before reading (e.g. 4 MiB), and merge on boot (durable wins
  unless strictly older than the in-memory snapshot). Flush any pending
  debounce on exit instead of dropping it. Surface the save mode in the
  status bar. State the power-loss limit (flush, no fsync) in code and
  README. Add the minimal deps (`serde`, `serde_json`, and a data-dir crate
  or an explicit path).
- **Done when:** Type a note, quit, relaunch, and the note is still there
  with its links intact; kill the process mid-edit and the last flushed
  write survives; corrupt the file and the app starts with an empty
  workspace plus a readable status sentence instead of a panic. Locked by
  store and boot-merge tests.

### TODO-002 — Test the model, the link graph, and (once it exists) the store

- **Intent:** I4.1, I3.2
- **Priority:** P0
- **Work:** Add a `tests/` tree (or `#[cfg(test)]` modules) covering: note
  creation id-allocation, `touch()` updating `updated_at`, `fmt_time`
  buckets, link add/remove, delete stripping dangling edges in both
  directions, and `visible_notes` search (case-insensitive title + body).
  Once TODO-001 lands, also cover store round-trip, atomic overwrite, size
  cap, corrupt-file degrade, and boot merge.
- **Done when:** Each branch above has a test; `cargo test` is green on a
  clean checkout. Locked by the suite.

## P1 — Make the workspace dependable

Open: TODO-003–TODO-007.

### TODO-003 — Add a visual graph view of the chains

- **Intent:** I1.4, I4.4
- **Priority:** P1
- **Work:** Beyond the list-based chains panel, render the note graph: nodes
  = notes, directed edges = `links`. Lay them out (force-directed or a
  simple layered layout), highlight the selected note and its neighbors, and
  click a node to select that note. Keep it derived in-memory from the
  record — never a second source of truth.
- **Done when:** A user sees the whole chain structure at a glance, clicks a
  node to jump to that note, and the view updates when a link is added or a
  note deleted. Locked by a view-state test.

### TODO-004 — Import and export a workspace

- **Intent:** I1.1, I2.2, I3.4
- **Priority:** P1
- **Work:** Menu actions to export the record to a user-chosen `.json` and
  import one back, using the same schema as the on-disk store. Validate on
  import (reject non-workspace JSON with a sentence, clamp out-of-range
  ids, rebuild the id counter past the max). Distinguish "user cancelled the
  picker" from "the file could not be read" in the status bar.
- **Done when:** Export produces a file that re-imports with identical
  content; a bad file produces a clear sentence and leaves the workspace
  unchanged; a cancelled picker is silent. Locked by round-trip and reject
  tests.

### TODO-005 — Bound the store's failure paths

- **Intent:** I3.2, I2.2, I3.4
- **Priority:** P1
- **Work:** Make the store return `Result<_, StoreError>` for every I/O and
  parse step; map each error to a status sentence (unreadable, too large,
  corrupt JSON, rename failed). No `unwrap`/`expect` on user-data paths.
  Ensure a failed write never clobbers the last good file (temp+rename
  guarantees this — prove it with a test).
- **Done when:** Each error variant has a test that produces it and asserts
  the status sentence; `rg 'unwrap\(|expect\('` over the store is clean.
  Locked by store tests.

### TODO-006 — Undo a note deletion

- **Intent:** I2.1, I2.2
- **Priority:** P1
- **Work:** Keep the last deleted note (and the links stripped from others)
  in a one-deep undo slot; add an "Undo delete" action in the menu or status
  bar that restores the note at its old id and re-establishes the stripped
  edges. Clear the slot on the next delete.
- **Done when:** Delete a note, click Undo, and the note and its links
  return; delete two in a row and only the last is restorable. Locked by a
  model test.

### TODO-007 — Add keyboard shortcuts

- **Intent:** I2.4, I2.1
- **Priority:** P1
- **Work:** Bind at least: Ctrl/Cmd+N new note, Ctrl/Cmd+S force-save once
  the store exists, Ctrl+D delete the selected note (guarded against firing
  while a `TextEdit` has focus), Ctrl/F focus search. Use egui's shortcut
  handling so the bindings are one table, not scattered `input.key_pressed`
  checks.
- **Done when:** Each shortcut works from any pane, and none fires while
  typing in the editor. Locked by an input-simulation test where feasible,
  else a documented manual check.

## P2 — Keep it maintainable and portable

Open: TODO-008–TODO-012.

### TODO-008 — Split `app.rs` into focused modules

- **Intent:** I0.1, I1.2
- **Priority:** P2
- **Work:** `src/app.rs` is ~294 lines holding app state plus all three
  panels plus the editor in one `impl`. Extract along existing seams: the
  app state/record (`src/state.rs`), the sidebar, the editor, and the
  chains panel (`src/ui/sidebar.rs`, `src/ui/editor.rs`, `src/ui/chains.rs`),
  keeping `ChainNotesApp` as the composition root. The split is structural
  only — behavior is locked by the existing tests.
- **Done when:** No file exceeds ~300 lines, each extracted module has one
  responsibility named in the README repository map, and
  `cargo build`/`cargo test` pass unchanged.

### TODO-009 — Add a CI workflow

- **Intent:** I4.1, I4.2
- **Priority:** P2
- **Work:** A `.github/workflows/ci.yml` running `cargo fmt --check`,
  `cargo clippy -- -D warnings`, `cargo build`, and `cargo test` on a pinned
  stable toolchain, with a README badge. Optionally a release build.
- **Done when:** A green run on `main`; a breaking change fails the run.
  Locked by the workflow itself.

### TODO-010 — Write the README

- **Intent:** I1.5, I2.4, I4.2, I3.3
- **Priority:** P2
- **Work:** Quick start (`cargo run`, `cargo test`, `cargo clippy`), what
  the three tools are, the architecture map (which module owns what), the
  persistence story and its power-loss limit, the supported platforms, and
  the rustc/edition assumption. Keep every claim checkable against the tree.
- **Done when:** A contributor can go from clone to running app using only
  the README; the repository map lists every source file. Locked by a docs
  guard test that fails when a file is added but not listed.

### TODO-011 — Give user-facing sentences one home

- **Intent:** I2.2, I1.5
- **Priority:** P2
- **Work:** Status sentences ("Edited note #N", "Created note #N",
  "Deleted note #N", future "could not save", "workspace is corrupt") are
  written inline across the app state and views. Extract them into one
  module (`src/messages.rs`) as named constants and point every site at
  them, so wording changes are one-line edits and tests assert via the same
  constants.
- **Done when:** No user-facing sentence literal remains in the view
  modules; tests assert via the constants; full suite green.

### TODO-012 — Document and check the toolchain and platform assumptions

- **Intent:** I4.2, I3.3
- **Priority:** P2
- **Work:** Pin the MSRV (or state "latest stable") in `Cargo.toml`/README;
  state the edition (2024) requirement; list the platforms that are actually
  built here (Linux verified, Windows/macOS declared but unbuilt) and check
  them in CI where a runner exists.
- **Done when:** The README names exactly the platforms that build, and the
  toolchain assumption is checked, not just stated.

## P3 — Optional polish and open decisions

Open: TODO-013–TODO-015.

### TODO-013 — Compile to WebAssembly

- **Intent:** I4.4, I3.3
- **Priority:** P3
- **Work:** eframe supports wasm; target the browser with the same app, a
  web runner page, and a persistence story that swaps the file store for
  `localStorage`/IndexedDB behind the same seam. Declare web as supported
  only once it runs.
- **Done when:** `cargo build --target wasm32-unknown-unknown` succeeds and
  a browser smoke test opens, edits, and reloads with the note intact.
  Locked by a build job.

### TODO-014 — Revisit the deferred heavy tools

- **Intent:** I4.4, I1.1
- **Priority:** P3
- **Work:** Decide whether PDF reading, image viewing, or map exploration
  from the Flutter original belong in the Rust port. Each is a product
  decision requiring new intents before code. Record the decision (in or
  out) in this file and the pyramid, so a reader is never left wondering.
- **Done when:** Each of the three tools has an explicit in/out decision
  recorded.

### TODO-015 — Accessibility pass

- **Intent:** I2.2
- **Priority:** P3
- **Work:** eframe already wires accesskit (verified: `accesskit_unix`,
  `accesskit_winit` are in the dependency tree); verify screen readers
  announce the note list, the editor, and the chains panel with useful
  names; add labels to icon-only buttons; check contrast of the default
  theme's muted text.
- **Done when:** Each pane and action announces a name; no behavior change.
  Locked by a documented manual screen-reader check.

## Closed (with evidence)

None yet — this backlog was established on 2026-10-15 against the initial
skeleton (361 lines, 3 source files, `cargo build`/`cargo clippy` clean,
rustc 1.98.1, eframe/egui 0.36.2). Items close here with their evidence
rather than moving to a changelog, so a reader can verify a claim without
leaving this file.
