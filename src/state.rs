//! The app record and its operations, with no egui in sight (pyramid I1.2).
//!
//! `ChainNotes` owns everything that reaches disk; the UI holds only
//! transient state (search text, open popup). Every mutation that should
//! persist goes through a method here, which also sets the status sentence
//! and returns whether the record changed.
//!
//! The link-graph rules live in `graph.rs` and the undo slot in
//! `history.rs`; this module orchestrates them and owns the record's own
//! fields (notes, selection, id counter, status).

use crate::graph;
use crate::history::History;
use crate::messages as msg;
use crate::note::Note;
use crate::store::WorkspaceRecord;

/// The shared record (I3.1). Widget-free and unit-tested.
pub struct ChainNotes {
    pub notes: Vec<Note>,
    pub selected: Option<u64>,
    pub next_id: u64,
    pub status: String,
    history: History,
}

impl ChainNotes {
    /// Boot from a record: the loaded copy, or the demo on a first run.
    pub fn from_record(record: WorkspaceRecord, status: String) -> Self {
        Self {
            notes: record.notes,
            selected: record.selected,
            next_id: record.next_id.max(1),
            status,
            history: History::default(),
        }
    }

    /// Snapshot the parts of the state that must survive a restart.
    pub fn to_record(&self, saved_at: u64) -> WorkspaceRecord {
        WorkspaceRecord {
            version: crate::store::SCHEMA_VERSION,
            notes: self.notes.clone(),
            selected: self.selected,
            next_id: self.next_id,
            saved_at,
        }
    }

    pub fn note_index(&self, id: u64) -> Option<usize> {
        self.notes.iter().position(|n| n.id == id)
    }

    /// The title of a note, or `None` when it does not exist.
    pub fn title(&self, id: u64) -> Option<&str> {
        self.note_index(id).map(|i| self.notes[i].title.as_str())
    }

