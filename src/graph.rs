//! Link-graph queries and invariants over `&[Note]` / `&mut [Note]`.
//!
//! Widget-free, filesystem-free: every function here is a pure operation on
//! the notes slice. `state.rs` delegates its link work here so the graph
//! rules live in one place (and can be tested without a `ChainNotes`).

use std::collections::HashSet;

use crate::note::Note;

/// Ids `id` links to, in link order, skipping dangles.
pub(crate) fn outgoing(notes: &[Note], id: u64) -> Vec<u64> {
    notes
        .iter()
        .find(|n| n.id == id)
        .map(|n| n.links.clone())
        .unwrap_or_default()
}

/// Ids of notes that link to `id` (derived, never stored).
pub(crate) fn backlinks(notes: &[Note], id: u64) -> Vec<u64> {
    notes
        .iter()
        .filter(|n| n.links.contains(&id))
        .map(|n| n.id)
        .collect()
}

/// Toggle the directed edge `from -> to`. Refuses a self-link or a missing
/// endpoint (returns `false`); a successful toggle returns `true`.
pub(crate) fn toggle_link(notes: &mut [Note], from: u64, to: u64) -> bool {
    if from == to {
        return false;
    }
    let Some(fi) = notes.iter().position(|n| n.id == from) else {
        return false;
    };
    if !notes.iter().any(|n| n.id == to) {
        return false;
    }
    let links = &mut notes[fi].links;
    if links.contains(&to) {
        links.retain(|&l| l != to);
    } else {
        links.push(to);
    }
    true
}

/// Strip every edge pointing at `id`, recording `(source_id, position)` for
/// each one so an undo can put the graph back exactly.
pub(crate) fn strip_edges_to(notes: &mut [Note], id: u64) -> Vec<(u64, usize)> {
    let mut stripped = Vec::new();
    for source in notes.iter_mut() {
        for (pos, &link) in source.links.iter().enumerate() {
            if link == id {
                stripped.push((source.id, pos));
            }
        }
        source.links.retain(|&l| l != id);
    }
    stripped
}

/// Re-insert previously stripped edges at their recorded positions.
pub(crate) fn restore_edges(notes: &mut [Note], edges: &[(u64, usize)], id: u64) {
    for &(source_id, pos) in edges {
        if let Some(si) = notes.iter().position(|n| n.id == source_id) {
            let p = pos.min(notes[si].links.len());
            notes[si].links.insert(p, id);
        }
    }
}

/// Drop dangling and self links (the load-time invariant).
pub(crate) fn normalize_links(notes: &mut [Note]) {
    let ids: HashSet<u64> = notes.iter().map(|n| n.id).collect();
    for note in notes.iter_mut() {
        note.links.retain(|l| ids.contains(l) && *l != note.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(n: u64) -> Vec<Note> {
        (1..=n).map(|i| Note::new(i, format!("n{i}"))).collect()
    }

    #[test]
    fn outgoing_and_backlinks_follow_the_edges() {
        let mut ns = notes(3);
        assert!(toggle_link(&mut ns, 2, 1));
        assert!(toggle_link(&mut ns, 3, 1));
        assert_eq!(outgoing(&ns, 1), Vec::<u64>::new());
        assert_eq!(outgoing(&ns, 2), vec![1]);
        assert_eq!(backlinks(&ns, 1), vec![2, 3]);
        assert_eq!(backlinks(&ns, 2), Vec::<u64>::new());
        assert_eq!(outgoing(&ns, 999), Vec::<u64>::new());
    }

    #[test]
    fn toggle_adds_then_removes_and_refuses_bad_endpoints() {
        let mut ns = notes(2);
        assert!(toggle_link(&mut ns, 1, 2));
        assert_eq!(outgoing(&ns, 1), vec![2]);
        assert!(toggle_link(&mut ns, 1, 2));
        assert_eq!(outgoing(&ns, 1), Vec::<u64>::new());
        assert!(!toggle_link(&mut ns, 1, 1));
        assert!(!toggle_link(&mut ns, 1, 999));
        assert!(!toggle_link(&mut ns, 999, 1));
    }

    #[test]
    fn strip_records_positions_and_restore_puts_them_back() {
        let mut ns = notes(3);
        toggle_link(&mut ns, 1, 2);
        toggle_link(&mut ns, 1, 3);
        let stripped = strip_edges_to(&mut ns, 3);
        assert_eq!(stripped, vec![(1, 1)]);
        assert_eq!(outgoing(&ns, 1), vec![2]);
        restore_edges(&mut ns, &stripped, 3);
        assert_eq!(outgoing(&ns, 1), vec![2, 3]);
    }

    #[test]
    fn restore_ignores_a_missing_source() {
        let mut ns = notes(1);
        restore_edges(&mut ns, &[(42, 0)], 1);
        assert_eq!(ns[0].links, Vec::<u64>::new());
    }

    #[test]
    fn normalize_links_drops_dangling_and_self_edges() {
        let mut ns = notes(2);
        toggle_link(&mut ns, 1, 2);
        ns[0].links.push(1); // self
        ns[0].links.push(99); // dangling
        normalize_links(&mut ns);
        assert_eq!(ns[0].links, vec![2]);
    }
}
