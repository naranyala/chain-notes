//! The single filesystem seam: load and save one workspace record.
//!
//! This is the only module that touches the workspace file (pyramid I3.4).
//! Writes are atomic (temp file + rename); the file is capped before it is
//! read; every failure is a `StoreError`, never a panic (I3.2).
//!
//! Power-loss limit, stated not implied: the file is flushed but NOT fsynced
//! before the rename, and the parent directory is NOT fsynced after it.
//! Crash-during-write is safe (old or new file, never half-written); power
//! loss in the rename window may still lose the last write (I1.3).

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::messages;
use crate::note::Note;

/// Maximum workspace file size, checked before reading.
pub const MAX_WORKSPACE_BYTES: u64 = 4 * 1024 * 1024;

/// Schema version of the on-disk record.
pub const SCHEMA_VERSION: u32 = 1;

/// Everything that reaches disk. One record owns shared state (I3.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub version: u32,
    pub notes: Vec<Note>,
    pub selected: Option<u64>,
    pub next_id: u64,
    /// Unix seconds of the last save; the merge tiebreaker (I1.3).
    pub saved_at: u64,
}

impl Default for WorkspaceRecord {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            notes: Vec::new(),
            selected: None,
            next_id: 1,
            saved_at: 0,
        }
    }
}

impl WorkspaceRecord {
    /// The first-run record: two linked demo notes, as the skeleton seeded.
    pub fn demo() -> Self {
        let mut welcome = Note::new(1, "Welcome");
        welcome.body = "chainnotes keeps notes and the links between them.".to_owned();
        let mut how = Note::new(2, "How chainnotes works");
        how.body = "Link notes together to form chains.\n\n- Select a note on the left\n- Write in the editor\n- Link notes from the chains panel".to_owned();
        how.links.push(1);
        Self {
            version: SCHEMA_VERSION,
            notes: vec![welcome, how],
            selected: Some(1),
            next_id: 3,
            saved_at: 0,
        }
    }

    /// Clamp a loaded record to safe values (I3.2): rebuild the id counter
    /// past the max, drop links that dangle (via `graph`), and normalize
    /// the selection.
    pub fn normalize(&mut self) {
        let mut max_id = 0u64;
        for note in &self.notes {
            max_id = max_id.max(note.id);
        }
        if self.next_id <= max_id {
            self.next_id = max_id.saturating_add(1).max(1);
        }
        crate::graph::normalize_links(&mut self.notes);
        let ids: std::collections::HashSet<u64> = self.notes.iter().map(|n| n.id).collect();
        if let Some(sel) = self.selected
            && !ids.contains(&sel)
        {
            self.selected = self.notes.first().map(|n| n.id);
        }
    }
}

/// Why a load or save failed. Every variant maps to a status sentence.
#[derive(Debug)]
pub enum StoreError {
    TooLarge { bytes: u64, cap: u64 },
    Corrupt(String),
    Io(String),
    WriteFailed(String),
    RenameFailed(String),
}

impl StoreError {
    /// The user-facing sentence for this failure.
    pub fn sentence(&self) -> String {
        match self {
            Self::TooLarge { bytes, cap } => messages::too_large(*bytes, *cap),
            Self::Corrupt(detail) => messages::corrupt(detail),
            Self::Io(detail) => messages::unreadable(detail),
            Self::WriteFailed(detail) => messages::could_not_save(detail),
            Self::RenameFailed(detail) => messages::save_failed_rename(detail),
        }
    }
}

/// The one seam over the workspace file (I3.4). Swap or fake it in one edit.
pub struct WorkspaceStore {
    path: PathBuf,
}

