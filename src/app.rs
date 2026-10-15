use eframe::egui;

use crate::note::{Note, fmt_time};

pub struct ChainNotesApp {
    notes: Vec<Note>,
    selected: Option<u64>,
    next_id: u64,
    search: String,
    status: String,
    link_popup_open: bool,
}

impl ChainNotesApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            notes: Vec::new(),
            selected: None,
            next_id: 1,
            search: String::new(),
            status: "Ready".to_owned(),
            link_popup_open: false,
        };
        app.add_note("Welcome");
        app.add_note("How chainnotes works");
        if let Some(note) = app.notes.get_mut(1) {
            note.body = "Link notes together to form chains.\n\n- Select a note on the left\n- Write in the editor\n- Link notes from the chains panel".to_owned();
        }
        app.status = "Ready".to_owned();
        app
    }

    fn add_note(&mut self, title: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.notes.push(Note::new(id, title));
        self.selected = Some(id);
        self.status = format!("Created note #{id}");
        id
    }

    fn delete_note(&mut self, id: u64) {
        self.notes.retain(|n| n.id != id);
        for note in &mut self.notes {
            note.links.retain(|&l| l != id);
        }
        if self.selected == Some(id) {
            self.selected = self.notes.first().map(|n| n.id);
        }
        self.status = format!("Deleted note #{id}");
    }

    fn note_index(&self, id: u64) -> Option<usize> {
        self.notes.iter().position(|n| n.id == id)
    }

    fn visible_notes(&self) -> Vec<usize> {
        let query = self.search.trim().to_lowercase();
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                query.is_empty()
                    || n.title.to_lowercase().contains(&query)
                    || n.body.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn ui_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("＋ New").clicked() {
                self.add_note("Untitled");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} notes", self.notes.len()));
            });
        });
        ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search…"));
        ui.separator();

        egui::ScrollArea::vertical().id_salt("sidebar_scroll").show(ui, |ui| {
            let visible = self.visible_notes();
            if visible.is_empty() {
                ui.weak("No notes match.");
                return;
            }
            for idx in visible {
                let note = &self.notes[idx];
                let selected = self.selected == Some(note.id);
                let label = note.title.as_str();
                if ui.selectable_label(selected, label).clicked() {
                    self.selected = Some(note.id);
                }
            }
        });
    }

    fn ui_editor(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected else {
            ui.centered_and_justified(|ui| ui.weak("Select or create a note."));
            return;
        };
        let Some(idx) = self.note_index(id) else {
            self.selected = None;
            return;
        };

        let mut delete = false;

        egui::Panel::top("editor_toolbar")
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        delete = true;
                    }
                    ui.separator();
                    ui.weak(format!("note #{id}"));
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let resp = ui.button("Link…");
                            if resp.clicked() {
                                self.link_popup_open = !self.link_popup_open;
                            }
                            egui::Popup::from_response(&resp)
                                .open_bool(&mut self.link_popup_open)
                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                .show(|ui| {
                                    let candidates: Vec<(u64, String)> = self
                                        .notes
                                        .iter()
                                        .filter(|n| n.id != id)
                                        .map(|n| (n.id, n.title.clone()))
                                        .collect();
                                    if candidates.is_empty() {
                                        ui.weak("No other notes to link.");
                                        return;
                                    }
                                    for (cid, title) in candidates {
                                        let linked = self.notes[idx].links.contains(&cid);
                                        if ui.selectable_label(linked, title).clicked() {
                                            let note = &mut self.notes[idx];
                                            if linked {
                                                note.links.retain(|&l| l != cid);
                                            } else {
                                                note.links.push(cid);
                                            }
                                            note.touch();
                                        }
                                    }
                                });
                        },
                    );
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("editor_scroll")
                .show(ui, |ui| {
                    let mut title_changed = false;
                    let mut body_changed = false;

                    let note = &mut self.notes[idx];
                    let title = egui::TextEdit::singleline(&mut note.title)
                        .hint_text("Title")
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Heading);
                    if ui.add(title).changed() {
                        title_changed = true;
                    }
                    ui.add_space(4.0);
                    let body = egui::TextEdit::multiline(&mut note.body)
                        .hint_text("Start writing…")
                        .desired_width(f32::INFINITY)
                        .desired_rows(20)
                        .code_editor();
                    if ui.add(body).changed() {
                        body_changed = true;
                    }

                    if title_changed || body_changed {
                        self.notes[idx].touch();
                        self.status = format!("Edited note #{id}");
                    }
                    if delete {
                        self.delete_note(id);
                    }
                });
        });
    }

    fn ui_chains(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected else {
            ui.weak("No note selected.");
            return;
        };
        let Some(idx) = self.note_index(id) else {
            return;
        };

        ui.heading("Chains");
        ui.weak(format!(
            "created {}, edited {}",
            fmt_time(self.notes[idx].created_at),
            fmt_time(self.notes[idx].updated_at)
        ));
        ui.separator();

        let linked: Vec<(u64, String)> = self.notes[idx]
            .links
            .iter()
            .filter_map(|&l| {
                self.note_index(l)
                    .map(|i| (l, self.notes[i].title.clone()))
            })
            .collect();

        if linked.is_empty() {
            ui.weak("No links yet. Use “Link…” in the editor.");
        } else {
            ui.label("Outgoing:");
            for (lid, title) in linked {
                if ui.selectable_label(false, title).clicked() {
                    self.selected = Some(lid);
                }
            }
        }

        ui.add_space(8.0);
        let backlinks: Vec<(u64, String)> = self
            .notes
            .iter()
            .filter(|n| n.links.contains(&id))
            .map(|n| (n.id, n.title.clone()))
            .collect();

        if !backlinks.is_empty() {
            ui.label("Backlinks:");
            for (bid, title) in backlinks {
                if ui.selectable_label(false, title).clicked() {
                    self.selected = Some(bid);
                }
            }
        }
    }
}

impl eframe::App for ChainNotesApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("menu").resizable(false).show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New note").clicked() {
                        self.add_note("Untitled");
                        ui.close();
                    }
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.status = "chainnotes — a chained note taker".to_owned();
                        ui.close();
                    }
                });
            });
        });

        egui::Panel::bottom("status")
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status);
                });
            });

        egui::Panel::left("sidebar")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| self.ui_sidebar(ui));

        egui::Panel::right("chains")
            .resizable(true)
            .default_size(200.0)
            .show(ui, |ui| self.ui_chains(ui));

        egui::CentralPanel::default().show(ui, |ui| self.ui_editor(ui));
    }
}
