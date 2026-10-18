//! One home for every user-facing sentence.
//!
//! Views, the app state, and tests all point here, so a wording change is a
//! one-line edit (TODO-011). Delivery is the status bar only — there is no
//! second channel.

use std::fmt::Display;

pub const READY: &str = "Ready";
pub const ABOUT: &str = "chainnotes — a chained note taker";
pub const SELECT_OR_CREATE: &str = "Select or create a note.";
pub const NO_NOTE_SELECTED: &str = "No note selected.";
pub const NO_NOTES_MATCH: &str = "No notes match.";
pub const NO_OTHER_NOTES: &str = "No other notes to link.";
pub const NO_LINKS_YET: &str = "No links yet. Use \u{201c}Link\u{2026}\u{201d} in the editor.";
pub const NOTHING_TO_UNDO: &str = "Nothing to undo.";
pub const SAVED_TO_DISK: &str = "Saved to disk.";
pub const NOT_SAVED: &str = "Not saved yet.";
pub const SAVING: &str = "Saving\u{2026}";
pub const NOT_SAVED_SEE_STATUS: &str = "Not saved \u{2014} see status.";
pub const NO_DATA_DIR: &str = "No data directory on this platform; not saving.";
pub const EXPORT_CANCELLED: &str = "PDF export cancelled.";

pub fn exported(path: impl Display) -> String {
    format!("Exported active note to {}.", path)
}

pub fn export_failed(detail: impl Display) -> String {
    format!("Could not export the active note as PDF ({detail}).")
}

pub fn created(id: u64) -> String {
    format!("Created note #{id}.")
}

pub fn edited(id: u64) -> String {
    format!("Edited note #{id}.")
}

pub fn deleted(id: u64) -> String {
    format!("Deleted note #{id}.")
}

pub fn restored(id: u64) -> String {
    format!("Restored note #{id}.")
}

pub fn loaded(count: usize) -> String {
    match count {
        0 => "Workspace is empty.".to_owned(),
        1 => "Loaded 1 note.".to_owned(),
        n => format!("Loaded {n} notes."),
    }
}

pub fn no_workspace() -> String {
    "New workspace — no file yet.".to_owned()
}

pub fn too_large(bytes: u64, cap: u64) -> String {
    format!("Workspace file is {bytes} bytes, over the {cap} byte cap; starting empty.")
}

pub fn corrupt(detail: impl Display) -> String {
    format!("Workspace file is corrupt ({detail}); starting empty.")
}

pub fn unreadable(detail: impl Display) -> String {
    format!("Could not read the workspace file ({detail}); starting empty.")
}

pub fn could_not_save(detail: impl Display) -> String {
    format!("Could not save the workspace ({detail}).")
}

pub fn save_failed_rename(detail: impl Display) -> String {
    format!("Could not finish saving the workspace ({detail}); the previous file is intact.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_use_plural_forms() {
        assert_eq!(loaded(0), "Workspace is empty.");
        assert_eq!(loaded(1), "Loaded 1 note.");
        assert_eq!(loaded(7), "Loaded 7 notes.");
    }

    #[test]
    fn detail_sentences_embed_the_cause() {
        assert!(corrupt("expected value").contains("expected value"));
        assert!(could_not_save("permission denied").contains("permission denied"));
        assert!(too_large(9, 5).contains('9'));
    }
}
