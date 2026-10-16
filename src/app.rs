//! The egui shell: menu, status bar, and panel layout over the shared
//! `ChainNotes` record.
//!
//! This module owns only the composition of the record, the autosave
//! controller, and the transient UI state (search text, open popup).
//! Every decision about what the record contains lives below the UI
//! boundary (`state`, `graph`, `history` — pyramid I1.2); the panel
//! builders live in `ui/`; the debounce/flush logic lives in `autosave`;
//! and record mutations enter through one write path, `handle(Command)`.

use std::path::PathBuf;

use eframe::egui;

use crate::autosave::{AutoSaver, SaveMode};
use crate::command::{Command, Effect};
use crate::messages as msg;
use crate::state::ChainNotes;
use crate::store::{self, WorkspaceRecord, WorkspaceStore};
use crate::ui::{self, PanelCtx};

pub struct ChainNotesApp {
    /// The shared record — everything that reaches disk.
    notes: ChainNotes,
    /// The autosave controller (owns the store, debounce, and save mode).
    saver: AutoSaver,
    /// Transient: the search box.
    search: String,
    /// Transient: the link popup.
    link_popup_open: bool,
}

impl ChainNotesApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        match WorkspaceStore::default_location() {
            Some(path) => Self::at(WorkspaceStore::at(path)),
            // No conventional data dir on this platform: stay in-memory
            // rather than guessing, and say so in the status bar.
            None => {
                let mut app = Self::in_memory();
                app.notes.status = msg::NO_DATA_DIR.to_owned();
                app
            }
        }
    }

    /// Build from an explicit store — the testable core of `new()`.
    fn at(store: WorkspaceStore) -> Self {
        let Boot {
            record,
            status,
            save_mode,
            first_run,
        } = boot(&store);
        Self {
            notes: ChainNotes::from_record(record, status),
            saver: AutoSaver::new(store, save_mode, first_run),
            search: String::new(),
            link_popup_open: false,
        }
    }

    fn in_memory() -> Self {
        Self {
            notes: ChainNotes::from_record(WorkspaceRecord::demo(), msg::READY.to_owned()),
            saver: AutoSaver::new(
                WorkspaceStore::at(PathBuf::from("/nonexistent-chainnotes")),
                SaveMode::None,
                false,
            ),
            search: String::new(),
            link_popup_open: false,
        }
    }

    /// The single write path for every record mutation (I1.2). Menus,
    /// panels, and future shortcuts all enter here, so adding a gesture is
    /// a matter of emitting a command — not of re-implementing a mutation.
    fn handle(&mut self, cmd: Command) -> Option<Effect> {
        match cmd {
            Command::NewNote => {
                self.notes.add_note("Untitled");
                self.saver.touch();
                None
            }
            Command::DeleteSelected => {
                if let Some(id) = self.notes.selected {
                    self.notes.delete_note(id);
                    self.saver.touch();
                }
                None
            }
            Command::UndoDelete => {
                if self.notes.undo_delete().is_some() {
                    self.saver.touch();
                }
                None
            }
            Command::SelectNote(id) => {
                self.notes.selected = Some(id);
                None
            }
            Command::ToggleLink { from, to } => {
                if self.notes.toggle_link(from, to) {
                    self.saver.touch();
                }
                None
            }
            Command::NoteEdited => {
                self.notes.touch_selected();
                self.saver.touch();
                None
            }
            Command::About => {
                self.notes.status = msg::ABOUT.to_owned();
                None
            }
            Command::Quit => {
                self.saver.flush(&self.notes);
                Some(Effect::CloseWindow)
            }
        }
    }

    /// Run a panel with a borrowed [`PanelCtx`], then drain and apply any
    /// commands it emitted.
    fn panel<F>(&mut self, ui: &mut egui::Ui, f: F)
    where
        F: FnOnce(&mut egui::Ui, &mut PanelCtx),
    {
        let mut pending = Vec::new();
        {
            let mut ctx = PanelCtx {
                notes: &mut self.notes,
                search: &mut self.search,
                link_popup_open: &mut self.link_popup_open,
                pending: &mut pending,
            };
            f(ui, &mut ctx);
        }
        for cmd in pending {
            self.handle(cmd);
        }
    }
}

