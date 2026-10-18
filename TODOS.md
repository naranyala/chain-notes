# TODOs

This is the implementation backlog for the intents in
[`PYRAMID-OF-INTENTS.md`](./PYRAMID-OF-INTENTS.md). Every item has an intent
reference, a priority, and a definition of done. Items closed during a
session are kept at the bottom **with evidence**: they stay here rather than
moving to a changelog, so a reader can verify a claim without leaving this
file.

Priority levels:

- **P0** — blocks the intended happy path or makes the project misleading.
- **P1** — important for a dependable project.
- **P2** — improves maintainability, portability, or teaching value.
- **P3** — optional polish or a future extension.

## P0 — Make the documented product work end to end

No open items. TODO-001, TODO-002 are closed with evidence below.

## P1 — Make the workspace dependable

Open: TODO-003, TODO-004.
Closed: TODO-005, TODO-006 (evidence below).

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

### TODO-007 — Add keyboard shortcuts

- **Intent:** I2.4, I2.1
- **Priority:** P1
- **Work:** Bind at least: Ctrl/Cmd+N new note, Ctrl/Cmd+S force-save,
  Ctrl+D delete the selected note (guarded against firing while a
  `TextEdit` has focus), Ctrl+F focus search. Use egui's shortcut handling
  so the bindings are one table, not scattered `input.key_pressed` checks.
- **Done when:** Each shortcut works from any pane, and none fires while
  typing in the editor. Locked by an input-simulation test where feasible,
  else a documented manual check.

### TODO-016 — Preserve editor scroll position across note switches — DONE

- **Intent:** I2.1, I3.1
- **Priority:** P1
- **Work:** Treat the editor's scroll position as transient UI state owned by
  the app, with a clear policy for switching notes (at minimum, restore the
  position the user had when returning to a note). Do not put view state in
  the persisted workspace record unless the product intent changes.
- **Done when:** Scrolling a note, switching to another note, and returning
  restores the original editor position without affecting the saved record.
  The editor now keys egui's transient scroll state by note id, so each note
  has its own viewport and no scroll state enters the persisted record.

## P2 — Keep it maintainable and portable

Open: TODO-009.
Closed: TODO-008, TODO-010, TODO-011, TODO-012, TODO-016, TODO-017, TODO-018, TODO-019 (evidence below).

### TODO-009 — Add a CI workflow

- **Intent:** I4.1, I4.2
- **Priority:** P2
- **Work:** A `.github/workflows/ci.yml` running `cargo fmt --check`,
  `cargo clippy -- -D warnings`, `cargo build`, and `cargo test` on a pinned
  stable toolchain, with a README badge. Optionally a release build.
- **Done when:** A green run on `main`; a breaking change fails the run.
  The workflow is implemented in `.github/workflows/ci.yml`: it runs
  formatting, clippy with warnings denied, build, and test on Ubuntu using
  Rust 1.98.1. The first green run on `main` remains to be observed.

### TODO-010 — Write the README — DONE

- **Intent:** I1.5, I2.4, I4.2, I3.3
- **Priority:** P2
- **Work:** The README exists and covers quick start, the three tools, the
  architecture map, the persistence story and its power-loss limit, the
  supported platforms, and the rustc/edition assumption. Refresh it to name
  the new modules (`state.rs`, `store.rs`, `messages.rs`, `ui.rs`) and the
  now-working persistence, so no claim lags the tree.
- **Done when:** A contributor can go from clone to running app and saving
  notes using only the README; the repository map lists every source file.
  The README and development/testing guides now describe the pinned toolchain,
  CI commands, current test count, persistence behavior, platform status, and
  all source modules.

### TODO-012 — Document and check the toolchain and platform assumptions — DONE

- **Intent:** I4.2, I3.3
- **Priority:** P2
- **Work:** Pin the MSRV (or state "latest stable") in `Cargo.toml`/README;
  state the edition (2024) requirement; list the platforms that are actually
  built here (Linux verified, Windows/macOS declared but unbuilt) and check
  them in CI where a runner exists.
