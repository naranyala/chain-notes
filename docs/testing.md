# Testing

```sh
cargo test
```

52 tests, all below the UI boundary — no window is opened, no egui context
is built. They run in well under a second.

## The suites

### `note` (4 tests, `src/note.rs`)

The `Note` model and the time helpers.

| Test | Locks |
| --- | --- |
| `new_stamps_both_timestamps_and_starts_linkless` | Construction sets id/title, empty body, empty links, and equal `created_at`/`updated_at` |
| `touch_moves_updated_at_forward` | `touch()` advances `updated_at` |
| `fmt_time_buckets` | The 59 s / 60 s / 3599 s / 3600 s / 86399 s / 86400 s boundaries of the relative-time formatter |
| `fmt_time_never_panics_on_future_or_zero` | A future timestamp clamps to "0s ago"; epoch does not overflow |

### `graph` (5 tests, `src/graph.rs`)

Pure link-graph operations over `&[Note]` / `&mut [Note]` — no
`ChainNotes`, no filesystem.

| Test | Locks |
| --- | --- |
| `outgoing_and_backlinks_follow_the_edges` | Outgoing is per-note in link order; backlinks are derived across all notes; a missing id yields empty |
| `toggle_adds_then_removes_and_refuses_bad_endpoints` | Toggle is idempotent-inverse; self-links and missing endpoints are refused |
| `strip_records_positions_and_restore_puts_them_back` | Stripping records `(source, position)`; restoring re-inserts at that exact position |
| `restore_ignores_a_missing_source` | Restoring an edge whose source note is gone is a no-op, not a panic |
| `normalize_links_drops_dangling_and_self_edges` | Load-time invariant: links to nonexistent ids and self-links are removed |

### `history` (3 tests, `src/history.rs`)

The one-deep undo slot.

| Test | Locks |
| --- | --- |
| `starts_empty` | No slot, `take()` returns `None` |
| `record_then_take_returns_the_slot_once` | A recorded delete is retrievable exactly once |
| `a_second_record_replaces_the_first` | One-deep: a new delete replaces the previous slot |

### `state` (12 tests, `src/state.rs`)

The `ChainNotes` record: everything that reaches disk except the file
itself. Delegates link work to `graph` and undo to `history`.

| Test | Locks |
| --- | --- |
| `add_allocates_sequential_ids_and_selects` | Ids increment, the new note is selected, `next_id` advances, the status sentence is set |
| `delete_strips_outgoing_and_incoming_edges` | Deleting a note removes every edge pointing at it, in both directions |
| `delete_of_selected_reselects_first` | Deleting the open note selects the first remaining one |
| `delete_missing_id_is_a_noop` | A bad id changes nothing and creates no undo slot |
| `undo_restores_note_and_its_incoming_edges` | Undo brings back the note *and* the backlinks, and selects it |
| `undo_restores_edges_at_their_original_positions` | An edge that was second in a list comes back second, not appended |
| `undo_without_a_slot_reports_nothing` | Undo with nothing to undo returns `None` and says so |
| `second_delete_replaces_the_undo_slot` | The slot is one-deep: only the most recent delete is restorable |
| `toggle_link_adds_then_removes_and_stamps` | Linking is a toggle; each change stamps the source note |
| `toggle_link_refuses_self_and_missing_endpoints` | Self-links and links to nonexistent ids are rejected |
| `search_matches_title_and_body_case_insensitively` | Search is case-insensitive over title and body; empty query shows all; no match returns empty |
| `to_record_round_trips_through_from_record` | The serde-bound snapshot round-trips through `from_record` |

### `store` (12 tests, `src/store.rs`)

The filesystem seam, exercised against scratch directories under the
system temp dir.

| Test | Locks |
| --- | --- |
| `missing_file_is_none_not_an_error` | A first run is `Ok(None)`, not an error |
| `round_trip_preserves_notes_and_selection` | Save then load returns the same notes, selection, `next_id`, and version |
| `corrupt_file_is_reported_not_panicked` | Unparseable JSON yields `StoreError::Corrupt`, not a panic |
| `empty_file_is_none` | A zero-byte file is a first run |
| `over_cap_is_too_large` | A file past 4 MiB is refused *before* reading |
| `failed_write_leaves_previous_file_intact` | A failed save does not clobber the last good file |
| `every_error_variant_has_a_distinct_sentence` | Each `StoreError` variant has its own wording, and a write failure never claims the file could not be *read* |
| `normalize_rebuilds_id_counter_and_drops_dangling_links` | Hand-edited input: `next_id` is raised past the max id, dangling and self links are dropped, a bogus `selected` is fixed |
| `merge_prefers_durable_unless_boot_is_strictly_newer` | The merge tiebreak at equal, older, and newer timestamps |
| `save_then_load_with_selected_none` | `selected: null` survives the round-trip |
| `default_location_resolves_under_the_user_data_dir` | The default path ends in `chainnotes/workspace.json` |
| `boot_wiring_restores_a_previously_saved_record` | The **boot path**, not just the merge rule: a saved record survives the same load+merge sequence `ChainNotesApp::new` performs |

