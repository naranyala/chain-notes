//! The autosave controller: debounce, flush, and the save-mode label.
//!
//! Owns the one filesystem seam (`WorkspaceStore`) plus the three pieces of
//! debounce bookkeeping that used to live inline on the app shell
//! (`dirty`, `last_edit`, `save_mode`). Pulled out so the debounce and
//! flush logic is unit-testable without an eframe context.

use std::time::{Duration, Instant};

use crate::messages as msg;
use crate::note::now_secs;
use crate::state::ChainNotes;
use crate::store::{StoreError, WorkspaceStore};

/// Debounce before a changed record hits disk (I1.3).
pub const DEBOUNCE: Duration = Duration::from_millis(250);

/// How the last save went, for the status bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SaveMode {
    /// No file yet, nothing saved.
    None,
    /// The record on disk matches memory.
    Saved,
    /// The record on disk is behind memory.
    Dirty,
    /// The last save attempt failed.
    Failed,
}

impl SaveMode {
    /// The right-aligned status-bar text (I2.3: it must say what actually
    /// happened to the file).
    pub fn label(self) -> &'static str {
        match self {
            Self::None => msg::NOT_SAVED,
            Self::Saved => msg::SAVED_TO_DISK,
            Self::Dirty => msg::SAVING,
            Self::Failed => msg::NOT_SAVED_SEE_STATUS,
        }
    }
}

/// Debounced autosave over one store.
pub struct AutoSaver {
    store: WorkspaceStore,
    mode: SaveMode,
    dirty: bool,
    last_edit: Option<Instant>,
    debounce: Duration,
}

impl AutoSaver {
    /// A saver with the production debounce.
    pub fn new(store: WorkspaceStore, mode: SaveMode, dirty: bool) -> Self {
        Self::with_debounce(store, mode, dirty, DEBOUNCE)
    }

    /// A saver with an explicit debounce — used by tests to fire
    /// `maybe_flush` without waiting on the wall clock.
    pub(crate) fn with_debounce(
        store: WorkspaceStore,
        mode: SaveMode,
        dirty: bool,
        debounce: Duration,
    ) -> Self {
        Self {
            store,
            mode,
            dirty,
            // A dirty boot (first run) must start its debounce clock, or the
            // demo would never reach disk until the user's first edit.
            last_edit: dirty.then(Instant::now),
            debounce,
        }
    }

    pub fn mode(&self) -> SaveMode {
        self.mode
    }

    #[cfg(test)]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Record changed: mark dirty and restart the debounce.
    pub fn touch(&mut self) {
        self.dirty = true;
        self.mode = SaveMode::Dirty;
        self.last_edit = Some(Instant::now());
    }

    /// Write now if dirty. Returns `None` when there was nothing to write,
    /// `Some(Ok(()))` on success, `Some(Err(_))` on failure (the caller
    /// puts the sentence in the status line).
    pub fn flush(&mut self, notes: &ChainNotes) -> Option<Result<(), StoreError>> {
        if !self.dirty {
            return None;
        }
        let record = notes.to_record(now_secs());
        match self.store.save(&record) {
            Ok(()) => {
                self.dirty = false;
                self.mode = SaveMode::Saved;
                self.last_edit = None;
                Some(Ok(()))
            }
            Err(e) => {
                self.mode = SaveMode::Failed;
                Some(Err(e))
            }
        }
    }

    /// Flush on debounce expiry (I1.3: a pending write is flushed, never
    /// dropped — `flush` is also called on exit).
    pub fn maybe_flush(&mut self, notes: &ChainNotes) -> Option<Result<(), StoreError>> {
        if let Some(at) = self.last_edit
            && at.elapsed() >= self.debounce
        {
            self.flush(notes)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::WorkspaceRecord;
    use std::fs;
    use std::path::PathBuf;

    fn scratch(name: &str) -> (WorkspaceStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "chainnotes-autosave-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("workspace.json");
        (WorkspaceStore::at(path.clone()), path)
    }

    fn notes() -> ChainNotes {
        ChainNotes::from_record(WorkspaceRecord::demo(), msg::READY.to_owned())
    }

    #[test]
    fn labels_are_distinct_per_mode() {
        let labels = [
            SaveMode::None.label(),
            SaveMode::Saved.label(),
            SaveMode::Dirty.label(),
            SaveMode::Failed.label(),
        ];
        for (i, a) in labels.iter().enumerate() {
            for (j, b) in labels.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "every save mode needs its own label");
                }
            }
        }
    }

    #[test]
    fn touch_marks_dirty_and_restarts_the_clock() {
        let (store, _path) = scratch("touch");
        let mut s = AutoSaver::new(store, SaveMode::Saved, false);
        assert!(!s.is_dirty());
        s.touch();
        assert!(s.is_dirty());
        assert_eq!(s.mode(), SaveMode::Dirty);
    }

    #[test]
    fn flush_writes_the_record_and_reports_saved() {
        let (store, path) = scratch("flush");
        let mut s = AutoSaver::new(store, SaveMode::Dirty, true);
        let n = notes();
        let out = s.flush(&n).expect("dirty, so a write happens");
        assert!(out.is_ok());
        assert!(path.exists());
        assert!(!s.is_dirty());
        assert_eq!(s.mode(), SaveMode::Saved);
        assert!(s.flush(&n).is_none(), "a clean saver has nothing to write");
    }

    #[test]
    fn flush_reports_a_write_failure_without_claiming_saved() {
        // A path whose parent is an existing *file* makes create_dir_all fail.
        let dir =
            std::env::temp_dir().join(format!("chainnotes-autosave-fail-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        let blocker = dir.join("blocker");
        fs::write(&blocker, b"x").expect("blocker file");
        let store = WorkspaceStore::at(blocker.join("workspace.json"));

        let mut s = AutoSaver::new(store, SaveMode::Dirty, true);
        let n = notes();
        let out = s.flush(&n).expect("attempted");
        assert!(out.is_err());
        assert_eq!(s.mode(), SaveMode::Failed);
        assert!(s.is_dirty(), "a failed write stays dirty for a retry");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn maybe_flush_waits_for_the_debounce() {
        let (store, path) = scratch("debounce-long");
        // A long debounce: maybe_flush must not fire.
        let mut s = AutoSaver::with_debounce(store, SaveMode::Dirty, true, Duration::from_secs(60));
        let n = notes();
        assert!(s.maybe_flush(&n).is_none());
        assert!(!path.exists());

        // A zero debounce fires on the next call.
        let (store, path) = scratch("debounce-zero");
        let mut s = AutoSaver::with_debounce(store, SaveMode::Dirty, true, Duration::ZERO);
        assert!(s.maybe_flush(&n).is_some());
        assert!(path.exists());
        assert_eq!(s.mode(), SaveMode::Saved);
    }

    #[test]
    fn a_dirty_boot_starts_with_the_clock_running() {
        let (store, path) = scratch("dirty-boot");
        // dirty=true at construction must still flush via maybe_flush,
        // because the demo has to reach disk on a first run.
        let mut s = AutoSaver::with_debounce(store, SaveMode::Dirty, true, Duration::ZERO);
        let n = notes();
        assert!(s.maybe_flush(&n).is_some());
        assert!(path.exists());
    }
}
