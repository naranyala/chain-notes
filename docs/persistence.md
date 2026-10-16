# Persistence

Everything below is implemented in `src/store.rs`, with the debounce and
flush wiring in `src/autosave.rs` (driven by `handle(Command)` in
`src/app.rs`). The rules are also stated as intents in
[`../PYRAMID-OF-INTENTS.md`](../PYRAMID-OF-INTENTS.md) (I1.3, I3.2, I3.4).

## The file

`WorkspaceStore::default_location()` returns
`dirs::data_dir()/chainnotes/workspace.json`, which resolves to:

| Platform | Path |
| --- | --- |
| Linux | `$XDG_DATA_HOME/chainnotes/workspace.json`, defaulting to `~/.local/share/chainnotes/workspace.json` |
| macOS | `~/Library/Application Support/chainnotes/workspace.json` |
| Windows | `%APPDATA%\chainnotes\workspace.json` |

If `dirs::data_dir()` returns `None` — an unusual platform — the app stays
in memory and the status bar says so. It never guesses a path.

The store is the **only** module that touches this file (pyramid I3.4). If
you want to swap in a different backend, `WorkspaceStore` is the seam: it
has two methods, `load()` and `save()`.

## The schema

One JSON object, versioned:

```json
{
  "version": 1,
  "notes": [
    {
      "id": 1,
      "title": "Welcome",
      "body": "chainnotes keeps notes and the links between them.",
      "links": [],
      "created_at": 1760505600,
      "updated_at": 1760505600
    },
    {
      "id": 2,
      "title": "How chainnotes works",
      "body": "Link notes together to form chains.",
      "links": [1],
      "created_at": 1760505600,
      "updated_at": 1760505600
    }
  ],
  "selected": 1,
  "next_id": 3,
  "saved_at": 1760505600
}
```

* `version` — `SCHEMA_VERSION` (currently 1). Reserved for future
  migrations.
* `notes` — the full list; `links` holds the *outgoing* target ids only.
  Backlinks are derived at load, never stored.
* `selected` — the id of the note that was open; `null` if none.
* `next_id` — the id counter. On load, `normalize()` raises it past the
  maximum existing id if a hand-edited or migrated file set it too low, so
  a new note can never collide.
* `saved_at` — Unix seconds of the last save. This is the merge tiebreaker
  (see below).

## Loading

`load()` returns `Result<Option<WorkspaceRecord>, StoreError>`:

1. `fs::metadata` the file. Not found → `Ok(None)` — a first run, not an
   error. The app seeds the demo notes and marks them dirty, so the boot
   debounce (~250 ms) puts the file on disk; the status bar says "Saving…"
   until then, never "Saved to disk" for a file that does not exist.
2. If the file is larger than `MAX_WORKSPACE_BYTES` (4 MiB) →
   `Err(TooLarge)`. The cap is checked **before** reading, so a runaway
   file cannot exhaust memory.
3. Read it as UTF-8. An I/O error → `Err(Io)`.
4. An empty or whitespace-only file → `Ok(None)`.
5. Parse with `serde_json`. A parse error → `Err(Corrupt(detail))`.
6. `normalize()` the record: raise `next_id` past the max id, drop links
   that point at ids that no longer exist (including self-links), and fix
   a `selected` that points nowhere.

No step panics. Every failure is an enum variant with a user-facing
sentence (see [Failure sentences](#failure-sentences)). On a load *error*
the app starts from the demo in memory but is **not** marked dirty: the
unreadable file stays on disk for the user to recover, and the status
label says "Not saved yet" rather than claiming the disk matches memory.

## Saving

`save()` writes atomically:

1. `create_dir_all` the parent directory.
2. `serde_json::to_string_pretty` the record.
3. Write the bytes to `workspace.json.tmp` in the same directory, then
   `flush()`.
4. `fs::rename` the temp file over `workspace.json`.

Because the rename is atomic on POSIX and replaces the target on Windows,
a crash mid-save leaves either the old file or the new one — never a
half-written file. If any step fails, the previous file is untouched.

## The debounce and the flush

The UI does not write on every keystroke. A record mutation runs through
`handle(Command)` (see [`architecture.md`](./architecture.md#the-command-layer)),
which calls `AutoSaver::touch()` (`src/autosave.rs`): that sets a flag and
stamps `last_edit`. 250 ms later, with no further edits,
`AutoSaver::maybe_flush()` writes. A pending write is **flushed, not
dropped**, on both exit paths:

* `App::on_exit()` calls `saver.flush()` — the normal close.
* the Quit menu runs `handle(Command::Quit)`, which also flushes, before
  sending `ViewportCommand::Close`.

## The boot merge

On load, the app merges the durable file against an in-memory boot
snapshot:

```rust
let boot = WorkspaceRecord::default();   // saved_at = 0
let merged = store::merge(&boot, &durable);
```

`merge` returns the durable copy **unless** `durable.saved_at <
boot.saved_at` — i.e. the on-disk file wins unless it is strictly older
than what is already in memory. On a single-file store the boot side is
the empty default (`saved_at = 0`), so the file always wins; the rule is
applied rather than assumed, and it is the same rule a second source (say,
a boot cache written more often) would use.

The test `boot_wiring_restores_a_previously_saved_record` locks this: it
saves a record, simulates the boot path, and asserts the saved content
survives. Reverting to a boot snapshot stamped with the current time —
the bug a naive implementation ships — fails it.

## Failure sentences

Every `StoreError` variant maps to one distinct status line, all of them
in `src/messages.rs`:

| Variant | Sentence |
| --- | --- |
| `TooLarge { bytes, cap }` | "Workspace file is *N* bytes, over the *C* byte cap; starting empty." |
| `Corrupt(detail)` | "Workspace file is corrupt (*detail*); starting empty." |
| `Io(detail)` | "Could not read the workspace file (*detail*); starting empty." |
| `WriteFailed(detail)` | "Could not save the workspace (*detail*)." |
| `RenameFailed(detail)` | "Could not finish saving the workspace (*detail*); the previous file is intact." |

Write failures are deliberately not labeled as read failures, and a rename
failure tells the user their previous file is safe.

## The durability limit, stated honestly

The store **flushes** the temp file but does **not `fsync`** it, and does
not `fsync` the parent directory after the rename. Rust's `std::fs`
exposes no `fsync` on a file or directory, so this is a language-level
limit, not a shortcut.

What that means in practice:

* **Crash mid-write (process killed, app panics):** safe. You get the old
  file or the new one, never a torn file.
* **Power loss in the rename window:** the last write may be lost, even
  though the app reported "Saved to disk".

No document in this project claims power-loss durability. If you need it,
the store is the one place to change: swap the write path for a crate that
can `fsync` (for example, `fs4` or an `O_SYNC` open), and update this
paragraph.
