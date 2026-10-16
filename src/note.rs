use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub links: Vec<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Note {
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        let now = now_secs();
        Self {
            id,
            title: title.into(),
            body: String::new(),
            links: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = now_secs();
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn fmt_time(unix: u64) -> String {
    let now = now_secs();
    let secs = now.saturating_sub(unix);
    match secs {
        0..=59 => format!("{secs}s ago"),
        60..=3599 => format!("{}m ago", secs / 60),
        3600..=86399 => format!("{}h ago", secs / 3600),
        _ => format!("{}d ago", secs / 86400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stamps_both_timestamps_and_starts_linkless() {
        let n = Note::new(7, "title");
        assert_eq!(n.id, 7);
        assert_eq!(n.title, "title");
        assert!(n.body.is_empty());
        assert!(n.links.is_empty());
        assert!(n.created_at > 0);
        assert_eq!(n.created_at, n.updated_at);
    }

    #[test]
    fn touch_moves_updated_at_forward() {
        let mut n = Note::new(1, "t");
        n.updated_at = n.created_at.saturating_sub(10);
        n.touch();
        assert!(n.updated_at >= n.created_at);
    }

    #[test]
    fn fmt_time_buckets() {
        let now = now_secs();
        assert_eq!(fmt_time(now), "0s ago");
        assert_eq!(fmt_time(now.saturating_sub(59)), "59s ago");
        assert_eq!(fmt_time(now.saturating_sub(60)), "1m ago");
        assert_eq!(fmt_time(now.saturating_sub(3599)), "59m ago");
        assert_eq!(fmt_time(now.saturating_sub(3600)), "1h ago");
        assert_eq!(fmt_time(now.saturating_sub(86399)), "23h ago");
        assert_eq!(fmt_time(now.saturating_sub(86400)), "1d ago");
        assert_eq!(fmt_time(now.saturating_sub(86400 * 3)), "3d ago");
    }

    #[test]
    fn fmt_time_never_panics_on_future_or_zero() {
        let now = now_secs();
        assert_eq!(fmt_time(now + 10_000), "0s ago");
        assert_eq!(fmt_time(0), fmt_time(0));
    }
}
