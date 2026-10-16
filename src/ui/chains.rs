//! The right panel: outgoing links and derived backlinks for the selection.

use eframe::egui;

use super::PanelCtx;
use crate::command::Command;
use crate::messages as msg;
use crate::note::fmt_time;

pub(crate) fn chains(ui: &mut egui::Ui, ctx: &mut PanelCtx<'_>) {
    let Some(id) = ctx.notes.selected else {
        ui.weak(msg::NO_NOTE_SELECTED);
        return;
    };
    let Some(idx) = ctx.notes.note_index(id) else {
        return;
    };

    ui.heading("Chains");
    ui.weak(format!(
        "created {}, edited {}",
        fmt_time(ctx.notes.notes[idx].created_at),
        fmt_time(ctx.notes.notes[idx].updated_at)
    ));
    ui.separator();

    link_section(ui, ctx, "Outgoing:", &ctx.notes.outgoing(id));
    ui.add_space(8.0);
    link_section(ui, ctx, "Backlinks:", &ctx.notes.backlinks(id));
}

/// One labeled list of linked ids; clicking a row selects that note.
fn link_section(ui: &mut egui::Ui, ctx: &mut PanelCtx<'_>, label: &str, ids: &[u64]) {
    if ids.is_empty() {
        if label.starts_with("Outgoing") {
            ui.weak(msg::NO_LINKS_YET);
        }
        return;
    }
    ui.label(label);
    let mut jump = None;
    for &lid in ids {
        let title = ctx
            .notes
            .title(lid)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("note #{lid}"));
        if ui.selectable_label(false, title).clicked() {
            jump = Some(lid);
        }
    }
    if let Some(lid) = jump {
        ctx.emit(Command::SelectNote(lid));
    }
}
