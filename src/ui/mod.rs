//! The three panel builders, split by pane.
//!
//! Each panel is a free function over a borrowed [`PanelCtx`]: it reads the
//! record, mutates only its own transient state (search text, popup), and
//! *emits* [`Command`]s for anything that should change the record. The app
//! shell drains the commands and runs them through one write path.

mod calendar;
mod chains;
mod editor;
mod sidebar;

pub(crate) use calendar::calendar;
pub(crate) use chains::chains;
pub(crate) use editor::editor;
pub(crate) use sidebar::sidebar;

use crate::command::Command;
use crate::state::ChainNotes;

/// What a panel can see and touch this frame.
pub(crate) struct PanelCtx<'a> {
    /// The shared record (read, and in-place text edits via `TextEdit`).
    pub notes: &'a mut ChainNotes,
    /// The sidebar's transient search box.
    pub search: &'a mut String,
    /// The editor's transient link-popup flag.
    pub link_popup_open: &'a mut bool,
    /// Requests focus for the sidebar search field on this frame.
    pub focus_search: &'a mut bool,
    /// Transient month/day selection for the metadata calendar.
    pub calendar_year: &'a mut i32,
    pub calendar_month: &'a mut u32,
    pub calendar_day: &'a mut Option<u32>,
    /// Whether the editor is currently read-only or editable.
    pub edit_mode: &'a mut bool,
    /// Commands the panel wants the shell to run after it returns.
    pub pending: &'a mut Vec<Command>,
}

impl PanelCtx<'_> {
    /// Queue a command for the shell.
    pub fn emit(&mut self, cmd: Command) {
        self.pending.push(cmd);
    }
}