- **Done when:** The README names exactly the platforms that build, and the
  toolchain assumption is checked, not just stated. `rust-toolchain.toml`
  pins Rust 1.98.1 plus rustfmt/clippy; README and development docs state
  Linux is verified locally and in CI while Windows/macOS remain unbuilt.

### TODO-017 — Close the direct `TextEdit` mutation exception — DONE

- **Intent:** I1.2, I3.1, I4.3
- **Priority:** P2
- **Work:** Reconcile the command-layer invariant with the editor's current
  in-place `TextEdit` binding. Prefer staging title/body edits in UI state and
  applying them through an explicit command/state method; if egui requires a
  controlled exception, narrow and document that contract instead of claiming
  that every record mutation uses `handle(Command)`. Preserve timestamping,
  autosave, and status behavior.
- **Done when:** There is one documented mutation contract, editor edits obey
  it, and tests prove title/body changes still update `updated_at`, trigger
  autosave, and survive a round trip without allowing a panel to bypass the
  state boundary. `Command::NoteEdited` now carries staged text into
  `ChainNotes::edit_note`; the state test covers the field update and
  timestamping, while the existing app/autosave tests cover the write path.

## P3 — Optional polish and open decisions

Open: TODO-013, TODO-014, TODO-015.

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
  belong in chainnotes. Each is a product decision requiring new intents
  before code. Record the decision (in or out) in this file and the
  pyramid, so a reader is never left wondering.
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

### TODO-001 — Persist the workspace to disk — DONE

- **Intent:** I1.3, I2.1, I3.1, I3.2, I3.4
- **Priority:** P0
- **Evidence:** New `src/store.rs`: `WorkspaceRecord` (notes, selection,
  next id, schema version, `saved_at`), `WorkspaceStore` behind the single
  filesystem seam (I3.4) using `dirs::data_dir()/chainnotes/workspace.json`,
  a 4 MiB cap checked before reading, temp-file + rename writes, and
  `store::merge` applying the boot rule (durable wins unless strictly older
  than the in-memory snapshot). `ChainNotesApp` debounces at 250 ms
  (`touch_dirty`/`maybe_flush`), and `on_exit` flushes a pending write
  instead of dropping it (I1.3). The save mode is surfaced in the status bar
  (`SaveMode::Saved/Dirt/Failed/None`). Verified 2026-10-15: `cargo build`
  clean; the round-trip, corrupt-file, over-cap, and merge tests pass.

### TODO-002 — Test the model, the link graph, and the store — DONE

- **Intent:** I4.1, I3.2
- **Priority:** P0
- **Evidence:** 30 tests across four modules, all passing on 2026-10-15
  (`cargo test`: 30 passed, 0 failed). `note.rs` (4): construction, `touch`,
  `fmt_time` buckets, future/zero edge. `state.rs` (12): id allocation,
  delete stripping both edge directions, reselect-on-delete, missing-id
  no-op, undo restoring note + edges + edge *positions*, one-deep slot
  replacement, self/missing link refusal, search, record round-trip.
  `store.rs` (12): missing file, round-trip, corrupt, empty, over-cap,
  failed-write-leaves-previous-intact, error-sentence distinctness, normalize
  (id counter + dangling links + selection), merge tiebreak, default-location
  shape, and the boot *wiring* test (`boot_wiring_restores_a_previously_
  saved_record`) that fails against unwired boot and passes now — the exact
  lesson a merge-rule-only suite would miss. `messages.rs` (2): plural
  forms, cause embedding. `cargo clippy --all-targets` clean.

### TODO-005 — Bound the store's failure paths — DONE

- **Intent:** I3.2, I2.2, I3.4
- **Priority:** P1
- **Evidence:** `StoreError` is a five-variant enum (`TooLarge`, `Corrupt`,
  `Io`, `WriteFailed`, `RenameFailed`); every variant maps to a distinct
  user-facing sentence via `StoreError::sentence()` and `src/messages.rs`.
  Write-path failures are `WriteFailed` ("Could not save…"), not mislabeled
  as read failures; rename failures say the previous file is intact. No
  `unwrap`/`expect` on user-data paths (verified: `rg 'unwrap\(|expect\('`
  finds them only inside `#[cfg(test)]`). `every_error_variant_has_a_
  distinct_sentence` asserts each variant's wording and that a write failure
  never claims the file could not be *read*. `failed_write_leaves_previous_
  file_intact` proves a failed save does not clobber the last good file.