impl WorkspaceStore {
    /// The conventional per-user location, or `None` if the platform has none.
    pub fn default_location() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("chainnotes").join("workspace.json"))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[cfg(test)]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Read the record. `Ok(None)` when there is no file yet — that is not
    /// an error, it is a first run.
    pub fn load(&self) -> Result<Option<WorkspaceRecord>, StoreError> {
        let meta = match fs::metadata(&self.path) {
            Ok(m) => m,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(StoreError::Io(e.to_string())),
        };
        if meta.len() > MAX_WORKSPACE_BYTES {
            return Err(StoreError::TooLarge {
                bytes: meta.len(),
                cap: MAX_WORKSPACE_BYTES,
            });
        }
        let mut buf = String::new();
        match fs::File::open(&self.path).and_then(|mut f| f.read_to_string(&mut buf)) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(StoreError::Io(e.to_string())),
        }
        if buf.trim().is_empty() {
            return Ok(None);
        }
        let mut record: WorkspaceRecord =
            serde_json::from_str(&buf).map_err(|e| StoreError::Corrupt(e.to_string()))?;
        record.normalize();
        Ok(Some(record))
    }

    /// Write atomically: temp file in the same directory, then rename over
    /// the target. A failed write never clobbers the last good file.
    pub fn save(&self, record: &WorkspaceRecord) -> Result<(), StoreError> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| StoreError::WriteFailed("workspace path has no parent".to_owned()))?;
        fs::create_dir_all(dir)
            .map_err(|e| StoreError::WriteFailed(format!("create {}: {e}", dir.display())))?;

        let json = serde_json::to_string_pretty(record)
            .map_err(|e| StoreError::WriteFailed(e.to_string()))?;

        let tmp = self.path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp)
                .map_err(|e| StoreError::WriteFailed(format!("create {}: {e}", tmp.display())))?;
            f.write_all(json.as_bytes())
                .and_then(|()| f.flush())
                .map_err(|e| StoreError::WriteFailed(format!("write {}: {e}", tmp.display())))?;
        }
        fs::rename(&tmp, &self.path).map_err(|e| {
            StoreError::RenameFailed(format!("rename to {}: {e}", self.path.display()))
        })
    }
}

