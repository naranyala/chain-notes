//! The one-deep undo slot.
//!
//! Remembers the most recent deletion — the note itself plus every incoming
//! edge that was stripped when it was removed — so an undo can put the
//! graph back exactly. A second delete replaces the slot (one-deep by
//! decision, TODO-006).

use crate::note::Note;

/// What a deletion remembers: the note, and the edges that pointed at it.
#[derive(Clone, Debug)]
pub(crate) struct DeletedSlot {
    pub note: Note,
    /// `(source_id, position_in_source_links)` for each edge that pointed at
    /// the deleted note, recorded front-to-back so undo restores order.
    pub stripped_incoming: Vec<(u64, usize)>,
}

/// The undo history. Currently one slot; the shape generalizes to a stack.
#[derive(Default)]
pub(crate) struct History {
    slot: Option<DeletedSlot>,
}

impl History {
    /// Record a deletion, replacing any previous slot (one-deep).
    pub fn record_delete(&mut self, note: Note, stripped_incoming: Vec<(u64, usize)>) {
        self.slot = Some(DeletedSlot {
            note,
            stripped_incoming,
        });
    }

    /// Take the slot, leaving the history empty.
    pub fn take(&mut self) -> Option<DeletedSlot> {
        self.slot.take()
    }

    pub fn has_undo(&self) -> bool {
        self.slot.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let mut h = History::default();
        assert!(!h.has_undo());
        assert!(h.take().is_none());
    }

    #[test]
    fn record_then_take_returns_the_slot_once() {
        let mut h = History::default();
        h.record_delete(Note::new(1, "a"), vec![(2, 0)]);
        assert!(h.has_undo());
        let slot = h.take().expect("slot present");
        assert_eq!(slot.note.id, 1);
        assert_eq!(slot.stripped_incoming, vec![(2, 0)]);
        assert!(!h.has_undo());
        assert!(h.take().is_none());
    }

    #[test]
    fn a_second_record_replaces_the_first() {
        let mut h = History::default();
        h.record_delete(Note::new(1, "a"), vec![]);
        h.record_delete(Note::new(2, "b"), vec![]);
        assert_eq!(h.take().expect("slot").note.id, 2);
    }
}
