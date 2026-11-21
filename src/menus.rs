//! Kontextmenü des Blattes wie im Original (PopupMenu1 der Formularressource TFORM1; Sichtbarkeit je
//! nach markiertem Objekt, verglichen mit Bildschirmfotos des Originals):
//! - Symbol: Löschen | Optionen ▸, Bildindex ▸, Alternativtext ▸, Eigentümer ▸, Kontaktspiegel hinzufügen,
//!   Bauteilliste ▸ | Symbol bearbeiten | Eigenschaften
//! - Text: Löschen | Ausrichtung ▸, Eigenschaften, Fixieren ▸
//! - Verbindung: Löschen | Sammelschiene ▸ | Eigenschaften
//! - Zeichenobjekt: Löschen | Eigenschaften, Fixieren ▸
//! - freie Fläche: Objekt einfügen ▸, Verweis einfügen ▸ | Eigenschaften, Pfadbezeichnung

use crate::app::App;
use crate::model::*;
use crate::symbols;
use eframe::egui;

impl App {
    fn change_symbol(&mut self, id: u64, f: impl FnOnce(&mut Kind)) {
        self.checkpoint();
        self.with_sheet(|s, _| {
            if let Some(o) = s.objects.iter_mut().find(|o| o.id == id) {
                f(&mut o.kind);
            }
        });
        self.dirty = true;
    }

