//! The command layer: every user intent that changes the record.
//!
//! Menus, panels, and (future) keyboard shortcuts emit a `Command`; the app
//! shell translates it into a single write path (`ChainNotesApp::handle`).
//! Panels never mutate the record directly, so adding a shortcut or a
//! toolbar is a matter of emitting a command — not of re-implementing the
//! mutation.

/// A user intent that the app shell knows how to carry out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Command {
    /// Create a note titled "Untitled" and select it.
    NewNote,
    /// Delete the currently selected note (and its edges).
    DeleteSelected,
    /// Restore the most recent deletion.
    UndoDelete,
    /// Select the note with this id.
    SelectNote(u64),
    /// Toggle the directed edge `from -> to`.
    ToggleLink { from: u64, to: u64 },
    /// The selected note's text was edited in place.
    NoteEdited,
    /// Show the about line in the status bar.
    About,
    /// Flush a pending write and close the window.
    Quit,
}

/// A side effect a command can ask the shell to perform (things that need
/// the egui context, which the core does not have).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Effect {
    CloseWindow,
}
