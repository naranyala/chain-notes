//! The monthly metadata calendar: a derived view over note timestamps.

use eframe::egui;

use super::PanelCtx;
use crate::calendar;
use crate::command::Command;

pub(crate) fn calendar(ui: &mut egui::Ui, ctx: &mut PanelCtx<'_>) {
    ui.heading("Calendar");
    ui.horizontal(|ui| {
        if ui.small_button("‹").clicked() {
            let (year, month) = calendar::shift_month(*ctx.calendar_year, *ctx.calendar_month, -1);
            *ctx.calendar_year = year;
            *ctx.calendar_month = month;
            *ctx.calendar_day = None;
        }
        if ui.small_button("Today").clicked() {
            let today = calendar::from_unix_seconds(crate::note::now_secs());
            *ctx.calendar_year = today.year;
            *ctx.calendar_month = today.month;
            *ctx.calendar_day = Some(today.day);
        }
        if ui.small_button("›").clicked() {
            let (year, month) = calendar::shift_month(*ctx.calendar_year, *ctx.calendar_month, 1);
            *ctx.calendar_year = year;
            *ctx.calendar_month = month;
            *ctx.calendar_day = None;
        }
        ui.weak(format!(
            "{} {}",
            calendar::month_name(*ctx.calendar_month),
            *ctx.calendar_year
        ));
    });

    egui::Grid::new("metadata_calendar")
        .num_columns(7)
        .spacing([2.0, 2.0])
        .show(ui, |ui| {
            for label in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
                ui.weak(label);
            }
            ui.end_row();

            let first = calendar::weekday(*ctx.calendar_year, *ctx.calendar_month, 1);
            let days = calendar::days_in_month(*ctx.calendar_year, *ctx.calendar_month);
            let mut cell = 0;
            while cell < first {
                ui.label("");
                cell += 1;
            }
            for day in 1..=days {
                let updated =
                    notes_on_day(ctx, *ctx.calendar_year, *ctx.calendar_month, day, false);
                let created = notes_on_day(ctx, *ctx.calendar_year, *ctx.calendar_month, day, true);
                let label = if updated + created == 0 {
                    day.to_string()
                } else {
                    format!("{day}\n{updated}/{created}")
                };
                let selected = *ctx.calendar_day == Some(day);
                if ui.selectable_label(selected, label).clicked() {
                    *ctx.calendar_day = Some(day);
                }
                cell += 1;
                if cell == 7 {
                    ui.end_row();
                    cell = 0;
                }
            }
            if cell != 0 {
                for _ in cell..7 {
                    ui.label("");
                }
                ui.end_row();
            }
        });

    if let Some(day) = *ctx.calendar_day {
        ui.separator();
        ui.weak(format!(
            "Updated or created on {day} {} {}",
            calendar::month_name(*ctx.calendar_month),
            *ctx.calendar_year
        ));
        let ids = note_ids_on_day(ctx, *ctx.calendar_year, *ctx.calendar_month, day);
        if ids.is_empty() {
            ui.weak("No notes on this day.");
        } else {
            for id in ids {
                let title = ctx.notes.title(id).unwrap_or("Untitled");
                if ui
                    .selectable_label(ctx.notes.selected == Some(id), title)
                    .clicked()
                {
                    ctx.emit(Command::SelectNote(id));
                }
            }
        }
    }
}

fn notes_on_day(ctx: &PanelCtx<'_>, year: i32, month: u32, day: u32, created: bool) -> usize {
    ctx.notes
        .notes
        .iter()
        .filter(|note| {
            let timestamp = if created {
                note.created_at
            } else {
                note.updated_at
            };
            let date = calendar::from_unix_seconds(timestamp);
            date.year == year && date.month == month && date.day == day
        })
        .count()
}

fn note_ids_on_day(ctx: &PanelCtx<'_>, year: i32, month: u32, day: u32) -> Vec<u64> {
    ctx.notes
        .notes
        .iter()
        .filter(|note| {
            let updated = calendar::from_unix_seconds(note.updated_at);
            let created = calendar::from_unix_seconds(note.created_at);
            (updated.year == year && updated.month == month && updated.day == day)
                || (created.year == year && created.month == month && created.day == day)
        })
        .map(|note| note.id)
        .collect()
}