/// What `boot()` decides: the record to start from, the status sentence,
/// the save mode (which must tell the truth about whether the file on disk
/// matches memory, I2.3), and whether the app starts dirty.
struct Boot {
    record: WorkspaceRecord,
    status: String,
    save_mode: SaveMode,
    first_run: bool,
}

/// Read the store and decide the boot record + save mode.
///
/// Split out of `new()` so this wiring is testable without an eframe
/// `CreationContext`. Three cases:
///
/// * **Loaded file** — merge it in (I1.3 boot rule) and report `Saved`:
///   disk already matches memory.
/// * **First run** (`Ok(None)`) — seed the demo marked `Dirty`, so the
///   debounce writes it right after launch instead of claiming "Saved to
///   disk" while no file exists.
/// * **Load error** — demo in memory, `None` on the label, and *not*
///   dirty: the corrupt file is left on disk for the user to recover
///   until they actually edit something.
fn boot(store: &WorkspaceStore) -> Boot {
    match store.load() {
        Ok(Some(durable)) => {
            // Boot rule (I1.3): the durable copy wins unless it is strictly
            // older than the in-memory boot snapshot. On a single-file store
            // the boot side is the empty default (saved_at = 0), so the file
            // always wins — but the rule is applied, not assumed, and it is
            // the same rule a second source (a boot cache) would use.
            let base = WorkspaceRecord::default();
            let record = store::merge(&base, &durable);
            let status = msg::loaded(record.notes.len());
            Boot {
                record,
                status,
                save_mode: SaveMode::Saved,
                first_run: false,
            }
        }
        Ok(None) => Boot {
            record: WorkspaceRecord::demo(),
            status: msg::no_workspace(),
            save_mode: SaveMode::Dirty,
            first_run: true,
        },
        Err(e) => Boot {
            record: WorkspaceRecord::demo(),
            status: e.sentence(),
            save_mode: SaveMode::None,
            first_run: false,
        },
    }
}