    pub fn canvas_menu(&mut self, ui: &mut egui::Ui, has_sel: bool, is_sym: bool, is_text: bool) {
        let objs = self.selected_objs();
        let is_wire = objs.iter().any(|o| matches!(o.kind, Kind::Wire { .. }));
        let at = self.mouse_ctx;

        if !has_sel {
            // freie Blattfläche
            ui.menu_button("Objekt einfügen", |ui| {
                let defs: Vec<(String, String)> = self.project.symdefs.iter().map(|d| (d.key.clone(), d.name.clone())).collect();
                egui::ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
                    for (k, n) in defs {
                        if ui.button(n).clicked() {
                            self.place_symbol(k, at);
                            ui.close();
                        }
                    }
                });
            });
            ui.menu_button("Verweis einfügen", |ui| {
                if ui.button("Start").clicked() {
                    self.xdlg = Some(crate::extra::XDialog::Verweis { text: String::new(), start: true });
                    ui.close();
                }
                if ui.button("Ziel").clicked() {
                    self.xdlg = Some(crate::extra::XDialog::Verweis { text: String::new(), start: false });
                    ui.close();
                }
            });
            ui.separator();
            if ui.button("Eigenschaften").clicked() {
                if let Some(r) = self.cur() {
                    self.open_sheet_props(r);
                }
                ui.close();
            }
            if ui.button("Pfadbezeichnung").clicked() {
                self.open_columns_dialog();
                ui.close();
            }
            return;
        }

        if ui.add(egui::Button::new("Löschen").shortcut_text("Del")).clicked() {
            self.delete_selection();
            ui.close();
            return;
        }
        ui.separator();

        if is_sym {
            let Some(so) = objs.iter().find(|o| matches!(o.kind, Kind::Symbol { .. })).cloned() else { return };
            let Kind::Symbol { key, option, flags, ext, .. } = &so.kind else { return };
            let def = symbols::find(&self.lib, key).cloned();
            let id = so.id;
            // Optionen: Zusatzoptionen zum An-/Abhaken (OPT-Blöcke)
            let names = def.as_ref().map(|d| d.flag_names.clone()).unwrap_or_default();
            if !names.is_empty() {
                ui.menu_button("Optionen", |ui| {
                    for (i, n) in names.iter().enumerate() {
                        if n.is_empty() {
                            continue;
                        }
                        let mut on = flags & (1 << i) != 0;
                        if ui.checkbox(&mut on, n.as_str()).clicked() {
                            self.change_symbol(id, |k| {
                                if let Kind::Symbol { flags, .. } = k {
                                    *flags ^= 1 << i;
                                }
                            });
                            ui.close();
                        }
                    }
                });
            }
            // Bildindex: Bildvarianten (SEI)
            let bilder = def.as_ref().map(|d| d.options.clone()).unwrap_or_default();
            if bilder.len() > 1 {
                ui.menu_button("Bildindex", |ui| {
                    for (i, n) in bilder.iter().enumerate() {
                        if ui.radio(*option as usize == i, n.as_str()).clicked() {
                            self.change_symbol(id, |k| {
                                if let Kind::Symbol { option, ext, .. } = k {
                                    *option = i as u16;
                                    ext.bounds = None;
                                }
                            });
                            ui.close();
                        }
                    }
                });
            }
            // Alternativtext: Standard, die Sätze des Symbols, Benutzerdefiniert
            let sets = def.as_ref().map(|d| d.alt_sets.clone()).unwrap_or_default();
            ui.menu_button("Alternativtext", |ui| {
                let cur = ext.alt;
                let mut items = vec!["Standard".to_string()];
                items.extend(sets.iter().map(|s| symbols::alt_label(s)));
                for (i, n) in items.iter().enumerate() {
                    if ui.radio(cur as usize == i, n.as_str()).clicked() {
                        self.change_symbol(id, |k| {
                            if let Kind::Symbol { ext, .. } = k {
                                ext.alt = i as u16;
                            }
                        });
                        ui.close();
                    }
                }
                if ui.radio(cur as usize >= items.len(), "Benutzerdefiniert").clicked() {
                    self.edit_props(id);
                    ui.close();
                }
            });
            ui.menu_button("Eigentümer", |ui| {
                if ui.button("Eigenschaften").clicked() {
                    self.edit_props(id);
                    ui.close();
                }
                let any_sym = self.selected_objs().iter().any(|o| matches!(o.kind, Kind::Symbol { .. }));
                if ui.add_enabled(any_sym, egui::Button::new("Setzen")).clicked() {
                    self.start_owner_pick();
                    ui.close();
                }
                if ui.add_enabled(self.selection.len() == 1, egui::Button::new("Suchen")).clicked() {
                    self.find_same_bmk(id);
                    ui.close();
                }
            });
            if ui.button("Kontaktspiegel hinzufügen").clicked() {
                self.open_spiegel();
                ui.close();
            }
            if ui.button("Bezeichner setzen").clicked() {
                self.open_textchange();
                ui.close();
            }
            ui.menu_button("Bauteilliste", |ui| {
                if ui.button("Bearbeiten").clicked() {
                    let idx = self.project.part_of.get(&id).copied().unwrap_or(0);
                    self.xdlg = Some(crate::extra::XDialog::PartAssign { obj: id, idx });
                    ui.close();
                }
                if ui.button("Loeschen").clicked() {
                    self.checkpoint();
                    self.project.part_of.remove(&id);
                    ui.close();
                }
            });
            ui.separator();
            if ui.button("Symbol bearbeiten").clicked() {
                self.open_symbol_editor_fuer(id);
                ui.close();
            }
            ui.separator();
            if ui.button("Eigenschaften").clicked() {
                self.props_command(id);
                ui.close();
            }
            return;
        }

        if is_wire {
            ui.menu_button("Sammelschiene", |ui| {
                if ui.button("Nach Unten").clicked() {
                    self.busbar_down();
                    ui.close();
                }
            });
            ui.separator();
        }
        if is_text {
            ui.menu_button("Ausrichtung", |ui| {
                for (n, al) in [("Links", Align::Left), ("Mittig", Align::Center), ("Rechts", Align::Right)] {
                    if ui.button(n).clicked() {
                        self.set_text_align(Some(al), None);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Hochkant").clicked() {
                    self.set_text_align(None, Some(true));
                    ui.close();
                }
                if ui.button("Waagerecht").clicked() {
                    self.set_text_align(None, Some(false));
                    ui.close();
                }
            });
        }
        if ui.button("Eigenschaften").clicked() {
            if let Some(id) = self.selection.first().copied() {
                self.props_command(id);
            }
            ui.close();
        }
        // Verweis: Ziel (Anlage, Blatt, Verweislink) setzen
        let verweis = objs.iter().find(|o| matches!(o.kind, Kind::Verweis { .. })).map(|o| o.id);
        if let Some(id) = verweis {
            if ui.button("Ziel setzen").clicked() {
                self.open_props3(id);
                ui.close();
            }
        }
        if !is_wire {
            ui.menu_button("Fixieren", |ui| {
                if ui.button("Ja").clicked() {
                    self.set_fixed(true);
                    ui.close();
                }
                if ui.button("Nein").clicked() {
                    self.set_fixed(false);
                    ui.close();
                }
            });
        }
    }
}