### TODO-006 — Undo a note deletion — DONE

- **Intent:** I2.1, I2.2
- **Priority:** P1
- **Evidence:** `ChainNotes` keeps a one-deep `DeletedSlot` holding the
  removed `Note` plus every `(source_id, position)` of an incoming edge that
  was stripped. `undo_delete` re-inserts the note at its old sort position
  and restores each edge at its recorded index; a second delete replaces the
  slot; undo with no slot reports `NOTHING_TO_UNDO`. UI: an "Undo delete"
  button in the editor toolbar (disabled when empty) and a File-menu entry.
  Locked by four tests: `undo_restores_note_and_its_incoming_edges`,
  `undo_restores_edges_at_their_original_positions`,
  `second_delete_replaces_the_undo_slot`, `undo_without_a_slot_reports_
  nothing`.

### TODO-008 — Split `app.rs` into focused modules — DONE

- **Intent:** I0.1, I1.2
- **Priority:** P2
- **Evidence:** The single 294-line `app.rs` is now seven files, every one
  under 300 production lines (test tails excluded): `main.rs` 24 (composition
  root), `app.rs` 231 (shell + persistence wiring), `ui.rs` 210 (the three
  panel builders as `impl ChainNotesApp`), `state.rs` 197 prod + 136 test
  (record + undo logic, no egui), `store.rs` 196 prod + 162 test (the
  filesystem seam), `messages.rs` 67 prod + 18 test, `note.rs` 50 prod +
  43 test. Each module has one responsibility named in the README
  repository map. `cargo build`, `cargo clippy --all-targets`, and
  `cargo test` (30/30) pass unchanged.

### TODO-011 — Give user-facing sentences one home — DONE

- **Intent:** I2.2, I1.5
- **Priority:** P2
- **Evidence:** New `src/messages.rs` holds every status sentence as a named
  constant or a small `fn` with an argument (`created`, `deleted`,
  `restored`, `edited`, `loaded`, `no_workspace`, `too_large`, `corrupt`,
  `unreadable`, `could_not_save`, `save_failed_rename`) plus the mode
  labels (`SAVED_TO_DISK`, `NOT_SAVED`, `SAVING`, `NOT_SAVED_SEE_STATUS`,
  `NO_DATA_DIR`). All status writes in `state.rs`, `app.rs`, `store.rs`
  (via `StoreError::sentence`), and `ui.rs` route through `msg::`. A grep
  over the view/app modules finds no user-facing status literal outside
  `messages.rs` — only button labels ("Delete", "New note") and panel
  headings ("Chains", "Outgoing:"), which are chrome, not sentences.
  Wording changes are one-line edits in `messages.rs`. Locked by
  `messages::tests`.

### TODO-018 — Add a monthly metadata calendar — DONE

- **Intent:** I1.6, I2.1, I1.5
- **Priority:** P2
- **Evidence:** Added widget-free UTC calendar helpers in
  `src/calendar.rs` with leap-year, month-shift, weekday, and Unix timestamp
  conversion tests. Added `src/ui/calendar.rs` as a derived monthly view in
  the Chains panel: previous/next month controls, Today, per-day updated /
  created counts, day selection, and note navigation. Calendar state is
  transient and keyed separately from the persisted `WorkspaceRecord`.
  Verified with `cargo test` (58 passing), formatting, and clippy with
  warnings denied.

### TODO-019 — Add view/edit modes and active-note PDF export — DONE

- **Intent:** I1.7, I2.2, I1.5
- **Priority:** P2
- **Evidence:** The editor now has explicit View and Edit modes; view mode
  renders read-only text and disables link editing, while edit mode uses the
  existing staged command path. File > Export active note as PDF… opens a
  native `rfd` save picker, writes the title, UTC created/updated dates, and
  wrapped body through the widget-free `src/export.rs` PDF writer, and reports
  cancellation or failure in the status bar. The exporter test passed and a
  rendered sample was visually checked with Poppler.