impl eframe::App for ChainNotesApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(Err(e)) = self.saver.maybe_flush(&self.notes) {
            self.notes.status = e.sentence();
        }

        egui::Panel::top("menu").resizable(false).show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New note").clicked() {
                        self.handle(Command::NewNote);
                        ui.close();
                    }
                    let can_undo = self.notes.has_undo();
                    if ui
                        .add_enabled(can_undo, egui::Button::new("Undo delete"))
                        .clicked()
                    {
                        self.handle(Command::UndoDelete);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        self.handle(Command::Quit);
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.handle(Command::About);
                        ui.close();
                    }
                });
            });
        });

        egui::Panel::bottom("status")
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.notes.status);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak(self.saver.mode().label());
                    });
                });
            });

        egui::Panel::left("sidebar")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| self.panel(ui, ui::sidebar));

        egui::Panel::right("chains")
            .resizable(true)
            .default_size(200.0)
            .show(ui, |ui| self.panel(ui, ui::chains));

        egui::CentralPanel::default().show(ui, |ui| self.panel(ui, ui::editor));
    }

    /// Flush a pending debounce instead of dropping it (I1.3, I2.1).
    fn on_exit(&mut self) {
        self.saver.flush(&self.notes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use std::fs;
    use std::path::PathBuf;

    /// Scratch workspace path under the system temp dir (never written
    /// unless the test's own flush creates it).
    fn scratch(name: &str) -> (WorkspaceStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "chainnotes-app-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("workspace.json");
        (WorkspaceStore::at(path.clone()), path)
    }

    #[test]
    fn first_run_flush_puts_the_demo_on_disk() {
        let (store, path) = scratch("first-run");
        let mut app = ChainNotesApp::at(store);

        // The demo is memory-only at boot but marked dirty, so the label
        // says "Saving…" rather than claiming a file that does not exist.
        assert_eq!(app.saver.mode(), SaveMode::Dirty);
        assert!(app.saver.is_dirty());
        assert_eq!(app.notes.status, msg::no_workspace());
        assert!(!path.exists(), "file must not exist before the flush");

        // The debounce expiry (first frame after 250 ms) runs this.
        app.saver.flush(&app.notes);
        assert!(path.exists(), "first-run flush must create the file");
        assert_eq!(app.saver.mode(), SaveMode::Saved);
        assert!(!app.saver.is_dirty());

        // The boot path now loads those demo notes and tells the truth
        // about the disk matching memory.
        let again = ChainNotesApp::at(WorkspaceStore::at(path));
        assert_eq!(again.saver.mode(), SaveMode::Saved);
        assert!(!again.saver.is_dirty());
        assert_eq!(again.notes.notes.len(), 2);
        assert_eq!(again.notes.status, msg::loaded(2));
    }

    #[test]
    fn boot_after_a_save_reports_saved_not_dirty() {
        let (store, path) = scratch("boot-loaded");
        let mut record = WorkspaceRecord::demo();
        record.selected = Some(2);
        store.save(&record).expect("setup save");

        let app = ChainNotesApp::at(store);
        assert_eq!(app.saver.mode(), SaveMode::Saved);
        assert!(!app.saver.is_dirty(), "a loaded file needs no write");
        assert_eq!(app.notes.status, msg::loaded(2));
        assert_eq!(app.notes.selected, Some(2), "selection survives boot");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn boot_on_a_corrupt_file_leaves_the_disk_copy_untouched() {
        let (store, path) = scratch("boot-corrupt");
        fs::write(&path, b"{ this is not json").expect("setup corrupt file");

        let mut app = ChainNotesApp::at(store);
        // The label must not claim the disk matches memory, and we must
        // not start writing over the user's bytes until they edit.
        assert_eq!(app.saver.mode(), SaveMode::None);
        assert!(!app.saver.is_dirty());
        assert_eq!(app.notes.notes.len(), 2, "falls back to the demo");
        assert!(
            app.notes.status.contains("corrupt"),
            "status explains why: {}",
            app.notes.status
        );

        app.saver.flush(&app.notes);
        let disk = fs::read(&path).expect("file still readable");
        assert_eq!(
            disk, b"{ this is not json",
            "an untouched corrupt file must survive an idle exit"
        );
    }

    #[test]
    fn save_mode_labels_are_distinct() {
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
    fn new_note_command_creates_selects_and_marks_dirty() {
        let (store, _path) = scratch("cmd-new");
        let mut app = ChainNotesApp::at(store);
        let before = app.notes.notes.len();
        app.handle(Command::NewNote);
        assert_eq!(app.notes.notes.len(), before + 1);
        let new_id = app.notes.selected.expect("new note is selected");
        assert_eq!(app.notes.title(new_id), Some("Untitled"));
        assert!(app.saver.is_dirty());
    }

    #[test]
    fn delete_and_undo_commands_round_trip_through_handle() {
        let (store, _path) = scratch("cmd-undo");
        let mut app = ChainNotesApp::at(store);
        let victim = app.notes.selected.expect("demo selects a note");
        app.handle(Command::DeleteSelected);
        assert_eq!(app.notes.notes.len(), 1);
        assert!(app.saver.is_dirty());
        app.handle(Command::UndoDelete);
        assert_eq!(app.notes.notes.len(), 2);
        assert_eq!(app.notes.selected, Some(victim));
    }

    #[test]
    fn toggle_link_command_edges_and_dirty_the_record() {
        let (store, _path) = scratch("cmd-link");
        let mut app = ChainNotesApp::at(store);
        // demo: note 2 already links to 1; toggling 1->2 adds the reverse.
        app.handle(Command::ToggleLink { from: 1, to: 2 });
        assert_eq!(app.notes.outgoing(1), vec![2]);
        assert!(app.saver.is_dirty());
        // A refused self-link must not dirty the record.
        app.saver.flush(&app.notes);
        app.handle(Command::ToggleLink { from: 1, to: 1 });
        assert!(!app.saver.is_dirty());
    }

    #[test]
    fn select_and_about_commands_do_not_dirty_the_record() {
        let (store, _path) = scratch("cmd-select");
        let mut app = ChainNotesApp::at(store);
        app.saver.flush(&app.notes);
        app.handle(Command::SelectNote(2));
        assert_eq!(app.notes.selected, Some(2));
        app.handle(Command::About);
        assert_eq!(app.notes.status, msg::ABOUT);
        assert!(!app.saver.is_dirty(), "view-only commands need no save");
    }
}