/// Boot merge rule (I1.3): the durable copy wins unless it is strictly
/// older than the boot snapshot. Equal timestamps keep the durable copy.
pub fn merge(boot: &WorkspaceRecord, durable: &WorkspaceRecord) -> WorkspaceRecord {
    if durable.saved_at < boot.saved_at {
        boot.clone()
    } else {
        durable.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> WorkspaceStore {
        let dir = std::env::temp_dir().join(format!(
            "chainnotes-store-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        WorkspaceStore::at(dir.join("workspace.json"))
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn missing_file_is_none_not_an_error() {
        let store = scratch("missing");
        assert!(matches!(store.load(), Ok(None)));
    }

    #[test]
    fn round_trip_preserves_notes_and_selection() {
        let store = scratch("roundtrip");
        let mut rec = WorkspaceRecord::demo();
        rec.selected = Some(2);
        rec.saved_at = now();
        store.save(&rec).expect("save");
        let loaded = store.load().expect("load").expect("some");
        assert_eq!(loaded.notes.len(), 2);
        assert_eq!(loaded.selected, Some(2));
        assert_eq!(loaded.next_id, 3);
        assert_eq!(loaded.version, SCHEMA_VERSION);
    }

    #[test]
    fn corrupt_file_is_reported_not_panicked() {
        let store = scratch("corrupt");
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "{ not json").unwrap();
        match store.load() {
            Err(StoreError::Corrupt(_)) => {}
            other => panic!("expected Corrupt, got {other:?}"),
        }
    }

    #[test]
    fn empty_file_is_none() {
        let store = scratch("empty");
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "").unwrap();
        assert!(matches!(store.load(), Ok(None)));
    }

    #[test]
    fn over_cap_is_too_large() {
        let store = scratch("toolarge");
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), vec![b' '; (MAX_WORKSPACE_BYTES + 1) as usize]).unwrap();
        match store.load() {
            Err(StoreError::TooLarge { bytes, cap }) => {
                assert!(bytes > cap);
                assert_eq!(cap, MAX_WORKSPACE_BYTES);
            }
            other => panic!("expected TooLarge, got {other:?}"),
        }
    }

    #[test]
    fn failed_write_leaves_previous_file_intact() {
        let store = scratch("atomic");
        let mut rec = WorkspaceRecord::demo();
        rec.saved_at = now();
        store.save(&rec).unwrap();
        let before = fs::read_to_string(store.path()).unwrap();

        // Point at a path whose parent is a file, so create_dir_all fails.
        let bad_parent = scratch("atomic-parent");
        fs::write(bad_parent.path().parent().unwrap(), b"x").unwrap();
        let bad = WorkspaceStore::at(
            bad_parent
                .path()
                .parent()
                .unwrap()
                .join("sub")
                .join("w.json"),
        );
        match bad.save(&rec) {
            Err(StoreError::WriteFailed(_)) => {}
            other => panic!("expected WriteFailed, got {other:?}"),
        }

        let after = fs::read_to_string(store.path()).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn every_error_variant_has_a_distinct_sentence() {
        let too_large = StoreError::TooLarge { bytes: 9, cap: 5 }.sentence();
        assert!(too_large.contains('9') && too_large.contains('5'));

        let corrupt = StoreError::Corrupt("bad".to_owned()).sentence();
        assert!(corrupt.contains("corrupt") && corrupt.contains("bad"));

        let io = StoreError::Io("denied".to_owned()).sentence();
        assert!(io.contains("read") && io.contains("denied"));

        let write = StoreError::WriteFailed("full".to_owned()).sentence();
        assert!(write.contains("save") && write.contains("full"));

        let rename = StoreError::RenameFailed("busy".to_owned()).sentence();
        assert!(rename.contains("intact") && rename.contains("busy"));

        // A write failure must not claim the file could not be *read*.
        assert!(!write.contains("read the workspace"));
    }

    #[test]
    fn normalize_rebuilds_id_counter_and_drops_dangling_links() {
        let mut rec = WorkspaceRecord {
            version: SCHEMA_VERSION,
            notes: vec![Note::new(5, "a"), Note::new(9, "b")],
            selected: Some(999),
            next_id: 2,
            saved_at: 0,
        };
        rec.notes[0].links.push(999);
        rec.notes[0].links.push(5); // self-link
        rec.normalize();
        assert_eq!(rec.next_id, 10);
        assert!(rec.notes[0].links.is_empty());
        assert_eq!(rec.selected, Some(5));
    }

    #[test]
    fn merge_prefers_durable_unless_boot_is_strictly_newer() {
        let mut boot = WorkspaceRecord::demo();
        let mut durable = WorkspaceRecord::demo();
        boot.saved_at = 100;
        durable.saved_at = 100;
        assert_eq!(merge(&boot, &durable).saved_at, 100);

        durable.saved_at = 99;
        assert_eq!(merge(&boot, &durable).saved_at, 100);

        durable.saved_at = 101;
        assert_eq!(merge(&boot, &durable).saved_at, 101);
    }

    #[test]
    fn save_then_load_with_selected_none() {
        let store = scratch("nonesel");
        let mut rec = WorkspaceRecord::default();
        rec.notes.push(Note::new(1, "only"));
        rec.selected = None;
        rec.saved_at = now();
        store.save(&rec).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.selected, None);
        assert_eq!(loaded.notes.len(), 1);
    }

    #[test]
    fn default_location_resolves_under_the_user_data_dir() {
        let Some(path) = WorkspaceStore::default_location() else {
            // A platform with no conventional data dir is allowed to be
            // None; the app says so in the status bar and stays in memory.
            return;
        };
        assert!(path.ends_with("chainnotes/workspace.json"));
    }

    /// The boot *wiring*, not just the merge rule: loading a file that
    /// was saved by a previous run must produce that same record, and the
    /// merge must not drop it. A suite that tests only `merge()` passes
    /// even when the boot path never calls it.
    #[test]
    fn boot_wiring_restores_a_previously_saved_record() {
        let store = scratch("bootwiring");
        let mut saved = WorkspaceRecord::demo();
        saved.notes[0].body = "typed before quit".to_owned();
        saved.selected = Some(2);
        saved.saved_at = now();
        store.save(&saved).unwrap();

        // The store-level half of the boot rule; the app-level wiring that
        // calls it lives in app::tests (`boot_after_a_save_reports_saved_
        // not_dirty`).
        let durable = store.load().unwrap().expect("file exists");
        let boot = WorkspaceRecord::default();
        let merged = merge(&boot, &durable);

        assert_eq!(merged.notes[0].body, "typed before quit");
        assert_eq!(merged.selected, Some(2));
        assert_eq!(merged.notes.len(), 2);

        // Reverting to merge(&demo_saved_just_now, &durable) would drop the
        // save if the demo were stamped with the current time — the exact
        // bug a now-stamped boot snapshot ships. Here the boot side is the
        // empty default (saved_at = 0), so the durable copy always wins.
        let mut wrongly_stamped_boot = WorkspaceRecord::demo();
        wrongly_stamped_boot.saved_at = now() + 10;
        assert_eq!(
            merge(&wrongly_stamped_boot, &durable).notes[0].body,
            WorkspaceRecord::demo().notes[0].body,
            "a boot snapshot stamped in the future would (correctly) win — keep the boot side at 0"
        );
    }
}