    /// Indices of notes matching the search query (case-insensitive title
    /// and body substring). An empty query shows everything.
    pub fn visible_notes(&self, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                query.is_empty()
                    || n.title.to_lowercase().contains(&query)
                    || n.body.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Ids of notes this one links to, in link order, skipping dangles.
    pub fn outgoing(&self, id: u64) -> Vec<u64> {
        graph::outgoing(&self.notes, id)
    }

    /// Ids of notes that link to this one (derived, never stored).
    pub fn backlinks(&self, id: u64) -> Vec<u64> {
        graph::backlinks(&self.notes, id)
    }

    /// Create a note, select it, and report. Always returns the new id.
    pub fn add_note(&mut self, title: &str) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1).max(1);
        self.notes.push(Note::new(id, title));
        self.selected = Some(id);
        self.status = msg::created(id);
        id
    }

    /// Delete a note, strip every edge to it, and remember the slot for
    /// undo. A second delete replaces the slot (one-deep, TODO-006).
    pub fn delete_note(&mut self, id: u64) {
        let Some(idx) = self.note_index(id) else {
            return;
        };
        let note = self.notes.remove(idx);
        let stripped_incoming = graph::strip_edges_to(&mut self.notes, id);

        if self.selected == Some(id) {
            self.selected = self.notes.first().map(|n| n.id);
        }

        self.history.record_delete(note, stripped_incoming);
        self.status = msg::deleted(id);
    }

    /// Restore the last deletion, including the edges that pointed at it.
    /// Returns the restored id, or `None` when there is nothing to undo.
    pub fn undo_delete(&mut self) -> Option<u64> {
        let Some(slot) = self.history.take() else {
            self.status = msg::NOTHING_TO_UNDO.to_owned();
            return None;
        };
        let id = slot.note.id;

        // Put the note back at its old index when we can find one that
        // sorts before it, else append.
        let insert_at = self
            .notes
            .iter()
            .position(|n| n.id > id)
            .unwrap_or(self.notes.len());
        self.notes.insert(insert_at, slot.note);

        graph::restore_edges(&mut self.notes, &slot.stripped_incoming, id);

        self.selected = Some(id);
        self.status = msg::restored(id);
        Some(id)
    }

    pub fn has_undo(&self) -> bool {
        self.history.has_undo()
    }

    /// Toggle a directed edge `from -> to`, stamping the source note.
    /// Linking a note to itself is refused; a missing endpoint is a no-op.
    pub fn toggle_link(&mut self, from: u64, to: u64) -> bool {
        if !graph::toggle_link(&mut self.notes, from, to) {
            return false;
        }
        if let Some(idx) = self.note_index(from) {
            self.notes[idx].touch();
        }
        true
    }

    /// Mark the selected note's text as edited.
    pub fn touch_selected(&mut self) {
        if let Some(id) = self.selected
            && let Some(idx) = self.note_index(id)
        {
            self.notes[idx].touch();
            self.status = msg::edited(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_two() -> ChainNotes {
        let mut c = ChainNotes::from_record(WorkspaceRecord::demo(), msg::READY.to_owned());
        c.status = msg::READY.to_owned();
        c
    }

    #[test]
    fn add_allocates_sequential_ids_and_selects() {
        let mut c = ChainNotes::from_record(WorkspaceRecord::default(), msg::READY.to_owned());
        let a = c.add_note("a");
        let b = c.add_note("b");
        assert_eq!((a, b), (1, 2));
        assert_eq!(c.selected, Some(2));
        assert_eq!(c.next_id, 3);
        assert_eq!(c.status, msg::created(2));
    }

    #[test]
    fn delete_strips_outgoing_and_incoming_edges() {
        let mut c = app_with_two();
        // demo: note 2 links to 1
        assert_eq!(c.backlinks(1), vec![2]);
        c.delete_note(1);
        assert!(c.notes.iter().all(|n| !n.links.contains(&1)));
        assert_eq!(c.notes.len(), 1);
    }

    #[test]
    fn delete_of_selected_reselects_first() {
        let mut c = app_with_two();
        c.selected = Some(1);
        c.delete_note(1);
        assert_eq!(c.selected, Some(2));
    }

    #[test]
    fn delete_missing_id_is_a_noop() {
        let mut c = app_with_two();
        let before = c.notes.len();
        c.delete_note(999);
        assert_eq!(c.notes.len(), before);
        assert!(!c.has_undo());
    }

    #[test]
    fn undo_restores_note_and_its_incoming_edges() {
        let mut c = app_with_two();
        assert_eq!(c.backlinks(1), vec![2]);
        c.delete_note(1);
        assert_eq!(c.backlinks(1), Vec::<u64>::new());
        let restored = c.undo_delete();
        assert_eq!(restored, Some(1));
        assert_eq!(c.backlinks(1), vec![2]);
        assert_eq!(c.selected, Some(1));
        assert_eq!(c.status, msg::restored(1));
    }

    #[test]
    fn undo_restores_edges_at_their_original_positions() {
        let mut c = ChainNotes::from_record(WorkspaceRecord::default(), msg::READY.to_owned());
        let a = c.add_note("a");
        let b = c.add_note("b");
        let d = c.add_note("d");
        // a -> b, a -> d. Deleting d should strip the second position.
        c.toggle_link(a, b);
        c.toggle_link(a, d);
        assert_eq!(c.outgoing(a), vec![b, d]);
        c.delete_note(d);
        assert_eq!(c.outgoing(a), vec![b]);
        c.undo_delete();
        assert_eq!(c.outgoing(a), vec![b, d]);
    }

    #[test]
    fn undo_without_a_slot_reports_nothing() {
        let mut c = app_with_two();
        assert_eq!(c.undo_delete(), None);
        assert_eq!(c.status, msg::NOTHING_TO_UNDO);
    }

    #[test]
    fn second_delete_replaces_the_undo_slot() {
        let mut c = app_with_two();
        c.delete_note(1);
        c.delete_note(2);
        assert_eq!(c.undo_delete(), Some(2));
        assert_eq!(c.undo_delete(), None);
    }

    #[test]
    fn toggle_link_adds_then_removes_and_stamps() {
        let mut c = app_with_two();
        assert!(c.toggle_link(1, 2));
        assert_eq!(c.outgoing(1), vec![2]);
        assert!(c.toggle_link(1, 2));
        assert_eq!(c.outgoing(1), Vec::<u64>::new());
    }

    #[test]
    fn toggle_link_refuses_self_and_missing_endpoints() {
        let mut c = app_with_two();
        assert!(!c.toggle_link(1, 1));
        assert!(!c.toggle_link(1, 999));
        assert!(!c.toggle_link(999, 1));
    }

    #[test]
    fn search_matches_title_and_body_case_insensitively() {
        let mut c = ChainNotes::from_record(WorkspaceRecord::default(), msg::READY.to_owned());
        c.add_note("Alpha");
        c.notes[0].body = "Body Content".to_owned();
        c.add_note("Beta");
        assert_eq!(c.visible_notes("alpha"), vec![0]);
        assert_eq!(c.visible_notes("CONTENT"), vec![0]);
        assert_eq!(c.visible_notes("beta"), vec![1]);
        assert_eq!(c.visible_notes("  "), vec![0, 1]);
        assert_eq!(c.visible_notes("gamma"), Vec::<usize>::new());
    }

    #[test]
    fn to_record_round_trips_through_from_record() {
        let mut c = app_with_two();
        c.add_note("third");
        let rec = c.to_record(123);
        assert_eq!(rec.saved_at, 123);
        assert_eq!(rec.next_id, c.next_id);
        let mut c2 = ChainNotes::from_record(rec, msg::READY.to_owned());
        assert_eq!(c2.notes.len(), 3);
        assert_eq!(c2.notes, c.notes);
        c2.status = c.status.clone();
    }
}