impl App {
    /// Werkzeugfenster "Ausrichtung" (Formular TAusrichtung: Referenzpunkt, Horizontal, Vertikal,
    /// Abstand Horizontal, Abstand Vertikal; bleibt im Vordergrund wie fsStayOnTop).
    pub fn align_window(&mut self, ctx: &egui::Context) {
        if !self.align_win {
            return;
        }
        let objs = self.selected_objs();
        let names: Vec<String> = objs
            .iter()
            .enumerate()
            .map(|(i, o)| match &o.kind {
                Kind::Symbol { bmk, .. } if !bmk.is_empty() => format!("{} {}", i + 1, bmk),
                Kind::Text { text, .. } => format!("{} {}", i + 1, text.lines().next().unwrap_or("")),
                k => format!("{} {}", i + 1, crate::epr_write::kind_name(k)),
            })
            .collect();
        let mut open = true;
        let mut action: Option<u8> = None;
        egui::Window::new("Ausrichtung")
            .id(egui::Id::unique("ausrichtung"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size([116.0, 156.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label("Referenzpunkt");
                let cur = names.get(self.align_ref).cloned().unwrap_or_default();
                egui::ComboBox::from_id_salt("refp").width(112.0).selected_text(cur).show_ui(ui, |ui| {
                    for (i, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut self.align_ref, i, n);
                    }
                });
                ui.add_space(6.0);
                let enabled = objs.len() >= 2;
                // vier Knöpfe wie im Original (TAusrichtung), mit eigenen Vektor-Icons
                use crate::icons::Icon;
                let glyphs: [(u8, &str, Icon); 4] = [
                    (0, "Horizontal", Icon::AlignHor),
                    (2, "Abstand Horizontal", Icon::SpaceHor),
                    (1, "Vertikal", Icon::AlignVer),
                    (3, "Abstand Vertikal", Icon::SpaceVer),
                ];
                for row in glyphs.chunks(2) {
                    ui.horizontal(|ui| {
                        for (code, hint, icon) in row {
                            let btn = egui::Button::image(crate::icons::image(*icon, 29.0));
                            if ui.add_enabled(enabled, btn.min_size(egui::Vec2::splat(37.0))).on_hover_text(*hint).clicked() {
                                action = Some(*code);
                            }
                            ui.add_space(12.0);
                        }
                    });
                }
            });
        if !open {
            self.align_win = false;
        }
        if let Some(a) = action {
            self.align_to_reference(a);
        }
    }

    /// Markierte Objekte am Referenzobjekt ausrichten bzw. gleichmäßig verteilen (Umrissrechtecke wie im Original).
    pub fn align_to_reference(&mut self, mode: u8) {
        let objs = self.selected_objs();
        if objs.len() < 2 {
            return;
        }
        let lib = self.lib.clone();
        let bounds: Vec<(u64, P, P)> = objs.iter().map(|o| {
            let (a, b) = crate::canvas::obj_bounds(o, &lib);
            (o.id, a, b)
        }).collect();
        let r = self.align_ref.min(bounds.len() - 1);
        let (_, rmn, rmx) = bounds[r];
        let mut moves: Vec<(u64, P)> = vec![];
        match mode {
            0 => {
                let cy = (rmn.y + rmx.y) / 2.0;
                for (id, a, b) in &bounds {
                    moves.push((*id, P::new(0.0, cy - (a.y + b.y) / 2.0)));
                }
            }
            1 => {
                let cx = (rmn.x + rmx.x) / 2.0;
                for (id, a, b) in &bounds {
                    moves.push((*id, P::new(cx - (a.x + b.x) / 2.0, 0.0)));
                }
            }
            _ => {
                let horiz = mode == 2;
                let mut v = bounds.clone();
                v.sort_by(|x, y| {
                    let (kx, ky) = if horiz { (x.1.x, y.1.x) } else { (x.1.y, y.1.y) };
                    kx.partial_cmp(&ky).unwrap_or(std::cmp::Ordering::Equal)
                });
                let first = if horiz { v[0].1.x } else { v[0].1.y };
                let last = if horiz { v[v.len() - 1].1.x } else { v[v.len() - 1].1.y };
                let step = (last - first) / (v.len() - 1) as f32;
                for (i, (id, a, _)) in v.iter().enumerate() {
                    let target = first + step * i as f32;
                    let d = target - if horiz { a.x } else { a.y };
                    moves.push((*id, if horiz { P::new(d, 0.0) } else { P::new(0.0, d) }));
                }
            }
        }
        self.checkpoint();
        self.with_sheet(|s, _| {
            for (id, d) in moves {
                if let Some(o) = s.objects.iter_mut().find(|o| o.id == id && !o.fixed) {
                    o.kind.translate(d);
                }
            }
        });
        self.dirty = true;
    }
}