### `autosave` (6 tests, `src/autosave.rs`)

The `AutoSaver` controller: debounce, flush, and the save-mode labels,
against a real scratch `WorkspaceStore`.

| Test | Locks |
| --- | --- |
| `labels_are_distinct_per_mode` | Every `SaveMode` has its own status-bar sentence (I2.3) |
| `touch_marks_dirty_and_restarts_the_clock` | `touch()` flips the mode to `Dirty` and marks the saver dirty |
| `flush_writes_the_record_and_reports_saved` | A dirty flush writes the file, reports `Ok`, flips to `Saved`, and a clean flush is a no-op |
| `flush_reports_a_write_failure_without_claiming_saved` | A failed write leaves the mode at `Failed` and the saver dirty for a retry |
| `maybe_flush_waits_for_the_debounce` | A long debounce defers the write; a zero debounce fires on the next call |
| `a_dirty_boot_starts_with_the_clock_running` | Constructing dirty (first run) still flushes via `maybe_flush` — the demo reaches disk without a user edit |

### `app` (8 tests, `src/app.rs`)

The boot wiring, the save-mode truthfulness (I2.3), and the command
write path. These run against a scratch `WorkspaceStore` through
`ChainNotesApp::at(store)` — the testable core of `new()`, which needs
an eframe `CreationContext` that tests cannot construct.

| Test | Locks |
| --- | --- |
| `first_run_flush_puts_the_demo_on_disk` | First run starts `Dirty` (the label says "Saving…", not "Saved to disk" for a file that does not exist yet), the first flush creates `workspace.json`, the mode flips to `Saved`, and a re-boot loads those demo notes |
| `boot_after_a_save_reports_saved_not_dirty` | A saved file boots as `Saved` with no pending write, the status sentence reports the note count, and the selection survives the round-trip |
| `boot_on_a_corrupt_file_leaves_the_disk_copy_untouched` | A corrupt file boots the demo in memory with mode `None` — and an idle exit (`flush` with nothing dirty) leaves the corrupt bytes on disk for the user to recover |
| `save_mode_labels_are_distinct` | Every `SaveMode` variant has its own status-bar sentence |
| `new_note_command_creates_selects_and_marks_dirty` | `Command::NewNote` through `handle()` creates a "Untitled" note, selects it, and dirties the saver |
| `delete_and_undo_commands_round_trip_through_handle` | `DeleteSelected` then `UndoDelete` through `handle()` restores the note and the selection |
| `toggle_link_command_edges_and_dirty_the_record` | A successful `ToggleLink` adds the edge and dirties; a refused self-link does not |
| `select_and_about_commands_do_not_dirty_the_record` | View-only commands (`SelectNote`, `About`) change state without marking a save |

The first two boot tests exist because of a bug caught during writing:
`new()` used to start the first run with `dirty = false` and
`save_mode = Saved`, so the status bar claimed "Saved to disk" while no
file existed, and the demo was never written until the user's first edit.
The command tests lock the single write path that every menu, panel, and
future shortcut goes through.

### `messages` (2 tests, `src/messages.rs`)

The status sentences.

| Test | Locks |
| --- | --- |
| `counts_use_plural_forms` | "Workspace is empty." / "Loaded 1 note." / "Loaded N notes." |
| `detail_sentences_embed_the_cause` | The parser/OS detail is interpolated into the sentence |

## What is not tested

* **The egui code itself** (`app.rs` menu/status drawing, `ui/*`). There
  is no widget or headless-egui test yet; the panels are drawn but never
  pumped in a test. The command *logic* (`handle()`) is covered; only the
  drawing and the panel→command emission wiring is not. A
  `egui_kittest`-style harness is the obvious next step, not yet done.
* **The debounce wall-clock timing.** The 250 ms constant is exercised via
  an injectable zero/long debounce (`AutoSaver::with_debounce`), not by
  fast-forwarding a clock; `maybe_flush` compares against
  `Instant::now()`.
* **A real end-to-end save through `eframe::App`.** `ChainNotesApp::new`
  needs a `CreationContext` whose fields are `pub(crate)` in eframe, so no
  test drives the app exactly as `run_native` does; `at(store)` is the
  same code path minus the store-location lookup.
* **Windows and macOS paths.** `dirs::data_dir()` is trusted for those
  platforms; only the Linux resolution is exercised here.
* **A screen-reader or contrast pass.** `accesskit` is compiled in by
  eframe's defaults, but nothing asserts on it (TODO-015).

## Adding a test

Put it in the `#[cfg(test)] mod tests` at the bottom of the module that
owns the behavior — `state.rs` for record logic, `graph.rs` / `history.rs`
for pure graph/undo rules, `autosave.rs` for debounce/flush, `store.rs`
for the file seam, `app.rs` for boot and the command write path. Tests
below the UI boundary need no window. If a test needs a scratch
directory, follow the existing `scratch(name)` helpers in `store.rs`,
`autosave.rs`, and `app.rs`: they build a unique path under
`std::env::temp_dir()` and remove any leftover from a prior run.
