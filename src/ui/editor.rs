//! The center panel: the title/body editor plus delete/undo/link actions.

use eframe::egui;

use super::PanelCtx;
use crate::command::Command;
use crate::messages as msg;

pub(crate) fn editor(ui: &mut egui::Ui, ctx: &mut PanelCtx<'_>) {
    let Some(id) = ctx.notes.selected else {
        ui.centered_and_justified(|ui| ui.weak(msg::SELECT_OR_CREATE));
        return;
    };
    let Some(idx) = ctx.notes.note_index(id) else {
        ctx.notes.selected = None;
        return;
    };

    let mut delete = false;
    let mut undo = false;

    egui::Panel::top("editor_toolbar")
        .resizable(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    delete = true;
                }
                let can_undo = ctx.notes.has_undo();
                if ui
                    .add_enabled(can_undo, egui::Button::new("Undo delete"))
                    .clicked()
                {
                    undo = true;
                }
                ui.separator();
                ui.weak(format!("note #{id}"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let resp = ui.button("Link…");
                    if resp.clicked() {
                        *ctx.link_popup_open = !*ctx.link_popup_open;
                    }
                    let candidates: Vec<(u64, String, bool)> = ctx
                        .notes
                        .notes
                        .iter()
                        .filter(|n| n.id != id)
                        .map(|n| {
                            let linked = ctx.notes.notes[idx].links.contains(&n.id);
                            (n.id, n.title.clone(), linked)
                        })
                        .collect();
                    let mut popup_open = *ctx.link_popup_open;
                    let mut toggled = None;
                    egui::Popup::from_response(&resp)
                        .open_bool(&mut popup_open)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            if candidates.is_empty() {
                                ui.weak(msg::NO_OTHER_NOTES);
                                return;
                            }
                            for (cid, title, linked) in &candidates {
                                if ui.selectable_label(*linked, title).clicked() {
                                    toggled = Some(*cid);
                                }
                            }
                        });
                    *ctx.link_popup_open = popup_open;
                    if let Some(cid) = toggled {
                        ctx.emit(Command::ToggleLink { from: id, to: cid });
                    }
                });
            });
        });

    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("editor_scroll")
            .show(ui, |ui| {
                let mut changed = false;
                {
                    let note = &mut ctx.notes.notes[idx];
                    let title = egui::TextEdit::singleline(&mut note.title)
                        .hint_text("Title")
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Heading);
                    changed |= ui.add(title).changed();
                    ui.add_space(4.0);
                    let body = egui::TextEdit::multiline(&mut note.body)
                        .hint_text("Start writing…")
                        .desired_width(f32::INFINITY)
                        .desired_rows(20)
                        .code_editor();
                    changed |= ui.add(body).changed();
                }
                if changed {
                    ctx.emit(Command::NoteEdited);
                }
            });
    });

    if delete {
        ctx.emit(Command::DeleteSelected);
    }
    if undo {
        ctx.emit(Command::UndoDelete);
    }
}
