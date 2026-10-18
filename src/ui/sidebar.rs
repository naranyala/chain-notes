//! The left panel: the note list, live search, and the "＋ New" button.

use eframe::egui;

use super::PanelCtx;
use crate::command::Command;
use crate::messages as msg;

pub(crate) fn sidebar(ui: &mut egui::Ui, ctx: &mut PanelCtx<'_>) {
    let mut created = false;
    ui.horizontal(|ui| {
        if ui.button("＋ New").clicked() {
            created = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!("{} notes", ctx.notes.notes.len()));
        });
    });
    if created {
        ctx.emit(Command::NewNote);
    }
    let search = ui.add(egui::TextEdit::singleline(ctx.search).hint_text("Search…"));
    if *ctx.focus_search {
        search.request_focus();
        *ctx.focus_search = false;
    }
    ui.separator();

    let selected = ctx.notes.selected;
    egui::ScrollArea::vertical()
        .id_salt("sidebar_scroll")
        .show(ui, |ui| {
            let visible = ctx.notes.visible_notes(ctx.search);
            if visible.is_empty() {
                ui.weak(msg::NO_NOTES_MATCH);
                return;
            }
            for idx in visible {
                let note = &ctx.notes.notes[idx];
                let is_selected = selected == Some(note.id);
                if ui
                    .selectable_label(is_selected, note.title.as_str())
                    .clicked()
                {
                    ctx.emit(Command::SelectNote(note.id));
                }
            }
        });
}
