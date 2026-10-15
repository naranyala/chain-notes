use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
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
