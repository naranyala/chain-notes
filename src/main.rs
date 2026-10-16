mod app;
mod autosave;
mod command;
mod graph;
mod history;
mod messages;
mod note;
mod state;
mod store;
mod ui;

use app::ChainNotesApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("ChainNotes"),
        ..Default::default()
    };

    eframe::run_native(
        "chainnotes",
        options,
        Box::new(|cc| Ok(Box::new(ChainNotesApp::new(cc)))),
    )
}
